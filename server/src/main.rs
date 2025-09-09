use std::{collections::HashMap, net::SocketAddr, path::PathBuf, sync::Arc};

use anyhow::{Result, anyhow};

use crate::email::{EmailService, SmtpConfig, StartTls};
use ::payments::{Catalog, EntitlementList, Sku, UserId};
use analytics::{Analytics, Event};
use axum::{
    Extension, Router,
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
use migration::{Migrator, MigratorTrait};
use net::server::ServerConnector;
use payments::EntitlementStore;
use sea_orm::Database;
use serde::{Deserialize, Serialize};
use sea_orm::{DatabaseConnection, DbBackend, Statement};
use storage::connect as connect_db;
use webrtc::peer_connection::sdp::sdp_type::RTCSdpType;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

mod auth;
mod config;
mod email;
mod leaderboard;
mod otp_store;
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
    #[arg(long, env = "ARENA_POSTHOG_KEY")]
    posthog_key: Option<String>,
    #[arg(long = "metrics-addr", env = "ARENA_METRICS_ADDR")]
    metrics_addr: Option<SocketAddr>,
    #[arg(long, env = "ARENA_ANALYTICS_OPT_OUT", default_value_t = false)]
    analytics_opt_out: bool,
}

#[derive(Parser, Debug, Clone)]
struct Config {
    #[arg(long, env = "ARENA_BIND_ADDR")]
    bind_addr: Option<SocketAddr>,
    #[arg(long, env = "ARENA_PUBLIC_BASE_URL")]
    public_base_url: Option<String>,
    #[arg(long, env = "ARENA_SIGNALING_WS_URL")]
    signaling_ws_url: Option<String>,
    #[arg(long, env = "ARENA_DB_URL")]
    db_url: Option<String>,
    #[arg(long, env = "ARENA_DB_MAX_CONNS")]
    db_max_conns: Option<u32>,
    #[arg(long, env = "ARENA_MIGRATE_ON_START", default_value_t = false)]
    migrate_on_start: bool,
    #[arg(long, env = "ARENA_ENABLE_COOP_COEP", default_value_t = false)]
    enable_coop_coep: bool,
    #[arg(long, env = "ARENA_STATIC_DIR")]
    static_dir: Option<PathBuf>,
    #[arg(long, env = "ARENA_ASSETS_DIR")]
    assets_dir: Option<PathBuf>,
    #[arg(long, env = "ARENA_ENABLE_SW", default_value_t = false)]
    enable_sw: bool,
    #[arg(long, env = "ARENA_CSP")]
    csp: Option<String>,
    #[arg(long, env = "ARENA_RTC_ICE_SERVERS_JSON")]
    rtc_ice_servers_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IceServerConfig {
    #[serde(
        deserialize_with = "deserialize_urls",
        serialize_with = "serialize_urls"
    )]
    pub urls: Vec<String>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub credential: Option<String>,
}

fn deserialize_urls<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Urls {
        Single(String),
        Multiple(Vec<String>),
    }

    match Urls::deserialize(deserializer)? {
        Urls::Single(url) => Ok(vec![url]),
        Urls::Multiple(urls) => Ok(urls),
    }
}

