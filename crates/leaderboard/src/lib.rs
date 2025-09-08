pub mod models;

use std::io;
use std::path::PathBuf;

use chrono::Utc;
use models::{LeaderboardWindow, Run, Score};
use redis::{aio::ConnectionManager, AsyncCommands};
use scylla::{Session, SessionBuilder};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use uuid::Uuid;

const WINDOWS: [LeaderboardWindow; 3] = [
    LeaderboardWindow::Daily,
    LeaderboardWindow::Weekly,
    LeaderboardWindow::AllTime,
];
#[derive(Clone)]
pub struct LeaderboardService {
    db: Session,
    cache: ConnectionManager,
    replay_dir: PathBuf,
    tx: broadcast::Sender<LeaderboardSnapshot>,
    max: usize,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LeaderboardSnapshot {
    pub leaderboard: Uuid,
    pub window: LeaderboardWindow,
    pub scores: Vec<Score>,
}

impl LeaderboardService {
    pub async fn new(database_url: &str, replay_dir: PathBuf) -> Result<Self, anyhow::Error> {
        let db = SessionBuilder::new()
            .known_node(database_url)
            .build()
            .await?;

        db.query(
            "CREATE KEYSPACE IF NOT EXISTS arena WITH replication = {'class': 'SimpleStrategy', 'replication_factor': 1}",
            &[],
        )
        .await?;
        db.use_keyspace("arena", false).await?;
        db.query(
            "CREATE TABLE IF NOT EXISTS runs (id uuid PRIMARY KEY, leaderboard_id uuid, player_id uuid, replay_path text, created_at timestamp, flagged boolean, replay_index bigint)",
            &[],
        )
        .await?;
        db.query(
            "CREATE TABLE IF NOT EXISTS scores (run_id uuid, window text, leaderboard_id uuid, player_id uuid, points int, created_at timestamp, verified boolean, PRIMARY KEY (run_id, window))",
            &[],
        )
        .await?;
        db.query(
            "CREATE TABLE IF NOT EXISTS purchases (id uuid PRIMARY KEY, user_id uuid, sku text, created_at timestamp)",
            &[],
        )
        .await?;

        let redis_url =
            std::env::var("ARENA_REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1/".into());
        let client = redis::Client::open(redis_url)?;
        let cache = ConnectionManager::new(client).await?;

        tokio::fs::create_dir_all(&replay_dir).await?;
        let (tx, _) = broadcast::channel(16);
        let max = std::env::var("ARENA_LEADERBOARD_MAX")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(100);
        Ok(Self {
            db,
            cache,
            replay_dir,
            tx,
            max,
        })
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

        self.db
            .query(
                "INSERT INTO runs (id, leaderboard_id, player_id, replay_path, created_at, flagged, replay_index) VALUES (?, ?, ?, ?, ?, ?, ?)",
                (
                    run.id,
                    leaderboard,
                    run.player_id,
                    run.replay_path.clone(),
                    run.created_at,
                    run.flagged,
                    run.replay_index,
                ),
            )
            .await
            .map_err(to_io_error)?;

        for window in WINDOWS {
            self.db
                .query(
                    "INSERT INTO scores (run_id, window, leaderboard_id, player_id, points, created_at, verified) VALUES (?, ?, ?, ?, ?, ?, ?)",
                    (
                        run.id,
                        window.as_str(),
                        leaderboard,
                        score.player_id,
                        score.points,
                        score.created_at,
                        score.verified,
                    ),
                )
                .await
                .map_err(to_io_error)?;

            let key = format!("lb:{}:{}", leaderboard, window.as_str());
            let mut conn = self.cache.clone();
            let mut s = score.clone();
            s.window = window;
            let json = serde_json::to_string(&s).unwrap();
            let _: () = conn
                .zadd(&key, json, score.points)
                .await
                .map_err(to_io_error)?;
            let _: () = conn
                .zremrangebyrank(&key, 0, -(self.max as i64) - 1)
                .await
                .map_err(to_io_error)?;
        }

        for window in WINDOWS {
            let scores = self.get_scores(leaderboard, window).await;
            let _ = self.tx.send(LeaderboardSnapshot {
                leaderboard,
                window,
                scores,
            });
        }
        Ok(())
    }

    pub async fn get_scores(
        &self,
        leaderboard: Uuid,
        window: LeaderboardWindow,
    ) -> Vec<Score> {
        let key = format!("lb:{}:{}", leaderboard, window.as_str());
        let mut conn = self.cache.clone();
        let vals: Vec<String> = conn
            .zrevrange(&key, 0, (self.max as isize) - 1)
            .await
            .unwrap_or_default();
        vals.into_iter()
            .filter_map(|v| serde_json::from_str(&v).ok())
            .collect()
    }

    pub async fn record_purchase(
        &self,
        user_id: Uuid,
        sku: &str,
    ) -> Result<Uuid, scylla::transport::errors::QueryError> {
        let id = Uuid::new_v4();
        self.db
            .query(
                "INSERT INTO purchases (id, user_id, sku, created_at) VALUES (?, ?, ?, ?)",
                (id, user_id, sku.to_string(), Utc::now()),
            )
            .await?;
        Ok(id)
    }

    pub fn subscribe(&self) -> broadcast::Receiver<LeaderboardSnapshot> {
        self.tx.subscribe()
    }

    pub async fn get_replay(&self, run_id: Uuid) -> Option<Vec<u8>> {
        if let Ok(result) = self
            .db
            .query("SELECT replay_path FROM runs WHERE id = ?", (run_id,))
            .await
        {
            if let Some(rows) = result.rows {
                if let Some(row) = rows.into_iter().next() {
                    if let Ok(rel) = row.get::<String>("replay_path") {
                        let path = self.replay_dir.join(rel);
                        return tokio::fs::read(path).await.ok();
                    }
                }
            }
        }
        None
    }

    pub async fn verify_run(&self, _run_id: Uuid) -> bool {
        // Updating verification status is left as future work.
        false
    }
}

fn to_io_error<E: std::error::Error>(e: E) -> io::Error {
    io::Error::new(io::ErrorKind::Other, e)
}

