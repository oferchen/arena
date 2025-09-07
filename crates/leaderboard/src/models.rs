use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
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
    pub flagged: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
#[sqlx(type_name = "TEXT")]
pub enum LeaderboardWindow {
    #[sqlx(rename = "daily")]
    Daily,
    #[sqlx(rename = "weekly")]
    Weekly,
    #[sqlx(rename = "all_time")]
    AllTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Score {
    pub id: Uuid,
    pub run_id: Uuid,
    pub player_id: Uuid,
    pub points: i32,
    pub window: LeaderboardWindow,
}
