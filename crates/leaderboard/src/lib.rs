pub mod models;

use std::{collections::HashMap, path::PathBuf, sync::Arc};

use models::{Run, Score};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, Mutex};
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
        Self {
            scores: Arc::new(Mutex::new(HashMap::new())),
            runs: Arc::new(Mutex::new(HashMap::new())),
            tx,
            replay_dir,
        }
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
        let snapshot = LeaderboardSnapshot { leaderboard, scores: list.clone() };
        drop(map);
        let _ = self.tx.send(snapshot);
        Ok(())
    }

    pub async fn get_scores(&self, leaderboard: Uuid) -> Vec<Score> {
        let map = self.scores.lock().await;
        map.get(&leaderboard).cloned().unwrap_or_default()
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
}

impl Default for LeaderboardService {
    fn default() -> Self {
        Self::new(PathBuf::from("replays"))
    }
}
