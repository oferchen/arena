use std::{net::SocketAddr, sync::Arc};

use anyhow::{Result, anyhow};

use crate::email::{EmailService, SmtpConfig};
use axum::{
    Router,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderName, HeaderValue, StatusCode, header::CACHE_CONTROL},
    response::IntoResponse,
    routing::{get, get_service, post},
};
use clap::Parser;
use net::server::ServerConnector;
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
    fn smtp_config(&self) -> Result<SmtpConfig> {
        let starttls = self.smtp_starttls.parse().map_err(|_| {
            anyhow!(
                "invalid value for --smtp-starttls: {}",
                self.smtp_starttls
            )
        })?;
        Ok(SmtpConfig {
            host: self.smtp_host.clone(),
            port: self.smtp_port,
            from: self.smtp_from.clone(),
            starttls,
            smtps: self.smtp_smtps,
            timeout: self.smtp_timeout_ms,
            user: self.smtp_user.clone(),
            pass: self.smtp_pass.clone(),
        })
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

async fn signal_ws_handler(
    State(state): State<Arc<AppState>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| async move {
        handle_signal_socket(state, socket).await;
    })
}

async fn handle_signal_socket(state: Arc<AppState>, mut socket: WebSocket) {
    if let Some(Ok(Message::Text(sdp))) = socket.recv().await {
        if let Ok(connector) = ServerConnector::new().await {
            let mut offer = RTCSessionDescription::default();
            offer.sdp_type = RTCSdpType::Offer;
            offer.sdp = sdp;
            if connector.pc.set_remote_description(offer).await.is_ok() {
                if let Ok(answer) = connector.pc.create_answer(None).await {
                    let _ = connector.pc.set_local_description(answer.clone()).await;
                    let _ = socket.send(Message::Text(answer.sdp.clone())).await;
                    state.rooms.add_peer(connector).await;
                }
            }
        }
    }
    let _ = socket.close().await;
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

    let email = Arc::new(EmailService::new(smtp).map_err(|e| {
        log::error!("failed to initialize email service: {e:?}");
        anyhow!("{e:?}")
    })?);

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
        .route("/signal", get(signal_ws_handler))
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
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("content-security-policy"),
            HeaderValue::from_static("default-src 'self'"),
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
    let smtp = match cli.smtp_config() {
        Ok(cfg) => cfg,
        Err(e) => {
            log::error!("{e}");
            std::process::exit(1);
        }
    };
    if let Err(e) = run(smtp).await {
        log::error!("{e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::{SinkExt, StreamExt};
    use sqlx::postgres::PgPoolOptions;
    use std::env;
    use tokio_tungstenite::tungstenite::Message;
    use webrtc::api::APIBuilder;
    use webrtc::api::media_engine::MediaEngine;
    use webrtc::peer_connection::configuration::RTCConfiguration;

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

    #[test]
    fn invalid_starttls_cli_value_errors() {
        let cli =
            Cli::try_parse_from(["prog", "--smtp-starttls", "bogus"]).unwrap();
        assert!(cli.smtp_config().is_err());
    }

    #[test]
    fn invalid_starttls_env_value_errors() {
        unsafe {
            env::set_var("ARENA_SMTP_STARTTLS", "bogus");
        }
        let cli = Cli::try_parse_from(["prog"]).unwrap();
        assert!(cli.smtp_config().is_err());
        unsafe {
            env::remove_var("ARENA_SMTP_STARTTLS");
        }
    }

    #[tokio::test]
    async fn websocket_signaling_completes_handshake() {
        let db = PgPoolOptions::new()
            .connect_lazy("postgres://localhost")
            .unwrap();
        let email = Arc::new(EmailService::new(SmtpConfig::default()).unwrap());
        let rooms = room::RoomManager::new();
        let state = Arc::new(AppState { db, email, rooms });

        let app = Router::new()
            .route("/signal", get(signal_ws_handler))
            .with_state(state);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let mut m = MediaEngine::default();
        m.register_default_codecs().unwrap();
        let api = APIBuilder::new().with_media_engine(m).build();
        let pc = api
            .new_peer_connection(RTCConfiguration::default())
            .await
            .unwrap();
        let _dc = pc.create_data_channel("data", None).await.unwrap();
        let offer = pc.create_offer(None).await.unwrap();
        pc.set_local_description(offer.clone()).await.unwrap();

        let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{}/signal", addr))
            .await
            .unwrap();
        ws.send(Message::Text(offer.sdp)).await.unwrap();
        let msg = ws.next().await.expect("no answer").unwrap();
        let answer_sdp = msg.into_text().unwrap();
        let mut answer = RTCSessionDescription::default();
        answer.sdp_type = RTCSdpType::Answer;
        answer.sdp = answer_sdp;
        pc.set_remote_description(answer).await.unwrap();
        assert!(pc.remote_description().await.is_some());
    }
}
