use std::{net::SocketAddr, path::PathBuf, sync::Arc};

use anyhow::{Result, anyhow};

use crate::email::{EmailService, SmtpConfig, StartTls};
use ::payments::{Catalog, Entitlement, EntitlementList, EntitlementStore, Sku, StripeClient};
use analytics::{Analytics, Event};
use axum::{
    Router,
    extract::{
        Json, Path, Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderName, HeaderValue, StatusCode, header::CACHE_CONTROL},
    response::IntoResponse,
    routing::{get, get_service, post},
};
use clap::Parser;
use email_address::EmailAddress;
use net::server::ServerConnector;
use serde::{Deserialize, Serialize};
use webrtc::peer_connection::sdp::sdp_type::RTCSdpType;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

mod email;
mod leaderboard;
mod payments;
mod room;
#[cfg(test)]
mod test_logger;
#[cfg(test)]
mod tests;
use prometheus::{Encoder, TextEncoder};
use tower_http::{services::ServeDir, set_header::SetResponseHeaderLayer};

#[derive(Parser, Debug)]
struct Cli {
    #[command(flatten)]
    smtp: SmtpConfig,
    #[arg(long, env = "POSTHOG_KEY")]
    posthog_key: Option<String>,
    #[arg(long, env = "ENABLE_OTEL", default_value_t = false)]
    enable_otel: bool,
}

#[derive(Clone)]
pub(crate) struct AppState {
    email: Arc<EmailService>,
    rooms: room::RoomManager,
    smtp: SmtpConfig,
    analytics: Analytics,
    leaderboard: ::leaderboard::LeaderboardService,
    catalog: Catalog,
    stripe: StripeClient,
    entitlements: EntitlementStore,
    entitlements_path: std::path::PathBuf,
}

fn auth_routes() -> Router<Arc<AppState>> {
    Router::new().route("/*path", get(|| async { StatusCode::OK }))
}

async fn ws_handler(State(state): State<Arc<AppState>>, ws: WebSocketUpgrade) -> impl IntoResponse {
    state.analytics.dispatch(Event::WsConnected);
    ws.on_upgrade(|socket| async move {
        handle_socket(socket).await;
    })
}

async fn signal_ws_handler(
    State(state): State<Arc<AppState>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    state.analytics.dispatch(Event::WsConnected);
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

    let queued = EmailAddress::is_valid(&to) && state.email.send_test(&to).is_ok();

    if queued {
        state.analytics.dispatch(Event::MailTestQueued);
    }

    Json(MailTestResponse { queued })
}

#[derive(Serialize)]
struct StoreResponse {
    items: Vec<Sku>,
}

async fn store_handler(State(state): State<Arc<AppState>>) -> Json<StoreResponse> {
    state.analytics.dispatch(Event::StoreViewed);
    Json(StoreResponse {
        items: state.catalog.all().to_vec(),
    })
}

#[derive(Deserialize)]
struct PurchaseRequest {
    user: String,
    sku: String,
}

#[derive(Serialize)]
struct PurchaseResponse {
    session_id: String,
}

async fn purchase_start_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PurchaseRequest>,
) -> Json<PurchaseResponse> {
    state.analytics.dispatch(Event::PurchaseInitiated);
    let session_id = ::payments::initiate_purchase(&req.user, &req.sku);
    Json(PurchaseResponse { session_id })
}

#[derive(Deserialize)]
struct StripeWebhook {
    r#type: String,
    data: StripeWebhookData,
}

#[derive(Deserialize)]
struct StripeWebhookData {
    object: StripeSession,
}

#[derive(Deserialize)]
struct StripeSession {
    client_reference_id: String,
    metadata: Option<StripeMetadata>,
}

#[derive(Deserialize)]
struct StripeMetadata {
    sku: String,
}

async fn stripe_webhook_handler(
    State(state): State<Arc<AppState>>,
    Json(event): Json<StripeWebhook>,
) -> StatusCode {
    if event.r#type == "checkout.session.completed" {
        if let Some(meta) = event.data.object.metadata {
            ::payments::complete_purchase(
                &state.entitlements,
                &event.data.object.client_reference_id,
                &meta.sku,
            );
            let _ = state.entitlements.save(&state.entitlements_path);
            state.analytics.dispatch(Event::PurchaseSucceeded);
            state.analytics.dispatch(Event::EntitlementGranted);
            StatusCode::OK
        } else {
            StatusCode::BAD_REQUEST
        }
    } else {
        StatusCode::BAD_REQUEST
    }
}

async fn entitlements_handler(
    State(state): State<Arc<AppState>>,
    Path(user): Path<String>,
) -> Json<EntitlementList> {
    let entitlements = state.entitlements.list(&user);
    Json(EntitlementList { entitlements })
}

async fn metrics_handler() -> impl IntoResponse {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
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

async fn setup(smtp: SmtpConfig, analytics: Analytics) -> Result<AppState> {
    let email = Arc::new(EmailService::new(smtp.clone()).map_err(|e| {
        log::error!("failed to initialize email service: {e}");
        anyhow!(e)
    })?);

    let rooms = room::RoomManager::new();
    let leaderboard = ::leaderboard::LeaderboardService::default();
    let catalog = Catalog::new(vec![Sku {
        id: "basic".to_string(),
        price_cents: 1000,
    }]);
    let stripe = StripeClient::new();
    let entitlements_path = PathBuf::from("entitlements.json");
    let entitlements = match std::fs::read(&entitlements_path) {
        Ok(data) => {
            let store = EntitlementStore::default();
            if let Ok(existing) = serde_json::from_slice::<Vec<Entitlement>>(&data) {
                for ent in existing {
                    store.grant(ent.user_id, ent.sku_id);
                }
            }
            store
        }
        Err(_) => EntitlementStore::default(),
    };
    Ok(AppState {
        email,
        rooms,
        smtp,
        analytics,
        leaderboard,
        catalog,
        stripe,
        entitlements,
        entitlements_path,
    })
}

async fn run(cli: Cli) -> Result<()> {
    let analytics = Analytics::new(cli.posthog_key.clone(), cli.enable_otel);
    let smtp = cli.smtp;
    let state = Arc::new(setup(smtp, analytics).await?);

    let assets_service =
        get_service(ServeDir::new("assets")).layer(SetResponseHeaderLayer::if_not_present(
            CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=31536000, immutable"),
        ));

    let app = Router::new()
        .route("/metrics", get(metrics_handler))
        .nest("/auth", auth_routes())
        .route("/ws", get(ws_handler))
        .route("/signal", get(signal_ws_handler))
        .route("/store", get(store_handler))
        .route("/purchase/start", post(purchase_start_handler))
        .route("/stripe/webhook", post(stripe_webhook_handler))
        .route("/entitlements/:user", get(entitlements_handler))
        .route("/admin/mail/test", post(mail_test_handler))
        .route("/admin/mail/config", get(mail_config_handler))
        .nest("/leaderboard", leaderboard::routes())
        .nest("/payments", payments::routes())
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
        .with_state(state.clone());

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| {
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
