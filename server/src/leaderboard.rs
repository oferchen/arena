use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use chrono::Utc;
use serde::Deserialize;
use uuid::Uuid;

use ::leaderboard::{
    LeaderboardService,
    models::{Run, Score},
};

use crate::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/:id", get(get_scores))
        .route("/:id/ws", get(ws_scores))
        .route("/:id/run", post(post_run))
        .route("/:id/run/:run_id/replay", get(get_replay))
}

async fn get_scores(Path(id): Path<Uuid>, State(state): State<Arc<AppState>>) -> Json<Vec<Score>> {
    let scores = state.leaderboard.get_scores(id).await;
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
    let run = Run {
        id: run_id,
        leaderboard_id: id,
        player_id: payload.player_id,
        replay_path: String::new(),
        created_at: Utc::now(),
    };
    let score = Score {
        id: score_id,
        run_id,
        player_id: payload.player_id,
        points: payload.points,
    };
    match state
        .leaderboard
        .submit_score(id, score, run, replay_bytes)
        .await
    {
        Ok(_) => StatusCode::CREATED,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
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

async fn ws_scores(
    Path(id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let service = state.leaderboard.clone();
    ws.on_upgrade(move |socket| async move {
        handle_ws(socket, id, service).await;
    })
}

async fn handle_ws(mut socket: WebSocket, id: Uuid, service: LeaderboardService) {
    let mut rx = service.subscribe();
    if let Ok(json) = serde_json::to_string(&service.get_scores(id).await) {
        let _ = socket.send(Message::Text(json)).await;
    }
    while let Ok(snapshot) = rx.recv().await {
        if snapshot.leaderboard != id {
            continue;
        }
        if let Ok(json) = serde_json::to_string(&snapshot.scores) {
            if socket.send(Message::Text(json)).await.is_err() {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        email::{EmailService, SmtpConfig},
        room,
    };
    use analytics::Analytics;
    use axum::Json;
    use axum::extract::{Path, State};

    #[tokio::test]
    async fn post_run_rejects_malformed_base64() {
        let cfg = SmtpConfig::default();
        let email = Arc::new(EmailService::new(cfg.clone()).unwrap());
        let rooms = room::RoomManager::new();
        let state = Arc::new(AppState {
            email,
            rooms,
            smtp: cfg,
            analytics: Analytics::new(None, false),
            leaderboard: ::leaderboard::LeaderboardService::default(),
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
                .get_scores(leaderboard_id)
                .await
                .is_empty()
        );
    }
}
