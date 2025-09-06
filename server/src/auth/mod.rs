use axum::{Router, routing::post};
use sqlx::PgPool;

pub mod register;
pub mod login;
pub mod twofa;

pub fn router() -> Router<PgPool> {
    Router::<PgPool>::new()
        .route("/register", post(register::register))
        .route("/login", post(login::login))
        .nest("/2fa", twofa::router())
}
