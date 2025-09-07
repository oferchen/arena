use std::sync::Arc;
use std::time::Duration as StdDuration;

use tokio::sync::{Mutex, mpsc::Receiver, mpsc::Sender, mpsc::error::TrySendError};
use tokio::time::{self, Duration};

use once_cell::sync::Lazy;
use prometheus::{IntCounter, register_int_counter};

use ::leaderboard::{
    LeaderboardService,
    models::{LeaderboardWindow, Run, Score},
};
use chrono::Utc;
use duck_hunt_server::{DuckState, Server as DuckServer, replicate, spawn_duck, validate_hit};
use glam::Vec3;
use net::message::{InputFrame, ServerMessage, Snapshot, delta_compress};
use net::server::ServerConnector;
use serde::{Deserialize, Serialize};
#[cfg(test)]
use std::sync::atomic::{AtomicBool, Ordering};
use uuid::Uuid;

#[cfg(test)]
static FORCE_SERIALIZATION_ERROR: AtomicBool = AtomicBool::new(false);

static SNAPSHOT_CHANNEL_FULL: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "snapshot_channel_full_total",
        "Number of times snapshot channel was full"
    )
    .unwrap()
});

struct ConnectorHandle {
    input_rx: Receiver<InputFrame>,
    snapshot_tx: Sender<ServerMessage>,
    /// Bitmask describing which updates this client is interested in.
    interest_mask: u64,
    /// Receives interest mask updates from the network layer.
    interest_rx: Receiver<u64>,
}

#[derive(Serialize, Deserialize)]
struct Shot {
    origin: [f32; 3],
    direction: [f32; 3],
    time: f32,
}

pub const LEADERBOARD_ID: Uuid = Uuid::from_u128(0);

struct Room {
    connectors: Vec<ConnectorHandle>,
    last_snapshot: Option<Snapshot>,
    frame: u32,
    duck_server: DuckServer,
    scores: Vec<u32>,
    player_ids: Vec<Uuid>,
    leaderboard: LeaderboardService,
    leaderboard_id: Uuid,
    start_time: std::time::Instant,
}

impl Room {
    fn new(leaderboard: LeaderboardService) -> Self {
        Self {
            connectors: Vec::new(),
            last_snapshot: None,
            frame: 0,
            duck_server: {
                let mut server = DuckServer {
                    latency: StdDuration::from_secs(0),
                    ducks: Vec::new(),
                    snapshot_txs: Vec::new(),
                };
                spawn_duck(
                    &mut server,
                    Vec3::new(0.0, 0.0, 5.0),
                    Vec3::new(1.0, 0.0, 0.0),
                );
                server
            },
            scores: Vec::new(),
            player_ids: Vec::new(),
            leaderboard,
            leaderboard_id: LEADERBOARD_ID,
            start_time: std::time::Instant::now(),
        }
    }

    fn add_connector(&mut self, connector: ServerConnector) -> usize {
        let ServerConnector {
            input_rx,
            snapshot_tx,
            interest_rx,
            ..
        } = connector;
        self.duck_server.snapshot_txs.push(snapshot_tx.clone());
        self.connectors.push(ConnectorHandle {
            input_rx,
            snapshot_tx,
            interest_mask: u64::MAX,
            interest_rx,
        });
        self.scores.push(0);
        self.player_ids.push(Uuid::new_v4());
        let ducks = self.duck_server.ducks.clone();
        for duck in &ducks {
            replicate(&self.duck_server, duck);
        }
        self.connectors.len() - 1
    }

    fn set_interest(&mut self, index: usize, mask: u64) {
        if let Some(conn) = self.connectors.get_mut(index) {
            conn.interest_mask = mask;
        }
    }