fn serialize_urls<S>(urls: &Vec<String>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::ser::Serializer,
{
    urls.serialize(serializer)
}

#[derive(Debug, Clone)]
pub struct ResolvedConfig {
    pub bind_addr: SocketAddr,
    pub public_base_url: String,
    pub signaling_ws_url: String,
    pub db_url: String,
    pub db_max_conns: u32,
    pub migrate_on_start: bool,
    pub enable_coop_coep: bool,
    pub static_dir: PathBuf,
    pub assets_dir: PathBuf,
    pub enable_sw: bool,
    pub csp: Option<String>,
    pub ice_servers: Vec<IceServerConfig>,
    pub feature_flags: HashMap<String, bool>,
    pub analytics_enabled: bool,
    pub analytics_opt_out: bool,
}

impl Config {
    fn resolve(self) -> Result<ResolvedConfig> {
        let ice_servers = if let Some(json) = self.rtc_ice_servers_json {
            serde_json::from_str(&json)
                .map_err(|e| anyhow!("invalid ARENA_RTC_ICE_SERVERS_JSON: {e}"))?
        } else {
            Vec::new()
        };
        let feature_flags = std::env::vars()
            .filter_map(|(k, v)| {
                k.strip_prefix("ARENA_FEATURE_").map(|name| {
                    let enabled =
                        matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on");
                    (name.to_ascii_lowercase(), enabled)
                })
            })
            .collect();

        Ok(ResolvedConfig {
            bind_addr: self
                .bind_addr
                .ok_or_else(|| anyhow!("ARENA_BIND_ADDR not set"))?,
            public_base_url: self
                .public_base_url
                .ok_or_else(|| anyhow!("ARENA_PUBLIC_BASE_URL not set"))?,
            signaling_ws_url: self
                .signaling_ws_url
                .ok_or_else(|| anyhow!("ARENA_SIGNALING_WS_URL not set"))?,
            db_url: self.db_url.ok_or_else(|| anyhow!("ARENA_DB_URL not set"))?,
            db_max_conns: self
                .db_max_conns
                .ok_or_else(|| anyhow!("ARENA_DB_MAX_CONNS not set"))?,
            migrate_on_start: self.migrate_on_start,
            enable_coop_coep: self.enable_coop_coep,
            static_dir: self
                .static_dir
                .ok_or_else(|| anyhow!("ARENA_STATIC_DIR not set"))?,
            assets_dir: self
                .assets_dir
                .ok_or_else(|| anyhow!("ARENA_ASSETS_DIR not set"))?,
            enable_sw: self.enable_sw,
            csp: self.csp,
            ice_servers,
            feature_flags,
            analytics_enabled: false,
            analytics_opt_out: false,
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
    db: Option<DatabaseConnection>,
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
            port: cfg.port.expect("validated"),
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
    match encoder.encode(&metric_families, &mut buffer) {
        Ok(_) => match String::from_utf8(buffer) {
            Ok(s) => s.into_response(),
            Err(e) => {
                log::error!("failed to convert metrics to UTF-8: {e}");
                String::new().into_response()
            }
        },
        Err(e) => {
            log::error!("failed to encode metrics: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[derive(Serialize)]
struct GuestResponse {
    user_id: String,
}

async fn guest_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let id = uuid::Uuid::new_v4().to_string();
    if let Some(db) = &state.db {
        let stmt = Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO players_by_id (id, guest) VALUES ($1, true)",
            vec![id.clone().into()],
        );
        let _ = db.execute(stmt).await;
    }
    let mut headers = HeaderMap::new();
    let same_site = std::env::var("ARENA_COOKIE_SAME_SITE").unwrap_or_else(|_| "Strict".into());
    let secure = std::env::var("ARENA_COOKIE_SECURE")
        .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(true);
    let cookie = format!(
        "session={}; Path=/; HttpOnly;{} SameSite={}",
        id,
        if secure { " Secure;" } else { "" },
        same_site
    );
    match HeaderValue::from_str(&cookie) {
        Ok(value) => {
            headers.insert(SET_COOKIE, value);
        }
        Err(e) => {
            log::error!("failed to create session cookie header: {e}");
        }
    }
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
    let email = Arc::new(EmailService::new(smtp.clone()).map_err(|e| {
        log::error!("failed to initialize email service: {e}");
        anyhow!(e)
    })?);

    let leaderboard = ::leaderboard::LeaderboardService::new(&cfg.db_url, PathBuf::from("replays"))
        .await
        .map_err(|e| anyhow!(e))?;
    let registry = Arc::new(shard::MemoryShardRegistry::new());
    let rooms = room::RoomManager::with_registry(
        leaderboard.clone(),
        registry,
        "shard1".into(),
        cfg.signaling_ws_url.clone(),
    );
    let catalog = Catalog::new(vec![Sku {
        id: "basic".to_string(),
        price_cents: 1000,
    }]);
    if cfg.migrate_on_start {
        let migration_db = Database::connect(&cfg.db_url).await?;
        Migrator::up(&migration_db, None).await?;
    }
    let db = connect_db(&cfg.db_url, cfg.db_max_conns).await?;
    let entitlements = EntitlementStore::new(Some(db.clone()));

    Ok(AppState {
        email,
        rooms,
        smtp,
        analytics,
        leaderboard,
        catalog,
        entitlements,
        db: Some(db),
    })
}

async fn run(cli: Cli) -> Result<()> {
    let Cli {
        smtp,
        config,
        posthog_key,
        metrics_addr,
        analytics_opt_out,
    } = cli;
    let mut config = config.resolve()?;
    config.analytics_enabled = posthog_key.is_some();
    config.analytics_opt_out = analytics_opt_out;
    log::info!("Using config: {:?}", config);
    let analytics = Analytics::new(
        config.analytics_enabled && !config.analytics_opt_out,
        posthog_key.clone(),
        metrics_addr,
    );
    let state = Arc::new(setup(&config, smtp, analytics).await?);

    let assets_service =
        get_service(ServeDir::new(&config.assets_dir)).layer(SetResponseHeaderLayer::if_not_present(
            CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=31536000, immutable"),
        ));

    let mut app = Router::new()
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
        .fallback_service(ServeDir::new(&config.static_dir));

    if config.enable_coop_coep {
        app = app
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
            ));
    }

    let app = app
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
        .layer(Extension(config.clone()))
        .with_state(state.clone());

    if let Some(addr) = metrics_addr {
        let metrics_app = Router::new().route("/metrics", get(metrics_handler));
        tokio::spawn(async move {
            let listener = match tokio::net::TcpListener::bind(addr).await {
                Ok(l) => l,
                Err(e) => {
                    log::error!("failed to bind metrics address: {e}");
                    return;
                }
            };
            if let Err(e) = axum::serve(listener, metrics_app).await {
                log::error!("metrics server error: {e}");
            }
        });
    }

    let listener = tokio::net::TcpListener::bind(config.bind_addr)
        .await
        .map_err(|e| {
            log::error!("failed to bind to address: {e}");
            e
        })?;

    let res = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await;
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
