pub mod models;

use std::{collections::HashMap, path::PathBuf, sync::Arc, time::Duration as StdDuration};

use chrono::{Duration, Utc};
use models::{Run, Score};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, broadcast};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize)]
pub struct LeaderboardSnapshot {
    pub leaderboard: Uuid,
    pub scores: Vec<Score>,
}

#[derive(Clone)]
pub struct LeaderboardService {
    scores: Arc<Mutex<HashMap<Uuid, Vec<Score>>>>,
    runs: Arc<Mutex<HashMap<Uuid, Run>>>,
    tx: broadcast::Sender<LeaderboardSnapshot>,
    replay_dir: PathBuf,
}

impl LeaderboardService {
    pub fn new(replay_dir: PathBuf) -> Self {
        let (tx, _) = broadcast::channel(16);
        let scores = Arc::new(Mutex::new(HashMap::new()));
        let runs = Arc::new(Mutex::new(HashMap::new()));
        let service = Self {
            scores: scores.clone(),
            runs: runs.clone(),
            tx,
            replay_dir,
        };

        tokio::spawn(Self::rollup_task(scores, runs));
        service
    }

    pub async fn submit_score(
        &self,
        leaderboard: Uuid,
        score: Score,
        mut run: Run,
        replay: Vec<u8>,
    ) -> std::io::Result<()> {
        tokio::fs::create_dir_all(&self.replay_dir).await?;
        let path = self.replay_dir.join(format!("{}.replay", run.id));
        tokio::fs::write(&path, replay).await?;
        run.replay_path = path.to_string_lossy().into_owned();
        let mut runs = self.runs.lock().await;
        runs.insert(run.id, run);
        drop(runs);

        let mut map = self.scores.lock().await;
        let list = map.entry(leaderboard).or_default();
        list.push(score.clone());
        list.sort_by(|a, b| b.points.cmp(&a.points));
        let snapshot = LeaderboardSnapshot {
            leaderboard,
            scores: list.clone(),
        };
        drop(map);
        let _ = self.tx.send(snapshot);
        Ok(())
    }

    pub async fn get_scores(&self, leaderboard: Uuid) -> Vec<Score> {
        let map = self.scores.lock().await;
        map.get(&leaderboard).cloned().unwrap_or_default()
    }

    pub async fn get_scores_window(&self, leaderboard: Uuid, hours: i64) -> Vec<Score> {
        let cutoff = Utc::now() - Duration::hours(hours);
        let runs = self.runs.lock().await;
        let mut scores = self.scores.lock().await;
        if let Some(list) = scores.get_mut(&leaderboard) {
            list.retain(|s| {
                runs.get(&s.run_id)
                    .map(|r| r.created_at > cutoff)
                    .unwrap_or(true)
            });
            list.clone()
        } else {
            Vec::new()
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<LeaderboardSnapshot> {
        self.tx.subscribe()
    }

    pub async fn get_replay(&self, run_id: Uuid) -> Option<Vec<u8>> {
        let runs = self.runs.lock().await;
        let path = runs.get(&run_id)?.replay_path.clone();
        drop(runs);
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

impl Default for LeaderboardService {
    fn default() -> Self {
        Self::new(PathBuf::from("replays"))
    }
}
