use std::{net::SocketAddr, path::PathBuf, sync::Arc};

use anyhow::{Result, anyhow};

use crate::email::{EmailService, SmtpConfig, StartTls};
use ::payments::{Catalog, EntitlementList, Sku, UserId};
use analytics::{Analytics, Event};
use axum::{
    Router,
    extract::{
        Json, Path, Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{
        HeaderMap, HeaderName, HeaderValue, StatusCode,
        header::{CACHE_CONTROL, SET_COOKIE},
    },
    response::IntoResponse,
    routing::{get, get_service, post},
};
use clap::Parser;
use email_address::EmailAddress;
use net::server::ServerConnector;
use payments::EntitlementStore;
use scylla::{Session, SessionBuilder};
use serde::{Deserialize, Serialize};
use webrtc::peer_connection::sdp::sdp_type::RTCSdpType;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

mod auth;
mod email;
mod leaderboard;
mod config;
mod payments;
mod room;
mod shard;
#[cfg(test)]
mod test_logger;
use prometheus::{Encoder, TextEncoder};
use tower_http::{services::ServeDir, set_header::SetResponseHeaderLayer};

#[derive(Parser, Debug)]
struct Cli {
    #[command(flatten)]
    smtp: SmtpConfig,
    #[command(flatten)]
    config: Config,
    #[arg(long, env = "POSTHOG_KEY")]
    posthog_key: Option<String>,
    #[arg(long, env = "ENABLE_OTEL", default_value_t = false)]
    enable_otel: bool,
    #[arg(long, env = "ARENA_ANALYTICS_OPT_OUT", default_value_t = false)]
    analytics_opt_out: bool,
}

#[derive(Parser, Debug, Clone)]
struct Config {
    #[arg(long, env = "ARENA_BIND_ADDR")]
    bind_addr: Option<SocketAddr>,
    #[arg(long, env = "ARENA_PUBLIC_URL")]
    public_url: Option<String>,
    #[arg(long, env = "ARENA_SHARD_HOST")]
    shard_host: Option<String>,
    #[arg(long, env = "SCYLLA_URI")]
    database_url: Option<String>,
    #[arg(long, env = "ARENA_CSP")]
    csp: Option<String>,
}

#[derive(Debug, Clone)]
struct ResolvedConfig {
    bind_addr: SocketAddr,
    public_url: String,
    shard_host: String,
    database_url: String,
    csp: Option<String>,
}

impl Config {
    fn resolve(self) -> Result<ResolvedConfig> {
        Ok(ResolvedConfig {
            bind_addr: self
                .bind_addr
                .ok_or_else(|| anyhow!("ARENA_BIND_ADDR not set"))?,
            public_url: self
                .public_url
                .ok_or_else(|| anyhow!("ARENA_PUBLIC_URL not set"))?,
            shard_host: self
                .shard_host
                .ok_or_else(|| anyhow!("ARENA_SHARD_HOST not set"))?,
            database_url: self
                .database_url
                .ok_or_else(|| anyhow!("SCYLLA_URI not set"))?,
            csp: self.csp,
        })
    }
}

#[derive(Clone)]
pub(crate) struct AppState {
    email: Arc<EmailService>,
    rooms: room::RoomManager,
    smtp: SmtpConfig,
    analytics: Analytics,
    leaderboard: ::leaderboard::LeaderboardService,
    catalog: Catalog,
    entitlements: EntitlementStore,
    db: Option<Arc<Session>>,
}

async fn ws_handler(State(state): State<Arc<AppState>>, ws: WebSocketUpgrade) -> impl IntoResponse {
    state.analytics.dispatch(Event::WsConnected);
    state.analytics.dispatch(Event::SessionStart);
    ws.on_upgrade(|socket| async move {
        handle_socket(socket).await;
    })
}

async fn signal_ws_handler(
    State(state): State<Arc<AppState>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    state.analytics.dispatch(Event::WsConnected);
    state.analytics.dispatch(Event::SessionStart);
    ws.on_upgrade(move |socket| async move {
        handle_signal_socket(state, socket).await;
    })
}

async fn handle_signal_socket(state: Arc<AppState>, mut socket: WebSocket) {
    use axum::extract::ws::CloseFrame;
    use serde_json::json;

    if let Some(msg) = socket.recv().await {
        match msg {
            Ok(Message::Text(sdp)) => match ServerConnector::new().await {
                Ok(connector) => {
                    let mut offer = RTCSessionDescription::default();
                    offer.sdp_type = RTCSdpType::Offer;
                    offer.sdp = sdp;
                    if let Err(e) = connector.pc.set_remote_description(offer).await {
                        log::warn!("invalid SDP offer: {e}");
                        let _ = socket
                            .send(Message::Text(
                                json!({ "error": "invalid SDP offer" }).to_string(),
                            ))
                            .await;
                        let _ = socket
                            .send(Message::Close(Some(CloseFrame {
                                code: 1002,
                                reason: "invalid SDP".into(),
                            })))
                            .await;
                        return;
                    }

                    match connector.pc.create_answer(None).await {
                        Ok(answer) => {
                            if connector
                                .pc
                                .set_local_description(answer.clone())
                                .await
                                .is_err()
                            {
                                log::warn!("failed to set local description");
                                let _ = socket
                                    .send(Message::Close(Some(CloseFrame {
                                        code: 1011,
                                        reason: "pc error".into(),
                                    })))
                                    .await;
                                return;
                            }

                            let _ = socket.send(Message::Text(answer.sdp.clone())).await;
                            state.rooms.add_peer(connector).await;
                        }
                        Err(e) => {
                            log::warn!("failed to create answer: {e}");
                            let _ = socket
                                .send(Message::Close(Some(CloseFrame {
                                    code: 1011,
                                    reason: "pc error".into(),
                                })))
                                .await;
                            return;
                        }
                    }
                }
                Err(e) => {
                    log::warn!("failed to create peer connection: {e}");
                    let _ = socket
                        .send(Message::Close(Some(CloseFrame {
                            code: 1011,
                            reason: "pc error".into(),
                        })))
                        .await;
                    return;
                }
            },
            Ok(_) => {
                log::warn!("expected SDP offer");
                let _ = socket
                    .send(Message::Text(
                        json!({ "error": "invalid SDP offer" }).to_string(),
                    ))
                    .await;
                let _ = socket
                    .send(Message::Close(Some(CloseFrame {
                        code: 1003,
                        reason: "invalid message".into(),
                    })))
                    .await;
                return;
            }
            Err(e) => {
                log::warn!("websocket error: {e}");
                return;
            }
        }
    }
    let _ = socket.close().await;
}

async fn handle_socket(mut socket: WebSocket) {
    use axum::extract::ws::Message;

    while let Some(msg) = socket.recv().await {
        match msg {
            Ok(Message::Ping(payload)) => {
                let _ = socket.send(Message::Pong(payload)).await;
            }
            Ok(Message::Pong(_)) => {}
            Ok(Message::Close(_)) => break,
            Ok(Message::Text(text)) => {
                log::warn!("unexpected text message: {text}");
                let _ = socket.close().await;
                break;
            }
            Ok(Message::Binary(_)) => {
                log::warn!("unexpected binary message");
                let _ = socket.close().await;
                break;
            }
            Err(e) => {
                log::warn!("websocket error: {e}");
                break;
            }
        }
    }
}

#[derive(Deserialize)]
struct MailTestParams {
    to: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct MailTestResponse {
    queued: bool,
}

#[derive(Serialize)]
struct RedactedSmtpConfig {
    host: String,
    port: u16,
    from: String,
    starttls: StartTls,
    smtps: bool,
    timeout: u64,
    user: Option<String>,
    pass: Option<String>,
}

impl From<&SmtpConfig> for RedactedSmtpConfig {
    fn from(cfg: &SmtpConfig) -> Self {
        Self {
            host: cfg.host.clone(),
            port: cfg.port,
            from: cfg.from.clone(),
            starttls: cfg.starttls.clone(),
            smtps: cfg.smtps,
            timeout: cfg.timeout,
            user: cfg.user.clone(),
            pass: cfg.pass.as_ref().map(|_| "***".into()),
        }
    }
}

async fn mail_config_handler(State(state): State<Arc<AppState>>) -> Json<RedactedSmtpConfig> {
    Json(RedactedSmtpConfig::from(&state.smtp))
}

async fn mail_test_handler(
    State(state): State<Arc<AppState>>,
    query: Option<Query<MailTestParams>>,
    body: Option<Json<MailTestParams>>,
) -> Json<MailTestResponse> {
    let to = query
        .map(|q| q.0.to)
        .or_else(|| body.map(|b| b.0.to))
        .unwrap_or_else(|| state.email.from_address().to_string());
    let queued = if !EmailAddress::is_valid(&to) {
        log::warn!("invalid test email address: {to}");
        false
    } else {
        match state.email.send_test(&to) {
            Ok(()) => {
                state.analytics.dispatch(Event::MailTestQueued);
                true
            }
            Err(e) => {
                log::warn!("failed to queue test email to {to}: {e}");
                false
            }
        }
    };

    Json(MailTestResponse { queued })
}

#[derive(Serialize)]
struct StoreResponse {
    items: Vec<Sku>,
}

async fn store_handler(State(state): State<Arc<AppState>>) -> Json<StoreResponse> {
    state.analytics.dispatch(Event::StoreViewed);
    state.analytics.dispatch(Event::StoreOpen);
    Json(StoreResponse {
        items: state.catalog.all().to_vec(),
    })
}

#[derive(Deserialize)]
struct ClaimRequest {
    sku: String,
}

async fn store_claim_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<ClaimRequest>,
) -> StatusCode {
    let user = match headers
        .get("X-Session")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| UserId::parse_str(s).ok())
    {
        Some(u) => u,
        None => return StatusCode::UNAUTHORIZED,
    };

    state.entitlements.grant(user, req.sku.clone()).await;
    state.analytics.dispatch(Event::EntitlementGranted);
    StatusCode::OK
}

