use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use rand::Rng;
use sqlx::PgPool;
use uuid::Uuid;
use chrono::{Utc, Duration};
use data_encoding::BASE32_NOPAD;
use serde::Deserialize;

pub fn router() -> Router<PgPool> {
    Router::<PgPool>::new()
        .route("/email", post(send_email_otp))
        .route("/email/verify", post(verify_email_otp))
        .route("/totp/enable", post(enable_totp))
}

#[derive(Deserialize)]
pub struct EmailOtpRequest {
    pub email: String,
}

pub async fn send_email_otp(
    State(pool): State<PgPool>,
    Json(payload): Json<EmailOtpRequest>,
) -> Result<StatusCode, StatusCode> {
    let otp: String = {
        let mut rng = rand::thread_rng();
        (0..6).map(|_| rng.gen_range(0..10).to_string()).collect()
    };
    let expires_at = Utc::now() + Duration::minutes(10);

    sqlx::query("INSERT INTO email_otps (id, email, otp, expires_at) VALUES ($1, $2, $3, $4)")
        .bind(Uuid::new_v4())
        .bind(&payload.email)
        .bind(&otp)
        .bind(expires_at)
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // In a real app, send the OTP via email here.
    Ok(StatusCode::OK)
}

#[derive(Deserialize)]
pub struct VerifyEmailOtpRequest {
    pub email: String,
    pub otp: String,
}

pub async fn verify_email_otp(
    State(pool): State<PgPool>,
    Json(payload): Json<VerifyEmailOtpRequest>,
) -> Result<StatusCode, StatusCode> {
    let row: Option<(Uuid, chrono::DateTime<Utc>)> = sqlx::query_as(
        "SELECT id, expires_at FROM email_otps WHERE email = $1 AND otp = $2",
    )
    .bind(&payload.email)
    .bind(&payload.otp)
    .fetch_optional(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some((id, expires_at)) = row {
        if expires_at > Utc::now() {
            sqlx::query("DELETE FROM email_otps WHERE id = $1")
                .bind(id)
                .execute(&pool)
                .await
                .ok();
            return Ok(StatusCode::OK);
        }
    }

    Err(StatusCode::UNAUTHORIZED)
}

#[derive(Deserialize)]
pub struct EnableTotpRequest {
    pub user_id: Uuid,
}

pub async fn enable_totp(
    State(pool): State<PgPool>,
    Json(payload): Json<EnableTotpRequest>,
) -> Result<Json<String>, StatusCode> {
    let secret: [u8; 32] = rand::random();
    let encoded = BASE32_NOPAD.encode(&secret);

    sqlx::query(
        "INSERT INTO user_totp (user_id, secret) VALUES ($1, $2) \
         ON CONFLICT (user_id) DO UPDATE SET secret = EXCLUDED.secret",
    )
    .bind(payload.user_id)
    .bind(&encoded)
    .execute(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(encoded))
}
