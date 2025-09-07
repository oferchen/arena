use std::sync::Arc;

use axum::{Router, extract::{State, Json}, http::StatusCode, routing::post};
use serde::{Deserialize, Serialize};
use ::payments::UserId;
use crate::{AppState};
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
    let session = state.stripe.create_checkout_session(sku).await;
    state.analytics.dispatch(Event::PurchaseInitiated);
    Ok(Json(CheckoutResponse { checkout_url: session.url }))
}

async fn webhook_handler(
    State(state): State<Arc<AppState>>,
    Json(evt): Json<WebhookEvent>,
) -> StatusCode {
    state.entitlements.grant(evt.user_id, evt.sku_id);
    state.analytics.dispatch(Event::PurchaseCompleted);
    StatusCode::OK
}
