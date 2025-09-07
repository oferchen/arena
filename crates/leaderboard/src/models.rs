use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
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

pub async fn scores_for_window(
    pool: &sqlx::SqlitePool,
    leaderboard: Uuid,
    window: LeaderboardWindow,
) -> Result<Vec<Score>, sqlx::Error> {
    #[derive(FromRow)]
    struct ScoreRow {
        id: Uuid,
        run_id: Uuid,
        player_id: Uuid,
        points: i32,
        verified: i64,
        created_at: DateTime<Utc>,
    }
    let rows: Vec<ScoreRow> = sqlx::query_as(
        r#"
        SELECT s.id, s.run_id, s.player_id, s.points, s.verified, s.created_at
        FROM scores s
        JOIN runs r ON s.run_id = r.id
        WHERE r.leaderboard_id = ? AND s.window = ?
        ORDER BY s.points DESC
        "#,
    )
    .bind(leaderboard)
    .bind(window.as_str())
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| Score {
            id: row.id,
            run_id: row.run_id,
            player_id: row.player_id,
            points: row.points,
            verified: row.verified != 0,
            created_at: row.created_at,
            window,
        })
        .collect())
}
