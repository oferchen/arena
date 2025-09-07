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
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Score {
    pub id: Uuid,
    pub run_id: Uuid,
    pub player_id: Uuid,
    pub points: i32,
    pub window: LeaderboardWindow,
    pub verified: bool,
    pub created_at: DateTime<Utc>,
}

impl LeaderboardWindow {
    pub fn since(&self) -> Option<DateTime<Utc>> {
        let now = Utc::now();
        match self {
            LeaderboardWindow::Daily => Some(now - chrono::Duration::days(1)),
            LeaderboardWindow::Weekly => Some(now - chrono::Duration::weeks(1)),
            LeaderboardWindow::AllTime => None,
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

    let since = window.since();
    let rows: Vec<ScoreRow> = if let Some(since) = since {
        sqlx::query_as(
            r#"
            SELECT s.id, s.run_id, s.player_id, s.points, s.verified, s.created_at
            FROM scores s
            JOIN runs r ON s.run_id = r.id
            WHERE r.leaderboard_id = ? AND s.created_at >= ?
            ORDER BY s.points DESC
            "#,
        )
        .bind(leaderboard)
        .bind(since)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as(
            r#"
            SELECT s.id, s.run_id, s.player_id, s.points, s.verified, s.created_at
            FROM scores s
            JOIN runs r ON s.run_id = r.id
            WHERE r.leaderboard_id = ?
            ORDER BY s.points DESC
            "#,
        )
        .bind(leaderboard)
        .fetch_all(pool)
        .await?
    };

    Ok(rows
        .into_iter()
        .map(|row| Score {
            id: row.id,
            run_id: row.run_id,
            player_id: row.player_id,
            points: row.points,
            window,
            verified: row.verified != 0,
            created_at: row.created_at,
        })
        .collect())
}
