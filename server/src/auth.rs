use std::time::Duration;

use axum::{
    Router,
    extract::{FromRef, State},
    http::{HeaderMap, HeaderValue, StatusCode, header::SET_COOKIE},
    response::IntoResponse,
    routing::post,
    Json,
};
use chrono::{Duration as ChronoDuration, Utc};
use rand::{distributions::Uniform, Rng};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{otp_store, AppState};

const REQUEST_COOLDOWN: Duration = Duration::from_secs(60);
const OTP_TTL: Duration = Duration::from_secs(300);
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
    let db = match &state.db {
        Some(db) => db,
        None => return StatusCode::INTERNAL_SERVER_ERROR,
    };
    if let Some((_, expires_at)) = otp_store::fetch_otp(db, &email_hash).await {
        let now = Utc::now();
        if now < expires_at {
            let created_at =
                expires_at - ChronoDuration::from_std(OTP_TTL).unwrap();
            if now < created_at + ChronoDuration::from_std(REQUEST_COOLDOWN).unwrap() {
                return StatusCode::TOO_MANY_REQUESTS;
            }
        }
        let _ = otp_store::delete_otp(db, &email_hash).await;
    }
    let mut rng = rand::thread_rng();
    let code = rng.sample(Uniform::new(0, 1_000_000));
    let code_str = format!("{:06}", code);
    let expires_at = Utc::now() + ChronoDuration::from_std(OTP_TTL).unwrap();
    let _ = otp_store::insert_otp(db, &email_hash, &code_str, expires_at).await;
    let _ = state.email.send_otp_code(&body.email, &code_str);
    StatusCode::OK
}

async fn verify_handler(
    State(state): State<std::sync::Arc<AppState>>,
    Json(body): Json<VerifyBody>,
) -> impl IntoResponse {
    let email_hash = hash_email(&body.email);
    let db = match &state.db {
        Some(db) => db,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(VerifyResponse {
                    token: String::new(),
                }),
            )
                .into_response()
        }
    };
    if let Some((code, expires_at)) = otp_store::fetch_otp(db, &email_hash).await {
        if code == body.code && Utc::now() <= expires_at {
            let _ = otp_store::delete_otp(db, &email_hash).await;
            let token = Uuid::new_v4().to_string();
            let mut headers = HeaderMap::new();
            let cookie =
                format!("session={}; Path=/; Secure; HttpOnly; SameSite=Lax", token);
            headers.insert(SET_COOKIE, HeaderValue::from_str(&cookie).unwrap());
            return (headers, Json(VerifyResponse { token })).into_response();
        }
    }
    (
        StatusCode::UNAUTHORIZED,
        Json(VerifyResponse {
            token: String::new(),
        }),
    )
        .into_response()
}

pub fn routes<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    std::sync::Arc<AppState>: FromRef<S>,
{
    Router::new()
        .route("/request", post(request_handler))
        .route("/verify", post(verify_handler))
}
