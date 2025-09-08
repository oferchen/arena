use super::*;
use analytics::{Analytics, Event};
use axum::body::Body;
use axum::extract::{Json, Query, State};
use axum::http::Request;
use futures_util::{SinkExt, StreamExt};
use serial_test::serial;
use std::env;
use tokio_tungstenite::tungstenite::Message;
use tower::ServiceExt;
use webrtc::api::APIBuilder;
use webrtc::api::media_engine::MediaEngine;
use webrtc::peer_connection::configuration::RTCConfiguration;

use crate::test_logger::{INIT, LOGGER};
use ::payments::{Catalog, Sku};
use crate::payments::EntitlementStore;
use std::sync::Arc;
use log::LevelFilter;
use std::path::PathBuf;

async fn leaderboard_service() -> ::leaderboard::LeaderboardService {
    std::env::set_var("ARENA_REDIS_URL", "redis://127.0.0.1/");
    ::leaderboard::LeaderboardService::new("127.0.0.1:9042", PathBuf::from("replays"))
        .await
        .unwrap()
}

fn smtp_cfg() -> SmtpConfig {
    SmtpConfig {
        host: "localhost".into(),
        from: "arena@localhost".into(),
        ..Default::default()
    }
}

#[tokio::test]
    async fn setup_succeeds_without_env_vars() {
        unsafe {
            env::remove_var("DATABASE_URL");
        }

        let analytics = Analytics::new(true, None, false);
        let cfg = ResolvedConfig {
            bind_addr: "127.0.0.1:3000".parse().unwrap(),
            public_base_url: "http://localhost".into(),
            signaling_ws_url: "ws://127.0.0.1".into(),
            db_url: "127.0.0.1:9042".into(),
            csp: None,
        };
        assert!(setup(&cfg, smtp_cfg(), analytics).await.is_ok());
    }

#[test]
fn cli_overrides_env() {
    unsafe {
        env::set_var("ARENA_SMTP_HOST", "envhost");
        env::set_var("ARENA_SMTP_FROM", "envfrom@example.com");
    }
    let cli = Cli::try_parse_from(["prog", "--smtp-host", "clihost"]).unwrap();
    assert_eq!(cli.smtp.host, "clihost");
    cli.smtp.validate().unwrap();
    unsafe {
        env::remove_var("ARENA_SMTP_HOST");
        env::remove_var("ARENA_SMTP_FROM");
    }
}

#[test]
fn env_used_when_no_cli() {
    unsafe {
        env::set_var("ARENA_SMTP_HOST", "envhost");
        env::set_var("ARENA_SMTP_FROM", "envfrom@example.com");
        env::set_var("ARENA_SMTP_PORT", "2525");
    }
    let cli = Cli::try_parse_from(["prog"]).unwrap();
    assert_eq!(cli.smtp.port, 2525);
    cli.smtp.validate().unwrap();
    unsafe {
        env::remove_var("ARENA_SMTP_HOST");
        env::remove_var("ARENA_SMTP_FROM");
        env::remove_var("ARENA_SMTP_PORT");
    }
}

#[test]
fn missing_bind_addr_errors() {
    unsafe {
        env::remove_var("ARENA_BIND_ADDR");
    }
    let cli = Cli::try_parse_from(["prog"]).unwrap();
    assert!(cli.config.clone().resolve().is_err());
}

#[test]
fn invalid_starttls_cli_value_errors() {
    unsafe {
        env::set_var("ARENA_SMTP_HOST", "envhost");
        env::set_var("ARENA_SMTP_FROM", "envfrom@example.com");
    }
    assert!(Cli::try_parse_from(["prog", "--smtp-starttls", "bogus"]).is_err());
    unsafe {
        env::remove_var("ARENA_SMTP_HOST");
        env::remove_var("ARENA_SMTP_FROM");
    }
}

