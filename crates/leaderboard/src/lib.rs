pub mod models;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use models::{LeaderboardWindow, Run, Score};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use uuid::Uuid;

#[derive(Clone)]
pub struct LeaderboardService {
    scores: Arc<Mutex<HashMap<(Uuid, LeaderboardWindow), Vec<Score>>>>,
    replays: Arc<Mutex<HashMap<Uuid, Vec<u8>>>>,
    tx: broadcast::Sender<LeaderboardSnapshot>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LeaderboardSnapshot {
    pub leaderboard: Uuid,
    pub window: LeaderboardWindow,
    pub scores: Vec<Score>,
}

impl LeaderboardService {
    pub async fn new(_database_url: &str, _replay_dir: PathBuf) -> Result<Self, sqlx::Error> {
        Ok(Self::default())
    }

    pub async fn submit_score(
        &self,
        leaderboard: Uuid,
        window: LeaderboardWindow,
        mut score: Score,
        run: Run,
        replay: Vec<u8>,
    ) -> std::io::Result<()> {
        self.replays.lock().unwrap().insert(run.id, replay);
        score.window = window;
        self.scores
            .lock()
            .unwrap()
            .entry((leaderboard, window))
            .or_default()
            .push(score);
        let scores = self.get_scores(leaderboard, window).await;
        let _ = self.tx.send(LeaderboardSnapshot {
            leaderboard,
            window,
            scores,
        });
        Ok(())
    }

    pub async fn get_scores(&self, leaderboard: Uuid, window: LeaderboardWindow) -> Vec<Score> {
        self.scores
            .lock()
            .unwrap()
            .get(&(leaderboard, window))
            .cloned()
            .unwrap_or_default()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<LeaderboardSnapshot> {
        self.tx.subscribe()
    }

    pub async fn get_replay(&self, run_id: Uuid) -> Option<Vec<u8>> {
        self.replays.lock().unwrap().get(&run_id).cloned()
    }

    pub async fn verify_run(&self, run_id: Uuid) -> bool {
        let mut scores = self.scores.lock().unwrap();
        let mut found = false;
        for list in scores.values_mut() {
            if let Some(score) = list.iter_mut().find(|s| s.run_id == run_id) {
                score.verified = true;
                found = true;
            }
        }
        found
    }
}

impl Default for LeaderboardService {
    fn default() -> Self {
        let (tx, _) = broadcast::channel(16);
        Self {
            scores: Arc::new(Mutex::new(HashMap::new())),
            replays: Arc::new(Mutex::new(HashMap::new())),
            tx,
        }
    }
}
