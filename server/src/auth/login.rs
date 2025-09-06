use axum::{extract::State, http::StatusCode, Json};
use serde::Deserialize;
use sqlx::PgPool;
use argon2::{Argon2, PasswordVerifier};
use argon2::password_hash::PasswordHash;

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

pub async fn login(
    State(pool): State<PgPool>,
    Json(payload): Json<LoginRequest>,
) -> Result<StatusCode, StatusCode> {
    let row: Option<(String,)> = sqlx::query_as("SELECT password_hash FROM users WHERE email = $1")
        .bind(&payload.email)
        .fetch_optional(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some((password_hash,)) = row {
        let parsed_hash = PasswordHash::new(&password_hash)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if Argon2::default()
            .verify_password(payload.password.as_bytes(), &parsed_hash)
            .is_ok()
        {
            return Ok(StatusCode::OK);
        }
    }

    Err(StatusCode::UNAUTHORIZED)
}
