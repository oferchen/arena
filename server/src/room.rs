use std::sync::Arc;
use std::time::Duration as StdDuration;

use tokio::sync::{Mutex, mpsc::Receiver, mpsc::Sender, mpsc::error::TrySendError};
use tokio::time::{self, Duration};

use once_cell::sync::Lazy;
use prometheus::{IntCounter, register_int_counter};

use duck_hunt_server::{DuckState, Server as DuckServer, validate_hit, replicate, spawn_duck};
use glam::Vec3;
use net::message::{InputFrame, ServerMessage, Snapshot, delta_compress};
use net::server::ServerConnector;
use serde::{Deserialize, Serialize};
#[cfg(test)]
use std::sync::atomic::{AtomicBool, Ordering};

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
}

#[derive(Serialize, Deserialize)]
struct Shot {
    origin: [f32; 3],
    direction: [f32; 3],
    time: f32,
}

struct Room {
    connectors: Vec<ConnectorHandle>,
    last_snapshot: Option<Snapshot>,
    frame: u32,
    duck_server: DuckServer,
    scores: Vec<u32>,
}

impl Room {
    fn new() -> Self {
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
        }
    }

    fn add_connector(&mut self, connector: ServerConnector) {
        let ServerConnector { input_rx, snapshot_tx, .. } = connector;
        self.duck_server.snapshot_txs.push(snapshot_tx.clone());
        self.connectors.push(ConnectorHandle {
            input_rx,
            snapshot_tx,
            interest_mask: u64::MAX,
        });
        self.scores.push(0);
        let ducks = self.duck_server.ducks.clone();
        for duck in &ducks {
            replicate(&self.duck_server, duck);
        }
    }

    async fn tick(&mut self) {
        self.frame = self.frame.wrapping_add(1);
        // Consume all pending input frames.
        for (i, conn) in self.connectors.iter_mut().enumerate() {
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
                        for (i, (prev, curr)) in prev_scores
                            .iter()
                            .zip(&self.scores)
                            .enumerate()
                            .take(64)
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

        for conn in &self.connectors {
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
                    }
                }
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
}

#[derive(Clone)]
pub struct RoomManager {
    room: Arc<Mutex<Room>>,
}

impl RoomManager {
    pub fn new() -> Self {
        let room = Arc::new(Mutex::new(Room::new()));
        let tick_room = Arc::clone(&room);
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs_f64(1.0 / 60.0));
            loop {
                interval.tick().await;
                tick_room.lock().await.tick().await;
            }
        });
        Self { room }
    }

    pub async fn add_peer(&self, connector: ServerConnector) {
        self.room.lock().await.add_connector(connector);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_logger::{INIT, LOGGER};
    use log::LevelFilter;
    use net::message::apply_delta;
    use std::sync::atomic::Ordering;
    use tokio::sync::mpsc;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn updates_snapshot_after_delta() {
        let mut room = Room::new();

        // Attach a dummy connector so messages are sent.
        let (_input_tx, input_rx) = mpsc::channel(1);
        let (snapshot_tx, mut snapshot_rx) = mpsc::channel(8);
        room.connectors.push(ConnectorHandle {
            input_rx,
            snapshot_tx,
            interest_mask: u64::MAX,
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
        let mut room = Room::new();
        let (tx1, rx1) = mpsc::channel(1);
        let (snap_tx1, mut snap_rx1) = mpsc::channel(8);
        room.connectors.push(ConnectorHandle {
            input_rx: rx1,
            snapshot_tx: snap_tx1,
            interest_mask: u64::MAX,
        });
        let (tx2, rx2) = mpsc::channel(1);
        let (snap_tx2, mut snap_rx2) = mpsc::channel(8);
        room.connectors.push(ConnectorHandle {
            input_rx: rx2,
            snapshot_tx: snap_tx2,
            interest_mask: u64::MAX,
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
        let mut room = Room::new();

        let (_tx1, rx1) = mpsc::channel(1);
        let (snap_tx1, mut snap_rx1) = mpsc::channel(8);
        room.connectors.push(ConnectorHandle {
            input_rx: rx1,
            snapshot_tx: snap_tx1,
            interest_mask: 1,
        });
        let (_tx2, rx2) = mpsc::channel(1);
        let (snap_tx2, mut snap_rx2) = mpsc::channel(8);
        room.connectors.push(ConnectorHandle {
            input_rx: rx2,
            snapshot_tx: snap_tx2,
            interest_mask: 1 << 1,
        });
        room.scores.push(0);
        room.scores.push(0);

        room.tick().await; // baseline
        assert!(matches!(snap_rx1.try_recv().unwrap(), ServerMessage::Baseline(_)));
        assert!(matches!(snap_rx2.try_recv().unwrap(), ServerMessage::Baseline(_)));

        room.scores[0] = 1;
        room.tick().await;
        assert!(matches!(snap_rx1.try_recv().unwrap(), ServerMessage::Delta(_)));
        assert!(snap_rx2.try_recv().is_err());

        room.scores[1] = 1;
        room.tick().await;
        assert!(matches!(snap_rx2.try_recv().unwrap(), ServerMessage::Delta(_)));
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

        let mut room = Room::new();
        let (_input_tx, input_rx) = mpsc::channel(1);
        let (snapshot_tx, mut snapshot_rx) = mpsc::channel(1);
        room.connectors.push(ConnectorHandle {
            input_rx,
            snapshot_tx,
            interest_mask: u64::MAX,
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

        let mut room = Room::new();
        let (_input_tx, input_rx) = mpsc::channel(1);
        let (snapshot_tx, snapshot_rx) = mpsc::channel(1);
        room.connectors.push(ConnectorHandle {
            input_rx,
            snapshot_tx,
            interest_mask: u64::MAX,
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
        let mut room = Room::new();
        let (_input_tx, input_rx) = mpsc::channel(1);
        let (snapshot_tx, mut snapshot_rx) = mpsc::channel(8);
        room.connectors.push(ConnectorHandle {
            input_rx,
            snapshot_tx: snapshot_tx.clone(),
            interest_mask: u64::MAX,
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
}
