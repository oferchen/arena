use std::{net::SocketAddr, sync::Arc};

use anyhow::Result;

use axum::{
    extract::{
        State,
        ws::{WebSocket, WebSocketUpgrade},
    },
    http::{header::CACHE_CONTROL, HeaderName, HeaderValue, StatusCode},
    response::IntoResponse,
    routing::{get, get_service, post},
    Json, Router,
};
use crate::email::EmailService;
use net::server::ServerConnector;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::sdp::sdp_type::RTCSdpType;
use serde::{Deserialize, Serialize};

mod room;
mod email;
use sqlx::PgPool;
use tower_http::{services::ServeDir, set_header::SetResponseHeaderLayer};

#[derive(Clone)]
struct AppState {
    db: PgPool,
    email: Arc<EmailService>,
    rooms: room::RoomManager,
}

fn auth_routes() -> Router<Arc<AppState>> {
    Router::new().route("/*path", get(|| async { StatusCode::OK }))
}

async fn ws_handler(
    State(_state): State<Arc<AppState>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| async move {
        handle_socket(socket).await;
    })
}

#[derive(Deserialize)]
struct SignalRequest {
    sdp: String,
}

#[derive(Serialize)]
struct SignalResponse {
    sdp: String,
}

async fn signal_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SignalRequest>,
) -> Result<Json<SignalResponse>, StatusCode> {
    let connector = ServerConnector::new().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut offer = RTCSessionDescription::default();
    offer.sdp_type = RTCSdpType::Offer;
    offer.sdp = req.sdp;
    connector
        .pc
        .set_remote_description(offer)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let answer = connector
        .pc
        .create_answer(None)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    connector
        .pc
        .set_local_description(answer.clone())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    state.rooms.add_peer(connector).await;
    Ok(Json(SignalResponse { sdp: answer.sdp }))
}

async fn handle_socket(mut socket: WebSocket) {
    while let Some(Ok(_)) = socket.recv().await {}
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

fn get_env(name: &str) -> Result<String> {
    std::env::var(name).map_err(|e| {
        log::error!("{name} environment variable not set: {e}");
        e.into()
    })
}

async fn setup() -> Result<AppState> {
    let database_url = get_env("DATABASE_URL")?;
    let db = PgPool::connect(&database_url).await.map_err(|e| {
        log::error!("failed to connect to database: {e}");
        e
    })?;

    let email = Arc::new(EmailService::new(
        &get_env("SMTP_SERVER")?,
        &get_env("SMTP_USERNAME")?,
        &get_env("SMTP_PASSWORD")?,
        &get_env("EMAIL_FROM")?,
    ));

    let rooms = room::RoomManager::new();
    Ok(AppState { db, email, rooms })
}

async fn run() -> Result<()> {
    let state = Arc::new(setup().await?);


    let assets_service =
        get_service(ServeDir::new("assets")).layer(SetResponseHeaderLayer::if_not_present(
            CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=31536000, immutable"),
        ));

    let app = Router::new()
        .nest("/auth", auth_routes())
        .route("/ws", get(ws_handler))
        .route("/signal", post(signal_handler))
        .nest_service("/assets", assets_service)
        .fallback_service(ServeDir::new("static"))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("cross-origin-opener-policy"),
            HeaderValue::from_static("same-origin"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("cross-origin-embedder-policy"),
            HeaderValue::from_static("require-corp"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("cross-origin-resource-policy"),
            HeaderValue::from_static("same-origin"),
        ))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| {
        log::error!("failed to bind to address: {e}");
        e
    })?;

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| {
            log::error!("server error: {e}");
            e
        })?;

    Ok(())
}

#[tokio::main]
async fn main() {
    env_logger::init();
    if let Err(e) = run().await {
        log::error!("{e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[tokio::test]
    async fn setup_fails_without_env_vars() {
        unsafe {
            env::remove_var("DATABASE_URL");
            env::remove_var("SMTP_SERVER");
            env::remove_var("SMTP_USERNAME");
            env::remove_var("SMTP_PASSWORD");
            env::remove_var("EMAIL_FROM");
        }

        assert!(setup().await.is_err());
    }
}
