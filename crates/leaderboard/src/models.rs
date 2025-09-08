use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum LeaderboardWindow {
    Daily,
    Weekly,
    AllTime,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Run {
    pub id: Uuid,
    pub leaderboard_id: Uuid,
    pub player_id: Uuid,
    pub replay_path: String,
    pub created_at: DateTime<Utc>,
    pub flagged: bool,
    pub replay_index: i64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Score {
    pub id: Uuid,
    pub run_id: Uuid,
    pub player_id: Uuid,
    pub points: i32,
    pub verified: bool,
    pub created_at: DateTime<Utc>,
    pub window: LeaderboardWindow,
}

impl LeaderboardWindow {
    pub fn as_str(&self) -> &'static str {
        match self {
            LeaderboardWindow::Daily => "daily",
            LeaderboardWindow::Weekly => "weekly",
            LeaderboardWindow::AllTime => "all_time",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "daily" => LeaderboardWindow::Daily,
            "weekly" => LeaderboardWindow::Weekly,
            _ => LeaderboardWindow::AllTime,
        }
    }
}

