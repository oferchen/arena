use std::sync::{Arc, Mutex};

use anyhow::Result;
use bevy::prelude::*;
use tokio::sync::mpsc::{self, Receiver, Sender};
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::APIBuilder;
use webrtc::data_channel::RTCDataChannel;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::RTCPeerConnection;

use crate::message::{InputFrame, Snapshot};

// Global channels used by the client Bevy systems. These are intentionally
// simple and only meant for tests / local communication. A real implementation
// would wire these up to the data channel and handle connection state.
static INPUT_SENDER: Mutex<Option<Sender<InputFrame>>> = Mutex::new(None);
static SNAPSHOT_RECEIVER: Mutex<Option<Receiver<Snapshot>>> = Mutex::new(None);

/// Handles the client side of the WebRTC connection.
pub struct ClientConnector {
    pc: RTCPeerConnection,
    _input_tx: Sender<InputFrame>,
    _snapshot_rx: Receiver<Snapshot>,
}

impl ClientConnector {
    /// Create a new connector with a single unreliable data channel.
    pub async fn new() -> Result<Self> {
        let mut m = MediaEngine::default();
        m.register_default_codecs()?;
        let api = APIBuilder::new().with_media_engine(m).build();
        let pc = api.new_peer_connection(RTCConfiguration::default()).await?;
        let dc = pc.create_data_channel("gamedata", None).await?;
        setup_channels(&dc);
        let (_input_tx, _input_rx) = mpsc::channel(32);
        let (_snapshot_tx, _snapshot_rx) = mpsc::channel(32);
        Ok(Self {
            pc,
            _input_tx,
            _snapshot_rx,
        })
    }

    /// Close the underlying connection.
    pub async fn close(self) -> Result<()> {
        self.pc.close().await?;
        Ok(())
    }
}

fn setup_channels(_dc: &Arc<RTCDataChannel>) {
    // In lieu of a full WebRTC implementation we create local mpsc channels
    // that act as the send/receive queues for client systems.
    let (input_tx, _input_rx) = mpsc::channel(32);
    let (_snapshot_tx, snapshot_rx) = mpsc::channel(32);

    *INPUT_SENDER.lock().unwrap() = Some(input_tx);
    *SNAPSHOT_RECEIVER.lock().unwrap() = Some(snapshot_rx);
}

/// Forward queued [`InputFrame`] events to the network channel each tick.
pub fn send_input_frames(mut reader: EventReader<InputFrame>) {
    if let Some(tx) = INPUT_SENDER.lock().unwrap().clone() {
        for frame in reader.iter() {
            let _ = tx.try_send(frame.clone());
        }
    }
}

/// Apply incoming [`Snapshot`] messages by emitting events into the world.
pub fn apply_snapshots(mut writer: EventWriter<Snapshot>) {
    if let Some(rx) = SNAPSHOT_RECEIVER.lock().unwrap().as_mut() {
        while let Ok(snapshot) = rx.try_recv() {
            writer.send(snapshot);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn send_input_frames_forwards_events() {
        let (tx, mut rx) = mpsc::channel(4);
        *INPUT_SENDER.lock().unwrap() = Some(tx);

        let mut app = App::new();
        app.add_event::<InputFrame>();
        app.add_systems(Update, send_input_frames);

        app.world
            .resource_mut::<Events<InputFrame>>()
            .send(InputFrame {
                frame: 1,
                data: vec![1],
            });

        app.update();

        assert_eq!(
            rx.try_recv().unwrap(),
            InputFrame {
                frame: 1,
                data: vec![1],
            }
        );

        *INPUT_SENDER.lock().unwrap() = None;
    }

    #[test]
    fn apply_snapshots_emits_events() {
        let (snapshot_tx, snapshot_rx) = mpsc::channel(4);
        *SNAPSHOT_RECEIVER.lock().unwrap() = Some(snapshot_rx);

        let mut app = App::new();
        app.add_event::<Snapshot>();
        app.add_systems(Update, apply_snapshots);

        snapshot_tx
            .blocking_send(Snapshot {
                frame: 5,
                data: vec![9],
            })
            .unwrap();

        app.update();

        let mut reader = bevy::ecs::event::ManualEventReader::<Snapshot>::default();
        let events_res = app.world.resource::<Events<Snapshot>>();
        let events: Vec<_> = reader.iter(events_res).cloned().collect();

        assert_eq!(
            events,
            vec![Snapshot {
                frame: 5,
                data: vec![9],
            }]
        );

        *SNAPSHOT_RECEIVER.lock().unwrap() = None;
    }
}