async fn entitlements_handler(
    State(state): State<Arc<AppState>>,
    Path(user): Path<String>,
) -> Json<EntitlementList> {
    let entitlements = state.entitlements.list(&user).await;
    Json(EntitlementList { entitlements })
}

async fn metrics_handler() -> impl IntoResponse {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}

#[derive(Serialize)]
struct GuestResponse {
    user_id: String,
}

async fn guest_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let id = uuid::Uuid::new_v4().to_string();
    if let Some(db) = &state.db {
        let query = "INSERT INTO players_by_id (id, guest) VALUES (?, true)";
        let _ = db.query(query, (id.clone(),)).await;
    }
    let mut headers = HeaderMap::new();
    headers.insert(
        SET_COOKIE,
        HeaderValue::from_str(&format!("session={}; Path=/; HttpOnly", id)).unwrap(),
    );
    (headers, Json(GuestResponse { user_id: id }))
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

async fn setup(cfg: &ResolvedConfig, smtp: SmtpConfig, analytics: Analytics) -> Result<AppState> {
    let smtp = smtp.validate()?;
    let email = Arc::new(EmailService::new(smtp.clone()).map_err(|e| {
        log::error!("failed to initialize email service: {e}");
        anyhow!(e)
    })?);

    let leaderboard =
        ::leaderboard::LeaderboardService::new(&cfg.database_url, PathBuf::from("replays"))
            .await
            .map_err(|e| anyhow!(e))?;
    let registry = Arc::new(shard::MemoryShardRegistry::new());
    let rooms = room::RoomManager::with_registry(
        leaderboard.clone(),
        registry,
        "shard1".into(),
        cfg.shard_host.clone(),
    );
    let catalog = Catalog::new(vec![Sku {
        id: "basic".to_string(),
        price_cents: 1000,
    }]);
    let db = match SessionBuilder::new()
        .known_node(&cfg.database_url)
        .build()
        .await
    {
        Ok(s) => Some(Arc::new(s)),
        Err(e) => {
            log::warn!("failed to connect to scylla: {e}");
            None
        }
    };
    let entitlements = EntitlementStore::new(db.clone());

    Ok(AppState {
        email,
        rooms,
        smtp,
        analytics,
        leaderboard,
        catalog,
        entitlements,
        db,
    })
}

