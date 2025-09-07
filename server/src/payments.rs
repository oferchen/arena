use std::sync::Arc;

use axum::{
    Router,
    extract::{State, Json},
    http::{HeaderMap, StatusCode},
    routing::post,
    body::Bytes,
};
use serde::{Deserialize, Serialize};
use ::payments::{UserId, StoreProvider};
use crate::AppState;
use analytics::Event;

#[derive(Deserialize)]
pub struct PurchaseRequest {
    pub user_id: UserId,
    pub sku_id: String,
}

#[derive(Serialize)]
pub struct CheckoutResponse {
    pub checkout_url: String,
}

#[derive(Deserialize)]
pub struct WebhookEvent {
    pub user_id: UserId,
    pub sku_id: String,
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/checkout", post(checkout_handler))
        .route("/webhook", post(webhook_handler))
}

async fn checkout_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PurchaseRequest>,
) -> Result<Json<CheckoutResponse>, StatusCode> {
    let sku = state.catalog.get(&req.sku_id).ok_or(StatusCode::NOT_FOUND)?;
    let session = state.store.create_checkout_session(sku).await;
    state.analytics.dispatch(Event::PurchaseInitiated);
    Ok(Json(CheckoutResponse { checkout_url: session.url }))
}

async fn webhook_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> StatusCode {
    let sig = match headers.get("Stripe-Signature").and_then(|v| v.to_str().ok()) {
        Some(s) => s,
        None => return StatusCode::UNAUTHORIZED,
    };

    if !state.store.verify_webhook(sig, &body) {
        return StatusCode::UNAUTHORIZED;
    }

    let evt: WebhookEvent = match serde_json::from_slice(&body) {
        Ok(e) => e,
        Err(_) => return StatusCode::BAD_REQUEST,
    };

    state
        .entitlements
        .grant(evt.user_id, evt.sku_id.clone());
    state.analytics.dispatch(Event::PurchaseCompleted {
        sku: evt.sku_id,
        user: evt.user_id.to_string(),
    });
    StatusCode::OK
}
