use std::sync::Arc;

use tokio::sync::{mpsc::Receiver, mpsc::Sender, Mutex};
use tokio::time::{self, Duration};

use net::message::{delta_compress, InputFrame, ServerMessage, Snapshot};
use net::server::ServerConnector;

struct ConnectorHandle {
    input_rx: Receiver<InputFrame>,
    snapshot_tx: Sender<ServerMessage>,
}

struct Room {
    connectors: Vec<ConnectorHandle>,
    last_snapshot: Option<Snapshot>,
    frame: u32,
}

impl Room {
    fn new() -> Self {
        Self { connectors: Vec::new(), last_snapshot: None, frame: 0 }
    }

    fn add_connector(&mut self, connector: ServerConnector) {
        self.connectors.push(ConnectorHandle {
            input_rx: connector.input_rx,
            snapshot_tx: connector.snapshot_tx,
        });
    }

    async fn tick(&mut self) {
        self.frame = self.frame.wrapping_add(1);
        // Consume all pending input frames.
        for conn in &mut self.connectors {
            while let Ok(_frame) = conn.input_rx.try_recv() {
                // Game state update would occur here.
            }
        }

        // Build a snapshot of the world. For now the payload is empty.
        let snapshot = Snapshot { frame: self.frame, data: Vec::new() };
        if let Some(ref base) = self.last_snapshot {
            if let Ok(delta) = delta_compress(base, &snapshot) {
                for conn in &self.connectors {
                    let _ = conn.snapshot_tx.try_send(ServerMessage::Delta(delta.clone()));
                }
            } else {
                for conn in &self.connectors {
                    let _ = conn.snapshot_tx.try_send(ServerMessage::Baseline(snapshot.clone()));
                }
                self.last_snapshot = Some(snapshot);
            }
        } else {
            for conn in &self.connectors {
                let _ = conn.snapshot_tx.try_send(ServerMessage::Baseline(snapshot.clone()));
            }
            self.last_snapshot = Some(snapshot);
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
            let mut interval = time::interval(Duration::from_millis(16));
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

