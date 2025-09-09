use std::{sync::Arc, time::Duration};

use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode, header::SET_COOKIE},
    response::IntoResponse,
    routing::post,
};
use chrono::{Duration as ChronoDuration, Utc};
use rand::{Rng, distributions::Uniform};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{AppState, otp_store};

const REQUEST_COOLDOWN: Duration = Duration::from_secs(60);
const OTP_TTL: Duration = Duration::from_secs(300);
fn hash_email(email: &str, salt: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(salt.as_bytes());
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

async fn request_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<RequestBody>,
) -> impl IntoResponse {
    let email_hash = hash_email(&body.email, &state.email_salt);
    match otp_store::fetch_otp(&state.db, &email_hash).await {
        Ok(Some((_, expires_at))) => {
            let now = Utc::now();
            if now < expires_at {
                let created_at = expires_at - ChronoDuration::from_std(OTP_TTL).unwrap();
                if now < created_at + ChronoDuration::from_std(REQUEST_COOLDOWN).unwrap() {
                    return StatusCode::TOO_MANY_REQUESTS;
                }
            }
            if let Err(e) = otp_store::delete_otp(&state.db, &email_hash).await {
                tracing::error!("failed to delete OTP: {e}");
                return StatusCode::INTERNAL_SERVER_ERROR;
            }
        }
        Ok(None) => {}
        Err(e) => {
            tracing::error!("failed to fetch OTP: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    }
    let code = rand::thread_rng().sample(Uniform::new(0, 1_000_000));
    let code_str = format!("{:06}", code);
    let expires_at = Utc::now() + ChronoDuration::from_std(OTP_TTL).unwrap();
    if let Err(e) = otp_store::insert_otp(&state.db, &email_hash, &code_str, expires_at).await {
        tracing::error!("failed to insert OTP: {e}");
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    let _ = state.email.send_otp_code(&body.email, &code_str);
    StatusCode::OK
}

async fn verify_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<VerifyBody>,
) -> impl IntoResponse {
    let email_hash = hash_email(&body.email, &state.email_salt);
    match otp_store::fetch_otp(&state.db, &email_hash).await {
        Ok(Some((code, expires_at))) => {
            if code == body.code && Utc::now() <= expires_at {
                if let Err(e) = otp_store::delete_otp(&state.db, &email_hash).await {
                    tracing::error!("failed to delete OTP: {e}");
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(VerifyResponse {
                            token: String::new(),
                        }),
                    )
                        .into_response();
                }
                let token = Uuid::new_v4().to_string();
                let mut headers = HeaderMap::new();
                let cookie = format!("session={}; Path=/; Secure; HttpOnly; SameSite=Lax", token);
                headers.insert(SET_COOKIE, HeaderValue::from_str(&cookie).unwrap());
                return (headers, Json(VerifyResponse { token })).into_response();
            }
        }
        Ok(None) => {}
        Err(e) => {
            tracing::error!("failed to fetch OTP: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(VerifyResponse {
                    token: String::new(),
                }),
            )
                .into_response();
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

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/request", post(request_handler))
        .route("/verify", post(verify_handler))
}
