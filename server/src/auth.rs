use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use axum::{
    Router,
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode, header::SET_COOKIE},
    response::IntoResponse,
    routing::post,
    Json,
};
use once_cell::sync::Lazy;
use rand::{distributions::Uniform, Rng};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::AppState;

static OTP_STORE: Lazy<Mutex<HashMap<String, (String, Instant)>>> = Lazy::new(|| Mutex::new(HashMap::new()));
const REQUEST_COOLDOWN: Duration = Duration::from_secs(60);
const SALT: &str = "arena_salt";

fn hash_email(email: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(SALT.as_bytes());
    hasher.update(email.as_bytes());
    hex::encode(hasher.finalize())
}

#[derive(Deserialize)]
struct RequestBody {
    email: String,
}

#[derive(Deserialize)]
struct VerifyBody {
    email: String,
    code: String,
}

#[derive(Serialize)]
struct VerifyResponse {
    token: String,
}

async fn request_handler(State(state): State<std::sync::Arc<AppState>>, Json(body): Json<RequestBody>) -> StatusCode {
    let email_hash = hash_email(&body.email);
    let mut store = OTP_STORE.lock().unwrap();
    if let Some((_, ts)) = store.get(&email_hash) {
        if ts.elapsed() < REQUEST_COOLDOWN {
            return StatusCode::TOO_MANY_REQUESTS;
        }
    }
    let mut rng = rand::thread_rng();
    let code = rng.sample(Uniform::new(0, 1_000_000));
    let code_str = format!("{:06}", code);
    store.insert(email_hash, (code_str.clone(), Instant::now()));
    let _ = state.email.send_otp_code(&body.email, &code_str);
    StatusCode::OK
}

async fn verify_handler(State(_state): State<std::sync::Arc<AppState>>, Json(body): Json<VerifyBody>) -> impl IntoResponse {
    let email_hash = hash_email(&body.email);
    let mut store = OTP_STORE.lock().unwrap();
    let token = if let Some((code, _)) = store.get(&email_hash) {
        if code == &body.code {
            store.remove(&email_hash);
            Uuid::new_v4().to_string()
        } else {
            String::new()
        }
    } else {
        String::new()
    };
    let mut headers = HeaderMap::new();
    headers.insert(
        SET_COOKIE,
        HeaderValue::from_str(&format!("session={}; Path=/; HttpOnly", token)).unwrap(),
    );
    (headers, Json(VerifyResponse { token }))
}

pub fn routes() -> Router<std::sync::Arc<AppState>> {
    Router::new()
        .route("/request", post(request_handler))
        .route("/verify", post(verify_handler))
}