    async fn tick(&mut self) {
        self.frame = self.frame.wrapping_add(1);
        // Consume all pending input frames.
        for (i, conn) in self.connectors.iter_mut().enumerate() {
            while let Ok(mask) = conn.interest_rx.try_recv() {
                conn.interest_mask = mask;
            }
            while let Ok(frame) = conn.input_rx.try_recv() {
                if frame.frame != self.frame {
                    continue;
                }
                if let Ok(shot) = postcard::from_bytes::<Shot>(&frame.data) {
                    let origin = Vec3::from_array(shot.origin);
                    let direction = Vec3::from_array(shot.direction);
                    if validate_hit(
                        &self.duck_server,
                        origin,
                        direction,
                        StdDuration::from_secs_f32(shot.time),
                    ) {
                        if let Some(score) = self.scores.get_mut(i) {
                            *score += 1;
                        }
                    }
                }
            }
        }

        // Build a snapshot of the world containing player scores.
        #[cfg(test)]
        if FORCE_SERIALIZATION_ERROR.load(Ordering::Relaxed) {
            log::error!("failed to serialize scores snapshot: forced error");
            return;
        }
        let data = match postcard::to_allocvec(&self.scores) {
            Ok(data) => data,
            Err(err) => {
                log::error!("failed to serialize scores snapshot: {err}");
                return;
            }
        };
        let snapshot = Snapshot {
            frame: self.frame,
            data,
        };

        let mut diff_mask = 0u64;
        let msg = if let Some(ref base) = self.last_snapshot {
            match delta_compress(base, &snapshot) {
                Ok(delta) => {
                    if let Ok(prev_scores) = postcard::from_bytes::<Vec<u32>>(&base.data) {
                        for (i, (prev, curr)) in
                            prev_scores.iter().zip(&self.scores).enumerate().take(64)
                        {
                            if prev != curr {
                                diff_mask |= 1 << i;
                            }
                        }
                    } else {
                        diff_mask = (1u64 << self.scores.len().min(64)) - 1;
                    }
                    ServerMessage::Delta(delta)
                }
                Err(_) => {
                    diff_mask = (1u64 << self.scores.len().min(64)) - 1;
                    ServerMessage::Baseline(snapshot.clone())
                }
            }
        } else {
            diff_mask = (1u64 << self.scores.len().min(64)) - 1;
            ServerMessage::Baseline(snapshot.clone())
        };

        let mut closed = Vec::new();
        let diff_masks = vec![diff_mask; self.connectors.len()];
        for (i, (conn, &diff_mask)) in self.connectors.iter().zip(diff_masks.iter()).enumerate() {
            if conn.interest_mask & diff_mask == 0 {
                continue;
            }
            if let Err(err) = conn.snapshot_tx.try_send(msg.clone()) {
                match err {
                    TrySendError::Full(msg) => {
                        SNAPSHOT_CHANNEL_FULL.inc();
                        log::warn!("snapshot channel full; falling back to send");
                        let _ = conn.snapshot_tx.send(msg).await;
                    }
                    TrySendError::Closed(_) => {
                        log::warn!("snapshot channel closed");
                        closed.push(i);
                    }
                }
            }
        }

        for i in closed.into_iter().rev() {
            self.connectors.remove(i);
            if i < self.scores.len() {
                self.scores.remove(i);
            }
        }

        self.last_snapshot = Some(snapshot);

        let dt = 1.0 / 60.0;
        let len = self.duck_server.ducks.len();
        for i in 0..len {
            let state = {
                let duck = &mut self.duck_server.ducks[i];
                duck.position += duck.velocity * dt;
                duck.clone()
            };
            replicate(&self.duck_server, &state);
        }
    }

    async fn submit_scores(&mut self) {
        let leaderboard = self.leaderboard.clone();
        let leaderboard_id = self.leaderboard_id;
        let player_ids = self.player_ids.clone();
        let scores = self.scores.clone();
        for (player_id, points) in player_ids.iter().zip(scores.iter()) {
            let run_id = Uuid::new_v4();
            let score_id = Uuid::new_v4();
            let run = Run {
                id: run_id,
                leaderboard_id,
                player_id: *player_id,
                replay_path: String::new(),
                created_at: Utc::now(),
                flagged: false,
            };
            let score = Score {
                id: score_id,
                run_id,
                player_id: *player_id,
                points: *points as i32,
                window: LeaderboardWindow::AllTime,
                verified: false,
                created_at: Utc::now(),
            };
            let _ = leaderboard
                .submit_score(leaderboard_id, score, run, Vec::new())
                .await;
        }
        if !self.scores.is_empty() {
            self.scores.fill(0);
        }
    }
}

#[derive(Clone)]
pub struct RoomManager {
    room: Arc<Mutex<Room>>,
}

impl RoomManager {
    pub fn new(leaderboard: LeaderboardService) -> Self {
        let room = Arc::new(Mutex::new(Room::new(leaderboard)));
        let tick_room = Arc::clone(&room);
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs_f64(1.0 / 60.0));
            loop {
                interval.tick().await;
                tick_room.lock().await.tick().await;
            }
        });
        let round_room = Arc::clone(&room);
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(1));
            loop {
                interval.tick().await;
                round_room.lock().await.submit_scores().await;
            }
        });
        Self { room }
    }

    pub async fn add_peer(&self, connector: ServerConnector) -> usize {
        self.room.lock().await.add_connector(connector)
    }

    pub async fn set_interest(&self, index: usize, mask: u64) {
        self.room.lock().await.set_interest(index, mask);
    }
}