async fn run(cli: Cli) -> Result<()> {
    let config = cli.config.resolve()?;
    log::info!("Using config: {:?}", config);
    let analytics = Analytics::new(
        !cli.analytics_opt_out,
        cli.posthog_key.clone(),
        cli.enable_otel,
    );
    let smtp = cli.smtp.validate()?;
    let state = Arc::new(setup(&config, smtp, analytics).await?);

    let assets_service =
        get_service(ServeDir::new("assets")).layer(SetResponseHeaderLayer::if_not_present(
            CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=31536000, immutable"),
        ));

    let app = Router::new()
        .route("/metrics", get(metrics_handler))
        .nest("/auth", auth::routes())
        .route("/auth/guest", post(guest_handler))
        .route("/ws", get(ws_handler))
        .route("/signal", get(signal_ws_handler))
        .route("/config.json", get(config::get_config))
        .route("/store", get(store_handler))
        .route("/store/claim", post(store_claim_handler))
        .route("/entitlements/:user", get(entitlements_handler))
        .route("/admin/mail/test", post(mail_test_handler))
        .route("/admin/mail/config", get(mail_config_handler))
        .nest("/leaderboard", leaderboard::routes())
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
            HeaderValue::from_str(
                &config
                    .csp
                    .clone()
                    .unwrap_or_else(|| "default-src 'self'".into()),
            )
            .expect("invalid content-security-policy"),
        ))
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind(config.bind_addr)
        .await
        .map_err(|e| {
            log::error!("failed to bind to address: {e}");
            e
        })?;

    let res = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await;
    state.email.abort_cleanup();
    res.map_err(|e| {
        log::error!("server error: {e}");
        e
    })?;

    Ok(())
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let cli = Cli::parse();
    if let Err(e) = run(cli).await {
        log::error!("{e}");
        std::process::exit(1);
    }
}
