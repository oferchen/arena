pub mod models;

use std::path::PathBuf;

use chrono::{Duration, Utc};
use models::{LeaderboardWindow, Run, Score};
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqlitePoolOptions, Row, SqlitePool};
use tokio::sync::broadcast;
use uuid::Uuid;

#[derive(Copy, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum LeaderboardWindow {
    Daily,
    Weekly,
    AllTime,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LeaderboardSnapshot {
    pub leaderboard: Uuid,
    pub window: LeaderboardWindow,
    pub scores: Vec<Score>,
}

#[derive(Clone)]
pub struct LeaderboardService {
    pool: SqlitePool,
    scores: Arc<Mutex<HashMap<(Uuid, LeaderboardWindow), Vec<Score>>>>,
    runs: Arc<Mutex<HashMap<Uuid, Run>>>,
    tx: broadcast::Sender<LeaderboardSnapshot>,
    replay_dir: PathBuf,
}

impl LeaderboardService {
    pub async fn new(database_url: &str, replay_dir: PathBuf) -> Result<Self, sqlx::Error> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect(database_url)
            .await?;
        let (tx, _) = broadcast::channel(16);
        Ok(Self { pool, tx, replay_dir })
    }

    async fn ensure_tables(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS runs(
                id TEXT PRIMARY KEY,
                leaderboard_id TEXT NOT NULL,
                player_id TEXT NOT NULL,
                replay_path TEXT NOT NULL,
                created_at TEXT NOT NULL,
                flagged INTEGER NOT NULL DEFAULT 0
            );
            "#,
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS scores(
                id TEXT NOT NULL,
                run_id TEXT NOT NULL,
                player_id TEXT NOT NULL,
                points INTEGER NOT NULL,
                window TEXT NOT NULL,
                FOREIGN KEY(run_id) REFERENCES runs(id) ON DELETE CASCADE,
                PRIMARY KEY (id, window)
            );
            "#,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn prune(&self) -> Result<(), sqlx::Error> {
        let now = Utc::now();
        let day_cutoff = now - Duration::days(1);
        let week_cutoff = now - Duration::weeks(1);

        sqlx::query(
            "DELETE FROM scores WHERE window = 'daily' AND run_id IN (SELECT id FROM runs WHERE created_at < ?)",
        )
        .bind(day_cutoff.to_rfc3339())
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "DELETE FROM scores WHERE window = 'weekly' AND run_id IN (SELECT id FROM runs WHERE created_at < ?)",
        )
        .bind(week_cutoff.to_rfc3339())
        .execute(&self.pool)
        .await?;
        sqlx::query("DELETE FROM runs WHERE id NOT IN (SELECT run_id FROM scores)")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn submit_score(
        &self,
        leaderboard: Uuid,
        mut score: Score,
        mut run: Run,
        replay: Vec<u8>,
    ) -> std::io::Result<()> {
        self.ensure_tables().await.map_err(to_io)?;
        tokio::fs::create_dir_all(&self.replay_dir).await?;
        let path = self.replay_dir.join(format!("{}.replay", run.id));
        tokio::fs::write(&path, replay).await?;
        run.replay_path = path.to_string_lossy().into_owned();
        run.flagged = false;
        sqlx::query(
            "INSERT INTO runs (id, leaderboard_id, player_id, replay_path, created_at, flagged) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(run.id.to_string())
        .bind(leaderboard.to_string())
        .bind(run.player_id.to_string())
        .bind(&run.replay_path)
        .bind(run.created_at.to_rfc3339())
        .bind(0)
        .execute(&self.pool)
        .await
        .map_err(to_io)?;

        for window in [
            LeaderboardWindow::Daily,
            LeaderboardWindow::Weekly,
            LeaderboardWindow::AllTime,
        ] {
            score.window = window;
            sqlx::query(
                "INSERT INTO scores (id, run_id, player_id, points, window) VALUES (?, ?, ?, ?, ?)",
            )
            .bind(score.id.to_string())
            .bind(run.id.to_string())
            .bind(score.player_id.to_string())
            .bind(score.points)
            .bind(match window {
                LeaderboardWindow::Daily => "daily",
                LeaderboardWindow::Weekly => "weekly",
                LeaderboardWindow::AllTime => "all_time",
            })
            .execute(&self.pool)
            .await
            .map_err(to_io)?;
        }

        self.prune().await.map_err(to_io)?;

        for window in [
            LeaderboardWindow::Daily,
            LeaderboardWindow::Weekly,
            LeaderboardWindow::AllTime,
        ] {
            if let Some(top_run) = sqlx::query_scalar::<_, String>(
                "SELECT runs.id FROM scores JOIN runs ON runs.id = scores.run_id WHERE runs.leaderboard_id = ? AND scores.window = ? ORDER BY scores.points DESC LIMIT 1",
            )
            .bind(leaderboard.to_string())
            .bind(match window {
                LeaderboardWindow::Daily => "daily",
                LeaderboardWindow::Weekly => "weekly",
                LeaderboardWindow::AllTime => "all_time",
            })
            .fetch_optional(&self.pool)
            .await
            .map_err(to_io)?
            {
                if top_run == run.id.to_string() {
                    sqlx::query("UPDATE runs SET flagged = 1 WHERE id = ?")
                        .bind(run.id.to_string())
                        .execute(&self.pool)
                        .await
                        .map_err(to_io)?;
                }
            }
            let scores = self.get_scores(leaderboard, window).await;
            let snapshot = LeaderboardSnapshot {
                leaderboard,
                window,
                scores: scores.clone(),
            };
            let _ = self.tx.send(snapshot);
        }

        Ok(())
    }

    pub async fn get_scores(&self, leaderboard: Uuid, window: LeaderboardWindow) -> Vec<Score> {
        self.ensure_tables().await.ok();
        let window_str = match window {
            LeaderboardWindow::Daily => "daily",
            LeaderboardWindow::Weekly => "weekly",
            LeaderboardWindow::AllTime => "all_time",
        };
        let rows = sqlx::query(
            "SELECT scores.id, scores.run_id, scores.player_id, scores.points, scores.window FROM scores JOIN runs ON runs.id = scores.run_id WHERE runs.leaderboard_id = ? AND scores.window = ? ORDER BY scores.points DESC",
        )
        .bind(leaderboard.to_string())
        .bind(window_str)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();
        rows
            .into_iter()
            .filter_map(|row| {
                let id: String = row.try_get("id").ok()?;
                let run_id: String = row.try_get("run_id").ok()?;
                let player_id: String = row.try_get("player_id").ok()?;
                let points: i32 = row.try_get("points").ok()?;
                let w: String = row.try_get("window").ok()?;
                let window = match w.as_str() {
                    "daily" => LeaderboardWindow::Daily,
                    "weekly" => LeaderboardWindow::Weekly,
                    _ => LeaderboardWindow::AllTime,
                };
                Some(Score {
                    id: Uuid::parse_str(&id).ok()?,
                    run_id: Uuid::parse_str(&run_id).ok()?,
                    player_id: Uuid::parse_str(&player_id).ok()?,
                    points,
                    window,
                })
            })
            .collect()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<LeaderboardSnapshot> {
        self.tx.subscribe()
    }

    pub async fn get_replay(&self, run_id: Uuid) -> Option<Vec<u8>> {
        self.ensure_tables().await.ok()?;
        let path: Option<String> = sqlx::query_scalar("SELECT replay_path FROM runs WHERE id = ?")
            .bind(run_id.to_string())
            .fetch_optional(&self.pool)
            .await
            .ok()?;
        let path = path?;
        tokio::fs::read(path).await.ok()
    }

    pub async fn verify_run(&self, run_id: Uuid) -> bool {
        let mut scores = self.scores.lock().await;
        for list in scores.values_mut() {
            if let Some(score) = list.iter_mut().find(|s| s.run_id == run_id) {
                score.verified = true;
                return true;
            }
        }
        false
    }

    async fn rollup_task(
        scores: Arc<Mutex<HashMap<Uuid, Vec<Score>>>>,
        runs: Arc<Mutex<HashMap<Uuid, Run>>>,
    ) {
        loop {
            let cutoff = Utc::now() - Duration::days(7);
            {
                let runs_lock = runs.lock().await;
                let mut scores_lock = scores.lock().await;
                for list in scores_lock.values_mut() {
                    list.retain(|s| {
                        runs_lock
                            .get(&s.run_id)
                            .map(|r| r.created_at > cutoff)
                            .unwrap_or(true)
                    });
                }
            }
            tokio::time::sleep(StdDuration::from_secs(3600)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn window_filters_old_scores() {
        let service = LeaderboardService::default();
        let leaderboard = Uuid::new_v4();
        let player = Uuid::new_v4();

        let old_run = Run {
            id: Uuid::new_v4(),
            leaderboard_id: leaderboard,
            player_id: player,
            replay_path: String::new(),
            created_at: Utc::now() - Duration::hours(5),
        };
        let old_score = Score {
            id: Uuid::new_v4(),
            run_id: old_run.id,
            player_id: player,
            points: 10,
            verified: false,
        };

        let new_run = Run {
            id: Uuid::new_v4(),
            leaderboard_id: leaderboard,
            player_id: player,
            replay_path: String::new(),
            created_at: Utc::now(),
        };
        let new_score = Score {
            id: Uuid::new_v4(),
            run_id: new_run.id,
            player_id: player,
            points: 20,
            verified: false,
        };

        service
            .submit_score(leaderboard, old_score, old_run, vec![])
            .await
            .unwrap();
        service
            .submit_score(leaderboard, new_score.clone(), new_run, vec![])
            .await
            .unwrap();

        let scores = service.get_scores_window(leaderboard, 2).await;
        assert_eq!(scores.len(), 1);
        assert_eq!(scores[0].points, new_score.points);
    }

    #[tokio::test]
    async fn verification_sets_flag() {
        let service = LeaderboardService::default();
        let leaderboard = Uuid::new_v4();
        let player = Uuid::new_v4();
        let run = Run {
            id: Uuid::new_v4(),
            leaderboard_id: leaderboard,
            player_id: player,
            replay_path: String::new(),
            created_at: Utc::now(),
        };
        let score = Score {
            id: Uuid::new_v4(),
            run_id: run.id,
            player_id: player,
            points: 30,
            verified: false,
        };
        service
            .submit_score(leaderboard, score.clone(), run, vec![])
            .await
            .unwrap();
        let scores = service.get_scores(leaderboard).await;
        assert!(!scores[0].verified);
        service.verify_run(score.run_id).await;
        let scores = service.get_scores(leaderboard).await;
        assert!(scores[0].verified);
    }
}

fn to_io(e: sqlx::Error) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, e)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn prunes_old_scores() {
        let svc = LeaderboardService::new(
            "sqlite::memory:",
            PathBuf::from("replays"),
        )
        .await
        .unwrap();
        let leaderboard_id = Uuid::new_v4();
        let player = Uuid::new_v4();

        let run_old = Run {
            id: Uuid::new_v4(),
            leaderboard_id,
            player_id: player,
            replay_path: String::new(),
            created_at: Utc::now() - Duration::days(2),
            flagged: false,
        };
        let score_old = Score {
            id: Uuid::new_v4(),
            run_id: run_old.id,
            player_id: player,
            points: 10,
            window: LeaderboardWindow::AllTime,
        };
        svc.submit_score(leaderboard_id, score_old, run_old, vec![])
            .await
            .unwrap();
        assert_eq!(
            svc.get_scores(leaderboard_id, LeaderboardWindow::Daily)
                .await
                .len(),
            0
        );
        assert_eq!(
            svc.get_scores(leaderboard_id, LeaderboardWindow::Weekly)
                .await
                .len(),
            1
        );
        assert_eq!(
            svc.get_scores(leaderboard_id, LeaderboardWindow::AllTime)
                .await
                .len(),
            1
        );

        let run_week_old = Run {
            id: Uuid::new_v4(),
            leaderboard_id,
            player_id: player,
            replay_path: String::new(),
            created_at: Utc::now() - Duration::weeks(2),
            flagged: false,
        };
        let score_week_old = Score {
            id: Uuid::new_v4(),
            run_id: run_week_old.id,
            player_id: player,
            points: 20,
            window: LeaderboardWindow::AllTime,
        };
        svc.submit_score(leaderboard_id, score_week_old, run_week_old, vec![])
            .await
            .unwrap();
        assert_eq!(
            svc.get_scores(leaderboard_id, LeaderboardWindow::Daily)
                .await
                .len(),
            0
        );
        assert_eq!(
            svc.get_scores(leaderboard_id, LeaderboardWindow::Weekly)
                .await
                .len(),
            1
        );
        assert_eq!(
            svc.get_scores(leaderboard_id, LeaderboardWindow::AllTime)
                .await
                .len(),
            2
        );
    }
}
