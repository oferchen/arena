use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use chrono::Utc;
use serde::Deserialize;
use uuid::Uuid;

use ::leaderboard::{
    LeaderboardService,
    models::{LeaderboardWindow, Run, Score},
};
use analytics::Event;

use crate::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/:id", get(get_scores))
        .route("/:id/ws", get(ws_scores))
        .route("/:id/run", post(post_run))
        .route("/:id/run/:run_id/replay", get(get_replay))
        .route("/:id/run/:run_id/verify", post(post_verify))
}

#[derive(Deserialize)]
struct WindowQuery {
    window: Option<LeaderboardWindow>,
}

async fn get_scores(
    Path(id): Path<Uuid>,
    Query(q): Query<WindowQuery>,
    State(state): State<Arc<AppState>>,
) -> Json<Vec<Score>> {
    let window = q.window.unwrap_or(LeaderboardWindow::AllTime);
    let scores = state.leaderboard.get_scores(id, window).await;
    Json(scores)
}

#[derive(Deserialize)]
struct SubmitRun {
    player_id: Uuid,
    points: i32,
    replay: String,
}

async fn post_run(
    Path(id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SubmitRun>,
) -> StatusCode {
    let run_id = Uuid::new_v4();
    let score_id = Uuid::new_v4();
    let replay_bytes = match base64::decode(payload.replay) {
        Ok(bytes) => bytes,
        Err(_) => return StatusCode::BAD_REQUEST,
    };
    let verified = verify_score(&replay_bytes);
    if verified != Some(payload.points) {
        state.analytics.dispatch(Event::RunVerificationFailed);
        return StatusCode::BAD_REQUEST;
    }

    let run = Run {
        id: run_id,
        leaderboard_id: id,
        player_id: payload.player_id,
        replay_path: String::new(),
        created_at: Utc::now(),
        flagged: false,
        replay_index: 0,
    };
    let score = Score {
        id: score_id,
        run_id,
        player_id: payload.player_id,
        points: payload.points,
        verified: false,
        created_at: Utc::now(),
        window: LeaderboardWindow::AllTime,
    };
    if state
        .leaderboard
        .submit_score(id, score, run, replay_bytes)
        .await
        .is_err()
    {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    StatusCode::CREATED
}

async fn get_replay(
    Path((_id, run_id)): Path<(Uuid, Uuid)>,
    State(state): State<Arc<AppState>>,
) -> Result<Vec<u8>, StatusCode> {
    if let Some(data) = state.leaderboard.get_replay(run_id).await {
        Ok(data)
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

async fn post_verify(
    Path((_id, run_id)): Path<(Uuid, Uuid)>,
    State(state): State<Arc<AppState>>,
) -> StatusCode {
    if state.leaderboard.verify_run(run_id).await {
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn ws_scores(
    Path(id): Path<Uuid>,
    Query(q): Query<WindowQuery>,
    State(state): State<Arc<AppState>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let service = state.leaderboard.clone();
    let window = q.window.unwrap_or(LeaderboardWindow::AllTime);
    ws.on_upgrade(move |socket| async move {
        handle_ws(socket, id, window, service).await;
    })
}

async fn handle_ws(
    mut socket: WebSocket,
    id: Uuid,
    window: LeaderboardWindow,
    service: LeaderboardService,
) {
    let mut rx = service.subscribe();
    if let Ok(json) = serde_json::to_string(&service.get_scores(id, window).await) {
        let _ = socket.send(Message::Text(json)).await;
    }
    while let Ok(snapshot) = rx.recv().await {
        if snapshot.leaderboard != id || snapshot.window != window {
            continue;
        }
        if let Ok(json) = serde_json::to_string(&snapshot) {
            if socket.send(Message::Text(json)).await.is_err() {
                break;
            }
        }
    }
}

fn verify_score(replay: &[u8]) -> Option<i32> {
    if replay.len() < 4 {
        return None;
    }
    let mut bytes = [0u8; 4];
    bytes.copy_from_slice(&replay[..4]);
    Some(i32::from_le_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        email::{EmailService, SmtpConfig},
        room,
    };
    use ::payments::{Catalog, EntitlementStore, Sku, StripeClient};
    use analytics::Analytics;
    use axum::Json;
    use axum::extract::{Path, State};
    use leaderboard::models::LeaderboardWindow;
    use std::path::PathBuf;

    #[tokio::test]
    async fn post_run_rejects_malformed_base64() {
        let cfg = SmtpConfig::default();
        let email = Arc::new(EmailService::new(cfg.clone()).unwrap());
        let leaderboard =
            ::leaderboard::LeaderboardService::new("sqlite::memory:", PathBuf::from("replays"))
                .await
                .unwrap();
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
            stripe: StripeClient::new(),
            entitlements: EntitlementStore::default(),
            entitlements_path: PathBuf::new(),
        });

        let leaderboard_id = Uuid::new_v4();
        let payload = SubmitRun {
            player_id: Uuid::new_v4(),
            points: 42,
            replay: "not base64".into(),
        };

        let status = post_run(Path(leaderboard_id), State(state.clone()), Json(payload)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(
            state
                .leaderboard
                .get_scores(leaderboard_id, LeaderboardWindow::AllTime)
                .await
                .is_empty()
        );
    }

    #[tokio::test]
    async fn post_run_rejects_invalid_score() {
        let cfg = SmtpConfig::default();
        let email = Arc::new(EmailService::new(cfg.clone()).unwrap());
        let leaderboard =
            ::leaderboard::LeaderboardService::new("sqlite::memory:", PathBuf::from("replays"))
                .await
                .unwrap();
        let rooms = room::RoomManager::new(leaderboard.clone());
        let state = Arc::new(AppState {
            email,
            rooms,
            smtp: cfg,
            analytics: Analytics::new(true, None, false),
            leaderboard: leaderboard.clone(),
            catalog: Catalog::new(vec![]),
            stripe: StripeClient::new(),
            entitlements: EntitlementStore::default(),
            entitlements_path: PathBuf::new(),
        });

        let leaderboard_id = Uuid::new_v4();
        let mut bytes = 41i32.to_le_bytes().to_vec();
        bytes.extend_from_slice(b"rest");
        let replay = base64::encode(bytes);
        let payload = SubmitRun {
            player_id: Uuid::new_v4(),
            points: 42,
            replay,
        };

        let status = post_run(Path(leaderboard_id), State(state.clone()), Json(payload)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(
            state
                .leaderboard
                .get_scores(leaderboard_id, LeaderboardWindow::AllTime)
                .await
                .is_empty()
        );
    }

    #[tokio::test]
    async fn verify_endpoint_marks_score_verified() {
        let cfg = SmtpConfig::default();
        let email = Arc::new(EmailService::new(cfg.clone()).unwrap());
        let leaderboard =
            ::leaderboard::LeaderboardService::new("sqlite::memory:", PathBuf::from("replays"))
                .await
                .unwrap();
        let rooms = room::RoomManager::new(leaderboard.clone());
        let state = Arc::new(AppState {
            email,
            rooms,
            smtp: cfg,
            analytics: Analytics::new(true, None, false),
            leaderboard: leaderboard.clone(),
            catalog: Catalog::new(vec![]),
            stripe: StripeClient::new(),
            entitlements: EntitlementStore::default(),
            entitlements_path: PathBuf::new(),
        });

        let leaderboard_id = Uuid::new_v4();
        let player = Uuid::new_v4();
        let replay = base64::encode(10i32.to_le_bytes());
        let payload = SubmitRun {
            player_id: player,
            points: 10,
            replay,
        };
        let status = post_run(Path(leaderboard_id), State(state.clone()), Json(payload)).await;
        assert_eq!(status, StatusCode::CREATED);
        let scores = state
            .leaderboard
            .get_scores(leaderboard_id, LeaderboardWindow::AllTime)
            .await;
        assert_eq!(scores.len(), 1);
        let run_id = scores[0].run_id;
        assert!(!scores[0].verified);

        let status = post_verify(Path((leaderboard_id, run_id)), State(state.clone())).await;
        assert_eq!(status, StatusCode::OK);
        let scores = state
            .leaderboard
            .get_scores(leaderboard_id, LeaderboardWindow::AllTime)
            .await;
        assert!(scores[0].verified);
    }
}
