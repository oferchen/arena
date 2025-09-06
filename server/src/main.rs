use std::{net::SocketAddr, sync::Arc};

use anyhow::Result;

use crate::email::{EmailService, SmtpConfig};
use axum::{
    Json, Router,
    extract::{
        State,
        ws::{WebSocket, WebSocketUpgrade},
    },
    http::{HeaderName, HeaderValue, StatusCode, header::CACHE_CONTROL},
    response::IntoResponse,
    routing::{get, get_service, post},
};
use clap::Parser;
use net::server::ServerConnector;
use serde::{Deserialize, Serialize};
use webrtc::peer_connection::sdp::sdp_type::RTCSdpType;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

mod email;
mod room;
use sqlx::PgPool;
use tower_http::{services::ServeDir, set_header::SetResponseHeaderLayer};

#[derive(Parser, Debug)]
struct Cli {
    #[arg(long, env = "ARENA_SMTP_HOST", default_value = "localhost")]
    smtp_host: String,
    #[arg(long, env = "ARENA_SMTP_PORT", default_value_t = 25)]
    smtp_port: u16,
    #[arg(long, env = "ARENA_SMTP_FROM", default_value = "arena@localhost")]
    smtp_from: String,
    #[arg(long, env = "ARENA_SMTP_STARTTLS", default_value = "auto")]
    smtp_starttls: String,
    #[arg(long, env = "ARENA_SMTP_SMTPS", default_value_t = false)]
    smtp_smtps: bool,
    #[arg(long, env = "ARENA_SMTP_USER")]
    smtp_user: Option<String>,
    #[arg(long, env = "ARENA_SMTP_PASS")]
    smtp_pass: Option<String>,
    #[arg(long, env = "ARENA_SMTP_TIMEOUT_MS", default_value_t = 10000)]
    smtp_timeout_ms: u64,
}

impl Cli {
    fn smtp_config(&self) -> SmtpConfig {
        SmtpConfig {
            host: self.smtp_host.clone(),
            port: self.smtp_port,
            from: self.smtp_from.clone(),
            starttls: self.smtp_starttls.parse().unwrap_or_default(),
            smtps: self.smtp_smtps,
            timeout: self.smtp_timeout_ms,
            user: self.smtp_user.clone(),
            pass: self.smtp_pass.clone(),
        }
    }
}

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
    let connector = ServerConnector::new()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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

async fn mail_test_handler(State(state): State<Arc<AppState>>) -> StatusCode {
    match state.email.send_test(state.email.from_address()) {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
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

async fn setup(smtp: SmtpConfig) -> Result<AppState> {
    let database_url = get_env("DATABASE_URL")?;
    let db = PgPool::connect(&database_url).await.map_err(|e| {
        log::error!("failed to connect to database: {e}");
        e
    })?;

    let email = Arc::new(EmailService::new(smtp));

    let rooms = room::RoomManager::new();
    Ok(AppState { db, email, rooms })
}

async fn run(smtp: SmtpConfig) -> Result<()> {
    let state = Arc::new(setup(smtp).await?);

    let assets_service =
        get_service(ServeDir::new("assets")).layer(SetResponseHeaderLayer::if_not_present(
            CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=31536000, immutable"),
        ));

    let app = Router::new()
        .nest("/auth", auth_routes())
        .route("/ws", get(ws_handler))
        .route("/signal", post(signal_handler))
        .route("/admin/mail/test", post(mail_test_handler))
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
    let cli = Cli::parse();
    if let Err(e) = run(cli.smtp_config()).await {
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
        }

        assert!(setup(SmtpConfig::default()).await.is_err());
    }

    #[test]
    fn cli_overrides_env() {
        unsafe {
            env::set_var("ARENA_SMTP_HOST", "envhost");
        }
        let cli = Cli::try_parse_from(["prog", "--smtp-host", "clihost"]).unwrap();
        assert_eq!(cli.smtp_host, "clihost");
        unsafe {
            env::remove_var("ARENA_SMTP_HOST");
        }
    }

    #[test]
    fn env_used_when_no_cli() {
        unsafe {
            env::set_var("ARENA_SMTP_PORT", "2525");
        }
        let cli = Cli::try_parse_from(["prog"]).unwrap();
        assert_eq!(cli.smtp_port, 2525);
        unsafe {
            env::remove_var("ARENA_SMTP_PORT");
        }
    }
}
