use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Information about a shard capable of hosting rooms.
#[derive(Clone, Debug)]
pub struct ShardInfo {
    pub id: String,
    pub addr: String,
    pub load: usize,
}

impl ShardInfo {
    pub fn new(id: String, addr: String, load: usize) -> Self {
        Self { id, addr, load }
    }
}

/// Registry of available shards.
pub trait ShardRegistry: Send + Sync {
    fn register(&self, shard: ShardInfo);
    fn heartbeat(&self, shard_id: &str, load: usize);
    fn least_loaded(&self) -> Option<ShardInfo>;
}

/// In-memory implementation used for testing and local runs.
#[derive(Default)]
pub struct MemoryShardRegistry {
    shards: Mutex<HashMap<String, ShardInfo>>,
}

impl MemoryShardRegistry {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ShardRegistry for MemoryShardRegistry {
    fn register(&self, shard: ShardInfo) {
        self.shards.lock().unwrap().insert(shard.id.clone(), shard);
    }

    fn heartbeat(&self, shard_id: &str, load: usize) {
        if let Some(shard) = self.shards.lock().unwrap().get_mut(shard_id) {
            shard.load = load;
        }
    }

    fn least_loaded(&self) -> Option<ShardInfo> {
        self.shards
            .lock()
            .unwrap()
            .values()
            .min_by_key(|s| s.load)
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::room;
    use std::path::PathBuf;

    #[tokio::test]
    #[ignore]
    async fn chooses_least_loaded_shard() {
        let registry = Arc::new(MemoryShardRegistry::new());
        let leaderboard =
            ::leaderboard::LeaderboardService::new("127.0.0.1:9042", PathBuf::from("replays"))
                .await
                .unwrap();
        let _s1 = room::RoomManager::with_registry(
            leaderboard.clone(),
            registry.clone(),
            "s1".into(),
            "addr1".into(),
        );
        let _s2 = room::RoomManager::with_registry(
            leaderboard.clone(),
            registry.clone(),
            "s2".into(),
            "addr2".into(),
        );
        registry.heartbeat("s1", 5);
        registry.heartbeat("s2", 1);
        let shard = registry.least_loaded().unwrap();
        assert_eq!(shard.id, "s2");
    }
}
