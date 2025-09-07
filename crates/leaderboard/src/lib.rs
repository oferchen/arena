pub mod models;

use std::io;
use std::path::PathBuf;

use models::{scores_for_window, LeaderboardWindow, Run, Score};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Row, SqlitePool};
use tokio::sync::broadcast;
use uuid::Uuid;

#[derive(Clone)]
pub struct LeaderboardService {
    pool: SqlitePool,
    replay_dir: PathBuf,
    tx: broadcast::Sender<LeaderboardSnapshot>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LeaderboardSnapshot {
    pub leaderboard: Uuid,
    pub window: LeaderboardWindow,
    pub scores: Vec<Score>,
}

impl LeaderboardService {
    pub async fn new(database_url: &str, replay_dir: PathBuf) -> Result<Self, sqlx::Error> {
        let pool = SqlitePoolOptions::new().max_connections(1).connect(database_url).await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS leaderboards (id TEXT PRIMARY KEY)"
        )
        .execute(&pool)
        .await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS runs (
                id TEXT PRIMARY KEY,
                leaderboard_id TEXT NOT NULL,
                player_id TEXT NOT NULL,
                replay_path TEXT NOT NULL,
                created_at TEXT NOT NULL,
                flagged INTEGER NOT NULL DEFAULT 0,
                replay_index INTEGER NOT NULL
            )"
        )
        .execute(&pool)
        .await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS scores (
                id TEXT PRIMARY KEY,
                run_id TEXT NOT NULL,
                player_id TEXT NOT NULL,
                points INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                verified INTEGER NOT NULL DEFAULT 0,
                window TEXT NOT NULL
            )"
        )
        .execute(&pool)
        .await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS purchases (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                sku TEXT NOT NULL,
                created_at TEXT NOT NULL
            )"
        )
        .execute(&pool)
        .await?;
        tokio::fs::create_dir_all(&replay_dir).await.map_err(|e| sqlx::Error::Io(e))?;
        let (tx, _) = broadcast::channel(16);
        Ok(Self { pool, replay_dir, tx })
    }

    pub async fn submit_score(
        &self,
        leaderboard: Uuid,
        score: Score,
        mut run: Run,
        replay: Vec<u8>,
    ) -> io::Result<()> {
        if !replay.is_empty() {
            let filename = format!("{}", run.id);
            let path = self.replay_dir.join(&filename);
            tokio::fs::write(&path, &replay).await?;
            run.replay_path = filename;
        }

        sqlx::query("INSERT OR IGNORE INTO leaderboards (id) VALUES (?)")
            .bind(leaderboard)
            .execute(&self.pool)
            .await
            .map_err(to_io_error)?;

        let idx_row = sqlx::query(
            "SELECT COALESCE(MAX(replay_index), 0) + 1 AS idx FROM runs WHERE leaderboard_id = ?",
        )
        .bind(leaderboard)
        .fetch_one(&self.pool)
        .await
        .map_err(to_io_error)?;
        let next_idx: i64 = idx_row.get("idx");
        run.replay_index = next_idx;

        sqlx::query(
            r#"INSERT OR IGNORE INTO runs (id, leaderboard_id, player_id, replay_path, created_at, flagged, replay_index)
               VALUES (?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(run.id)
        .bind(leaderboard)
        .bind(run.player_id)
        .bind(&run.replay_path)
        .bind(run.created_at)
        .bind(run.flagged as i64)
        .bind(run.replay_index)
        .execute(&self.pool)
        .await
        .map_err(to_io_error)?;

        for window in [
            LeaderboardWindow::Daily,
            LeaderboardWindow::Weekly,
            LeaderboardWindow::AllTime,
        ] {
            sqlx::query(
                r#"INSERT OR IGNORE INTO scores (id, run_id, player_id, points, created_at, verified, window)
                   VALUES (?, ?, ?, ?, ?, ?, ?)"#,
            )
            .bind(Uuid::new_v4())
            .bind(run.id)
            .bind(score.player_id)
            .bind(score.points)
            .bind(score.created_at)
            .bind(score.verified as i64)
            .bind(window.as_str())
            .execute(&self.pool)
            .await
            .map_err(to_io_error)?;
        }

        for window in [
            LeaderboardWindow::Daily,
            LeaderboardWindow::Weekly,
            LeaderboardWindow::AllTime,
        ] {
            let scores = scores_for_window(&self.pool, leaderboard, window)
                .await
                .map_err(to_io_error)?;
            let _ = self.tx.send(LeaderboardSnapshot {
                leaderboard,
                window,
                scores,
            });
        }
        Ok(())
    }

    pub async fn get_scores(&self, leaderboard: Uuid, window: LeaderboardWindow) -> Vec<Score> {
        scores_for_window(&self.pool, leaderboard, window)
            .await
            .unwrap_or_default()
    }

    pub async fn record_purchase(
        &self,
        user_id: Uuid,
        sku: &str,
    ) -> Result<Uuid, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO purchases (id, user_id, sku, created_at) VALUES (?, ?, ?, ?)"
        )
        .bind(id)
        .bind(user_id)
        .bind(sku)
        .bind(chrono::Utc::now())
        .execute(&self.pool)
        .await?;
        Ok(id)
    }

    pub fn subscribe(&self) -> broadcast::Receiver<LeaderboardSnapshot> {
        self.tx.subscribe()
    }

    pub async fn get_replay(&self, run_id: Uuid) -> Option<Vec<u8>> {
        if let Ok(Some(row)) =
            sqlx::query("SELECT replay_path FROM runs WHERE id = ?")
                .bind(run_id)
                .fetch_optional(&self.pool)
                .await
        {
            let rel: String = row.get("replay_path");
            let path = self.replay_dir.join(rel);
            tokio::fs::read(path).await.ok()
        } else {
            None
        }
    }

    pub async fn verify_run(&self, run_id: Uuid) -> bool {
        let result = sqlx::query("UPDATE scores SET verified = 1 WHERE run_id = ?")
            .bind(run_id)
            .execute(&self.pool)
            .await;
        if let Ok(res) = result {
            if res.rows_affected() > 0 {
                if let Ok(row) =
                    sqlx::query("SELECT leaderboard_id FROM runs WHERE id = ?")
                        .bind(run_id)
                        .fetch_one(&self.pool)
                        .await
                {
                    let leaderboard: Uuid = row.get("leaderboard_id");
                    for window in [
                        LeaderboardWindow::Daily,
                        LeaderboardWindow::Weekly,
                        LeaderboardWindow::AllTime,
                    ] {
                        if let Ok(scores) =
                            scores_for_window(&self.pool, leaderboard, window).await
                        {
                            let _ = self.tx.send(LeaderboardSnapshot {
                                leaderboard,
                                window,
                                scores,
                            });
                        }
                    }
                }
                return true;
            }
        }
        false
    }
}

fn to_io_error(e: sqlx::Error) -> io::Error {
    io::Error::new(io::ErrorKind::Other, e)
}