#[cfg(test)]
impl RoomManager {
    pub async fn push_score(&self, score: u32) {
        let mut room = self.room.lock().await;
        room.player_ids.push(Uuid::new_v4());
        room.scores.push(score);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_logger::{INIT, LOGGER};
    use log::LevelFilter;
    use net::message::apply_delta;
    use serial_test::serial;
    use std::path::PathBuf;
    use std::sync::atomic::Ordering;
    use tokio::sync::mpsc;

    async fn test_room() -> Room {
        let leaderboard =
            ::leaderboard::LeaderboardService::new("sqlite::memory:", PathBuf::from("replays"))
                .await
                .unwrap();
        Room::new(leaderboard)
    }

    #[tokio::test]
    #[serial]
    async fn updates_snapshot_after_delta() {
        let mut room = test_room().await;

        // Attach a dummy connector so messages are sent.
        let (_input_tx, input_rx) = mpsc::channel(1);
        let (_interest_tx, interest_rx) = mpsc::channel(1);
        let (snapshot_tx, mut snapshot_rx) = mpsc::channel(8);
        room.connectors.push(ConnectorHandle {
            input_rx,
            snapshot_tx,
            interest_mask: u64::MAX,
            interest_rx,
        });
        room.scores.push(0);

        // First tick sends a baseline snapshot.
        room.tick().await;
        match snapshot_rx.try_recv().expect("no baseline message") {
            ServerMessage::Baseline(s) => assert_eq!(s.frame, 1),
            other => panic!("expected baseline, got {:?}", other),
        }
        assert_eq!(room.last_snapshot.as_ref().unwrap().frame, 1);

        // Second tick sends a delta and updates the last snapshot.
        room.scores[0] = 1;
        room.tick().await;
        match snapshot_rx.try_recv().expect("no delta message") {
            ServerMessage::Delta(d) => assert_eq!(d.frame, 2),
            other => panic!("expected delta, got {:?}", other),
        }
        assert_eq!(room.last_snapshot.as_ref().unwrap().frame, 2);

        // Third tick should base its delta on the second snapshot.
        room.scores[0] = 2;
        room.tick().await;
        match snapshot_rx.try_recv().expect("no second delta message") {
            ServerMessage::Delta(d) => assert_eq!(d.frame, 3),
            other => panic!("expected delta, got {:?}", other),
        }
        assert_eq!(room.last_snapshot.as_ref().unwrap().frame, 3);
    }

    #[tokio::test]
    #[serial]
    async fn multiplayer_scoring() {
        let mut room = test_room().await;
        let (tx1, rx1) = mpsc::channel(1);
        let (_i1tx, i1rx) = mpsc::channel(1);
        let (snap_tx1, mut snap_rx1) = mpsc::channel(8);
        room.connectors.push(ConnectorHandle {
            input_rx: rx1,
            snapshot_tx: snap_tx1,
            interest_mask: u64::MAX,
            interest_rx: i1rx,
        });
        let (tx2, rx2) = mpsc::channel(1);
        let (_i2tx, i2rx) = mpsc::channel(1);
        let (snap_tx2, mut snap_rx2) = mpsc::channel(8);
        room.connectors.push(ConnectorHandle {
            input_rx: rx2,
            snapshot_tx: snap_tx2,
            interest_mask: u64::MAX,
            interest_rx: i2rx,
        });
        room.scores.push(0);
        room.scores.push(0);

        room.tick().await; // baseline
        let base1 = match snap_rx1.try_recv().unwrap() {
            ServerMessage::Baseline(b) => b,
            _ => panic!("expected baseline"),
        };
        let base2 = match snap_rx2.try_recv().unwrap() {
            ServerMessage::Baseline(b) => b,
            _ => panic!("expected baseline"),
        };

        let shot = Shot {
            origin: [0.0, 0.0, 0.0],
            direction: [0.0, 0.0, 1.0],
            time: 0.0,
        };
        let bytes = postcard::to_allocvec(&shot).unwrap();
        tx1.send(InputFrame {
            frame: room.frame + 1,
            data: bytes,
        })
        .await
        .unwrap();
        room.tick().await;
        let delta1_p1 = match snap_rx1.try_recv().unwrap() {
            ServerMessage::Delta(d) => d,
            _ => panic!("expected delta"),
        };
        let snap1_p1 = apply_delta(&base1, &delta1_p1).unwrap();
        let scores: Vec<u32> = postcard::from_bytes(&snap1_p1.data).unwrap();
        assert_eq!(scores, vec![1, 0]);
        let delta1_p2 = match snap_rx2.try_recv().unwrap() {
            ServerMessage::Delta(d) => d,
            _ => panic!("expected delta"),
        };
        let snap1_p2 = apply_delta(&base2, &delta1_p2).unwrap();

        let shot = Shot {
            origin: [0.0, 0.0, 0.0],
            direction: [0.0, 0.0, 1.0],
            time: 0.0,
        };
        let bytes = postcard::to_allocvec(&shot).unwrap();
        tx2.send(InputFrame {
            frame: room.frame + 1,
            data: bytes,
        })
        .await
        .unwrap();
        room.tick().await;
        let delta2_p1 = match snap_rx1.try_recv().unwrap() {
            ServerMessage::Delta(d) => d,
            _ => panic!("expected delta"),
        };
        let snap2_p1 = apply_delta(&snap1_p1, &delta2_p1).unwrap();
        let scores: Vec<u32> = postcard::from_bytes(&snap2_p1.data).unwrap();
        assert_eq!(scores, vec![1, 1]);
        let delta2_p2 = match snap_rx2.try_recv().unwrap() {
            ServerMessage::Delta(d) => d,
            _ => panic!("expected delta"),
        };
        let snap2_p2 = apply_delta(&snap1_p2, &delta2_p2).unwrap();
        let scores: Vec<u32> = postcard::from_bytes(&snap2_p2.data).unwrap();
        assert_eq!(scores, vec![1, 1]);
    }

    #[tokio::test]
    #[serial]
    async fn selective_updates_to_multiple_clients() {
        let mut room = test_room().await;

        let (_tx1, rx1) = mpsc::channel(1);
        let (_i1tx, i1rx) = mpsc::channel(1);
        let (snap_tx1, mut snap_rx1) = mpsc::channel(8);
        room.connectors.push(ConnectorHandle {
            input_rx: rx1,
            snapshot_tx: snap_tx1,
            interest_mask: 1,
            interest_rx: i1rx,
        });
        let (_tx2, rx2) = mpsc::channel(1);
        let (_i2tx, i2rx) = mpsc::channel(1);
        let (snap_tx2, mut snap_rx2) = mpsc::channel(8);
        room.connectors.push(ConnectorHandle {
            input_rx: rx2,
            snapshot_tx: snap_tx2,
            interest_mask: 1 << 1,
            interest_rx: i2rx,
        });
        room.scores.push(0);
        room.scores.push(0);

        room.tick().await; // baseline
        assert!(matches!(
            snap_rx1.try_recv().unwrap(),
            ServerMessage::Baseline(_)
        ));
        assert!(matches!(
            snap_rx2.try_recv().unwrap(),
            ServerMessage::Baseline(_)
        ));

        room.scores[0] = 1;
        room.tick().await;
        assert!(matches!(
            snap_rx1.try_recv().unwrap(),
            ServerMessage::Delta(_)
        ));
        assert!(snap_rx2.try_recv().is_err());

        room.scores[1] = 1;
        room.tick().await;
        assert!(matches!(
            snap_rx2.try_recv().unwrap(),
            ServerMessage::Delta(_)
        ));
        assert!(snap_rx1.try_recv().is_err());
    }

    #[tokio::test]
    #[serial]
    async fn serialization_error_logged_and_skips_snapshot() {
        INIT.call_once(|| {
            log::set_logger(&LOGGER).unwrap();
            log::set_max_level(LevelFilter::Error);
        });

        LOGGER.messages.lock().unwrap().clear();
        FORCE_SERIALIZATION_ERROR.store(true, Ordering::Relaxed);

        let mut room = test_room().await;
        let (_input_tx, input_rx) = mpsc::channel(1);
        let (_interest_tx, interest_rx) = mpsc::channel(1);
        let (snapshot_tx, mut snapshot_rx) = mpsc::channel(1);
        room.connectors.push(ConnectorHandle {
            input_rx,
            snapshot_tx,
            interest_mask: u64::MAX,
            interest_rx,
        });

        room.tick().await;

        assert!(snapshot_rx.try_recv().is_err());
        assert!(room.last_snapshot.is_none());

        let logs = LOGGER.messages.lock().unwrap();
        assert!(
            logs.iter()
                .any(|msg| msg.contains("failed to serialize scores snapshot"))
        );
        FORCE_SERIALIZATION_ERROR.store(false, Ordering::Relaxed);
    }

    #[tokio::test]
    #[serial]
    async fn logs_warning_when_channel_full() {
        INIT.call_once(|| {
            log::set_logger(&LOGGER).unwrap();
        });
        log::set_max_level(LevelFilter::Warn);

        LOGGER.messages.lock().unwrap().clear();

        let mut room = test_room().await;
        let (_input_tx, input_rx) = mpsc::channel(1);
        let (_interest_tx, interest_rx) = mpsc::channel(1);
        let (snapshot_tx, snapshot_rx) = mpsc::channel(1);
        room.connectors.push(ConnectorHandle {
            input_rx,
            snapshot_tx,
            interest_mask: u64::MAX,
            interest_rx,
        });
        room.scores.push(0);

        // First tick fills the channel with a baseline snapshot.
        room.tick().await;

        room.scores[0] = 1;

        // Drain baseline and future delta to allow fallback send to complete.
        let drain = tokio::spawn(async move {
            let mut rx = snapshot_rx;
            let _ = rx.recv().await;
            let _ = rx.recv().await;
        });

        // Second tick encounters a full channel and logs a warning.
        room.tick().await;
        drain.await.unwrap();

        let logs = LOGGER.messages.lock().unwrap();
        assert!(
            logs.iter().any(|msg| msg.contains("snapshot channel full")),
            "expected warning not found: {:?}",
            *logs
        );
    }

    #[tokio::test]
    async fn duck_spawn_and_position_updates() {
        let mut room = test_room().await;
        let (_input_tx, input_rx) = mpsc::channel(1);
        let (_interest_tx, interest_rx) = mpsc::channel(1);
        let (snapshot_tx, mut snapshot_rx) = mpsc::channel(8);
        room.connectors.push(ConnectorHandle {
            input_rx,
            snapshot_tx: snapshot_tx.clone(),
            interest_mask: u64::MAX,
            interest_rx,
        });
        room.scores.push(0);
        room.duck_server.snapshot_txs.push(snapshot_tx);

        let ducks = room.duck_server.ducks.clone();
        for duck in &ducks {
            replicate(&room.duck_server, duck);
        }

        let spawn_state = loop {
            if let ServerMessage::Baseline(b) = snapshot_rx.recv().await.unwrap() {
                if let Ok(state) = postcard::from_bytes::<DuckState>(&b.data) {
                    break state;
                }
            }
        };
        assert_eq!(spawn_state.position, Vec3::new(0.0, 0.0, 5.0));

        room.tick().await;

        let update_state = loop {
            match snapshot_rx.recv().await {
                Some(ServerMessage::Baseline(b)) => {
                    if let Ok(state) = postcard::from_bytes::<DuckState>(&b.data) {
                        break state;
                    }
                }
                Some(_) => continue,
                None => panic!("channel closed"),
            }
        };
        assert!((update_state.position.x - (1.0 / 60.0)).abs() < 1e-6);
    }

    #[tokio::test]
    async fn removes_closed_connectors() {
        let mut room = test_room().await;
        let (_input_tx, input_rx) = mpsc::channel(1);
        let (_interest_tx, interest_rx) = mpsc::channel(1);
        let (snapshot_tx, snapshot_rx) = mpsc::channel(1);
        drop(snapshot_rx);
        room.connectors.push(ConnectorHandle {
            input_rx,
            snapshot_tx,
            interest_mask: u64::MAX,
            interest_rx,
        });
        room.scores.push(0);

        room.tick().await;

        assert!(room.connectors.is_empty());
        assert!(room.scores.is_empty());
    }

    #[tokio::test]
    async fn set_interest_updates_mask() {
        let mut room = test_room().await;
        let (_input_tx, input_rx) = mpsc::channel(1);
        let (_interest_tx, interest_rx) = mpsc::channel(1);
        let (snapshot_tx, mut snapshot_rx) = mpsc::channel(8);
        room.connectors.push(ConnectorHandle {
            input_rx,
            snapshot_tx,
            interest_mask: 0,
            interest_rx,
        });
        room.scores.push(0);

        room.tick().await; // baseline exists but not sent due to zero interest
        assert!(snapshot_rx.try_recv().is_err());

        room.set_interest(0, 1);
        room.scores[0] = 1;
        room.tick().await;
        assert!(matches!(
            snapshot_rx.try_recv().unwrap(),
            ServerMessage::Delta(_)
        ));
    }
}
