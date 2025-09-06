use std::{net::SocketAddr, sync::Arc};

use axum::{
    Router,
    extract::{
        State,
        ws::{WebSocket, WebSocketUpgrade},
    },
    http::{HeaderName, HeaderValue, StatusCode, header::CACHE_CONTROL},
    response::IntoResponse,
    routing::{get, get_service},
};
use duck_hunt::DuckHuntPlugin;
use platform_api::ServerApp;
use server::{ServerAppExt, email::EmailService};
use sqlx::PgPool;
use tower_http::{services::ServeDir, set_header::SetResponseHeaderLayer};

#[derive(Clone)]
struct AppState {
    db: PgPool,
    email: Arc<EmailService>,
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

#[tokio::main]
async fn main() {
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL environment variable not set");
    let db = PgPool::connect(&database_url)
        .await
        .expect("failed to connect to database");

    let email = Arc::new(EmailService::new(
        &std::env::var("SMTP_SERVER").expect("SMTP_SERVER not set"),
        &std::env::var("SMTP_USERNAME").expect("SMTP_USERNAME not set"),
        &std::env::var("SMTP_PASSWORD").expect("SMTP_PASSWORD not set"),
        &std::env::var("EMAIL_FROM").expect("EMAIL_FROM not set"),
    ));

    let state = Arc::new(AppState { db, email });

    let mut _game_app = ServerApp::new();
    _game_app.add_game_module::<DuckHuntPlugin>();

    let assets_service =
        get_service(ServeDir::new("assets")).layer(SetResponseHeaderLayer::if_not_present(
            CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=31536000, immutable"),
        ));

    let app = Router::new()
        .nest("/auth", auth_routes())
        .route("/ws", get(ws_handler))
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
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}