#[test]
fn invalid_starttls_env_value_errors() {
    unsafe {
        env::set_var("ARENA_SMTP_HOST", "envhost");
        env::set_var("ARENA_SMTP_FROM", "envfrom@example.com");
        env::set_var("ARENA_SMTP_STARTTLS", "bogus");
    }
    assert!(Cli::try_parse_from(["prog"]).is_err());
    unsafe {
        env::remove_var("ARENA_SMTP_HOST");
        env::remove_var("ARENA_SMTP_FROM");
        env::remove_var("ARENA_SMTP_STARTTLS");
    }
}

#[tokio::test]
async fn websocket_signaling_completes_handshake() {
    let cfg = smtp_cfg();
    let email = Arc::new(EmailService::new(cfg.clone()).unwrap());
    let leaderboard = leaderboard_service().await;
    let rooms = room::RoomManager::new(leaderboard.clone());
    let state = Arc::new(AppState {
        email,
        rooms,
        smtp: cfg,
        analytics: Analytics::new(true, None, false),
        leaderboard: leaderboard.clone(),
        catalog: Catalog::new(vec![Sku {
            id: "basic".into(),
            price_cents: 1000,
        }]),
        entitlements: EntitlementStore::default(),
        db: None,
    });

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

#[tokio::test]
#[serial]
async fn websocket_signaling_invalid_sdp_logs_and_closes() {
    INIT.call_once(|| {
        log::set_logger(&LOGGER).unwrap();
    });
    log::set_max_level(LevelFilter::Warn);
    LOGGER.messages.lock().unwrap().clear();

    let cfg = smtp_cfg();
    let email = Arc::new(EmailService::new(cfg.clone()).unwrap());
    let leaderboard = leaderboard_service().await;
    let rooms = room::RoomManager::new(leaderboard.clone());
    let state = Arc::new(AppState {
        email,
        rooms,
        smtp: cfg,
        analytics: Analytics::new(true, None, false),
        leaderboard: leaderboard.clone(),
        catalog: Catalog::new(vec![Sku {
            id: "basic".into(),
            price_cents: 1000,
        }]),
        entitlements: EntitlementStore::default(),
        db: None,
    });

    let app = Router::new()
        .route("/signal", get(signal_ws_handler))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let (mut ws, _) =
        tokio_tungstenite::connect_async(format!("ws://{}/signal", addr))
            .await
            .unwrap();
    ws.send(Message::Text("bogus".into())).await.unwrap();

    let msg = ws.next().await.unwrap().unwrap();
    let err: serde_json::Value = serde_json::from_str(&msg.into_text().unwrap()).unwrap();
    assert_eq!(err["error"], "invalid SDP offer");

    let msg = ws.next().await.unwrap().unwrap();
    assert!(matches!(msg, Message::Close(_)));
    assert!(ws.next().await.is_none());

    let logs = LOGGER.messages.lock().unwrap();
    assert!(logs.iter().any(|m| m.contains("invalid SDP offer")));
}

#[tokio::test]
#[serial]
async fn websocket_signaling_unexpected_binary_logs_and_closes() {
    INIT.call_once(|| {
        log::set_logger(&LOGGER).unwrap();
    });
    log::set_max_level(LevelFilter::Warn);
    LOGGER.messages.lock().unwrap().clear();

    let cfg = smtp_cfg();
    let email = Arc::new(EmailService::new(cfg.clone()).unwrap());
    let leaderboard = leaderboard_service().await;
    let rooms = room::RoomManager::new(leaderboard.clone());
    let state = Arc::new(AppState {
        email,
        rooms,
        smtp: cfg,
        analytics: Analytics::new(true, None, false),
        leaderboard: leaderboard.clone(),
        catalog: Catalog::new(vec![Sku {
            id: "basic".into(),
            price_cents: 1000,
        }]),
        entitlements: EntitlementStore::default(),
        db: None,
    });

    let app = Router::new()
        .route("/signal", get(signal_ws_handler))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let (mut ws, _) =
        tokio_tungstenite::connect_async(format!("ws://{}/signal", addr))
            .await
            .unwrap();
    ws.send(Message::Binary(vec![1, 2, 3])).await.unwrap();

    let msg = ws.next().await.unwrap().unwrap();
    let err: serde_json::Value = serde_json::from_str(&msg.into_text().unwrap()).unwrap();
    assert_eq!(err["error"], "invalid SDP offer");

    let msg = ws.next().await.unwrap().unwrap();
    assert!(matches!(msg, Message::Close(_)));
    assert!(ws.next().await.is_none());

    let logs = LOGGER.messages.lock().unwrap();
    assert!(logs.iter().any(|m| m.contains("expected SDP offer")));
}

#[tokio::test]
#[serial]
async fn websocket_logs_unexpected_messages_and_closes() {
    INIT.call_once(|| {
        log::set_logger(&LOGGER).unwrap();
    });
    log::set_max_level(LevelFilter::Warn);
    LOGGER.messages.lock().unwrap().clear();

    let cfg = smtp_cfg();
    let email = Arc::new(EmailService::new(cfg.clone()).unwrap());
    let leaderboard = leaderboard_service().await;
    let rooms = room::RoomManager::new(leaderboard.clone());
    let state = Arc::new(AppState {
        email,
        rooms,
        smtp: cfg,
        analytics: Analytics::new(true, None, false),
        leaderboard: leaderboard.clone(),
        catalog: Catalog::new(vec![Sku {
            id: "basic".into(),
            price_cents: 1000,
        }]),
        entitlements: EntitlementStore::default(),
        db: None,
    });

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/ws"))
        .await
        .unwrap();
    ws.send(Message::Text("unexpected".into())).await.unwrap();

    let msg = ws.next().await.unwrap().unwrap();
    assert!(matches!(msg, Message::Close(_)));
    assert!(ws.next().await.is_none());

    let logs = LOGGER.messages.lock().unwrap();
    assert!(logs.iter().any(|m| m.contains("unexpected text message")));
}

#[tokio::test]
#[serial]
async fn mail_test_defaults_to_from_address() {
    let mut cfg = smtp_cfg();
    cfg.from = "default@example.com".into();
    let email = Arc::new(EmailService::new(cfg.clone()).unwrap());
    let leaderboard = leaderboard_service().await;
    let rooms = room::RoomManager::new(leaderboard.clone());
    let state = Arc::new(AppState {
        email,
        rooms,
        smtp: cfg.clone(),
        analytics: Analytics::new(true, None, false),
        leaderboard: leaderboard.clone(),
        catalog: Catalog::new(vec![Sku {
            id: "basic".into(),
            price_cents: 1000,
        }]),
        entitlements: EntitlementStore::default(),
        db: None,
    });

    assert_eq!(
        mail_test_handler(State(state.clone()), None, None).await.0,
        MailTestResponse { queued: true }
    );
    assert_eq!(
        mail_test_handler(State(state.clone()), None, None).await.0,
        MailTestResponse { queued: false }
    );
}

#[tokio::test]
#[serial]
async fn mail_test_accepts_user_address_query() {
    let mut cfg = smtp_cfg();
    cfg.from = "query@example.com".into();
    let email = Arc::new(EmailService::new(cfg.clone()).unwrap());
    let leaderboard = leaderboard_service().await;
    let rooms = room::RoomManager::new(leaderboard.clone());
    let state = Arc::new(AppState {
        email,
        rooms,
        smtp: cfg.clone(),
        analytics: Analytics::new(true, None, false),
        leaderboard: leaderboard.clone(),
        catalog: Catalog::new(vec![Sku {
            id: "basic".into(),
            price_cents: 1000,
        }]),
        entitlements: EntitlementStore::default(),
        db: None,
    });

    assert_eq!(
        mail_test_handler(State(state.clone()), None, None).await.0,
        MailTestResponse { queued: true }
    );

    let query = Query(MailTestParams {
        to: "user_q@example.com".into(),
    });

    assert_eq!(
        mail_test_handler(State(state.clone()), Some(query), None)
            .await
            .0,
        MailTestResponse { queued: true }
    );
}

#[tokio::test]
#[serial]
async fn mail_test_accepts_user_address_body() {
    let mut cfg = smtp_cfg();
    cfg.from = "body@example.com".into();
    let email = Arc::new(EmailService::new(cfg.clone()).unwrap());
    let leaderboard = leaderboard_service().await;
    let rooms = room::RoomManager::new(leaderboard.clone());
    let state = Arc::new(AppState {
        email,
        rooms,
        smtp: cfg.clone(),
        analytics: Analytics::new(true, None, false),
        leaderboard: leaderboard.clone(),
        catalog: Catalog::new(vec![Sku {
            id: "basic".into(),
            price_cents: 1000,
        }]),
        entitlements: EntitlementStore::default(),
        db: None,
    });

    assert_eq!(
        mail_test_handler(State(state.clone()), None, None).await.0,
        MailTestResponse { queued: true }
    );

    let body = Json(MailTestParams {
        to: "user_b@example.com".into(),
    });

    assert_eq!(
        mail_test_handler(State(state.clone()), None, Some(body))
            .await
            .0,
        MailTestResponse { queued: true }
    );
}

#[tokio::test]
async fn mail_config_redacts_password() {
    let mut cfg = smtp_cfg();
    cfg.pass = Some("secret".into());
    let email = Arc::new(EmailService::new(cfg.clone()).unwrap());
    let leaderboard = leaderboard_service().await;
    let rooms = room::RoomManager::new(leaderboard.clone());
    let state = Arc::new(AppState {
        email,
        rooms,
        smtp: cfg.clone(),
        analytics: Analytics::new(true, None, false),
        leaderboard: leaderboard.clone(),
        catalog: Catalog::new(vec![Sku {
            id: "basic".into(),
            price_cents: 1000,
        }]),
        entitlements: EntitlementStore::default(),
        db: None,
    });

    let Json(redacted) = mail_config_handler(State(state)).await;
    assert_eq!(redacted.pass, Some("***".into()));
    assert_eq!(redacted.user, None);
}

#[tokio::test]
async fn admin_mail_config_route() {
    let cfg = smtp_cfg();
    let email = Arc::new(EmailService::new(cfg.clone()).unwrap());
    let leaderboard = leaderboard_service().await;
    let rooms = room::RoomManager::new(leaderboard.clone());
    let state = Arc::new(AppState {
        email,
        rooms,
        smtp: cfg,
        analytics: Analytics::new(true, None, false),
        leaderboard: leaderboard.clone(),
        catalog: Catalog::new(vec![Sku {
            id: "basic".into(),
            price_cents: 1000,
        }]),
        entitlements: EntitlementStore::default(),
        db: None,
    });

    let app = Router::new()
        .route("/admin/mail/config", get(mail_config_handler))
        .route("/admin/mail/test", post(mail_test_handler))
        .with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/admin/mail/config")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}


#[tokio::test]
async fn round_scores_appear_in_leaderboard() {
    use ::leaderboard::models::Score;
    use std::time::Duration;

    let cfg = smtp_cfg();
    let email = Arc::new(EmailService::new(cfg.clone()).unwrap());
    let leaderboard = leaderboard_service().await;
    let rooms = room::RoomManager::new(leaderboard.clone());
    rooms.push_score(7).await;
    let state = Arc::new(AppState {
        email,
        rooms: rooms.clone(),
        smtp: cfg,
        analytics: Analytics::new(true, None, false),
        leaderboard: leaderboard.clone(),
        catalog: Catalog::new(vec![Sku {
            id: "basic".into(),
            price_cents: 1000,
        }]),
        entitlements: EntitlementStore::default(),
        db: None,
    });

    let app = Router::new()
        .nest("/leaderboard", crate::leaderboard::routes())
        .with_state(state);

    tokio::time::sleep(Duration::from_secs(2)).await;
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/leaderboard/{}", room::LEADERBOARD_ID))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let scores: Vec<Score> = serde_json::from_slice(&body).unwrap();
    assert!(scores.iter().any(|s| s.points == 7));
}
