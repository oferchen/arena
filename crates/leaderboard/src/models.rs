use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Leaderboard {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Player {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Run {
    pub id: Uuid,
    pub leaderboard_id: Uuid,
    pub player_id: Uuid,
    pub replay_path: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Score {
    pub id: Uuid,
    pub run_id: Uuid,
    pub player_id: Uuid,
    pub points: i32,
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LeaderboardWindow {
    pub id: Uuid,
    pub leaderboard_id: Uuid,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}
