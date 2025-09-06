use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use bevy::prelude::*;
use bytes::Bytes;
use wasm_bindgen_futures::spawn_local;
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::APIBuilder;
use webrtc::data_channel::data_channel_message::DataChannelMessage;
use webrtc::data_channel::RTCDataChannel;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::RTCPeerConnection;

use crate::message::{apply_delta, InputFrame, ServerMessage, Snapshot};

static DATA_CHANNEL: Mutex<Option<Arc<RTCDataChannel>>> = Mutex::new(None);
static SNAPSHOT_QUEUE: Mutex<VecDeque<Snapshot>> = Mutex::new(VecDeque::new());
static LAST_SNAPSHOT: Mutex<Option<Snapshot>> = Mutex::new(None);
static CONNECTION_EVENTS: Mutex<VecDeque<ConnectionEvent>> =
    Mutex::new(VecDeque::new());

/// Events describing the state of the underlying connection.
#[derive(Debug, Clone, Event)]
pub enum ConnectionEvent {
    Open,
    Closed,
    Error(String),
}

/// Handles the client side of the WebRTC connection.
pub struct ClientConnector {
    pc: RTCPeerConnection,
    _dc: Arc<RTCDataChannel>,
}

impl ClientConnector {
    /// Create a new connector with a single unreliable data channel.
    pub async fn new() -> Result<Self> {
        let mut m = MediaEngine::default();
        m.register_default_codecs()?;
        let api = APIBuilder::new().with_media_engine(m).build();
        let pc = api.new_peer_connection(RTCConfiguration::default()).await?;
        let dc = pc.create_data_channel("gamedata", None).await?;
        setup_channel(&dc);
        *DATA_CHANNEL.lock().unwrap() = Some(Arc::clone(&dc));
        Ok(Self { pc, _dc: dc })
    }

    /// Close the underlying connection.
    pub async fn close(self) -> Result<()> {
        self.pc.close().await?;
        Ok(())
    }
}

fn setup_channel(dc: &Arc<RTCDataChannel>) {
    dc.on_open(Box::new(|| {
        CONNECTION_EVENTS
            .lock()
            .unwrap()
            .push_back(ConnectionEvent::Open);
        Box::pin(async {})
    }));

    dc.on_close(Box::new(|| {
        CONNECTION_EVENTS
            .lock()
            .unwrap()
            .push_back(ConnectionEvent::Closed);
        Box::pin(async {})
    }));

    dc.on_error(Box::new(|e| {
        CONNECTION_EVENTS
            .lock()
            .unwrap()
            .push_back(ConnectionEvent::Error(e.to_string()));
        Box::pin(async {})
    }));

    dc.on_message(Box::new(|msg: DataChannelMessage| {
        if !msg.is_string {
            if let Ok(msg) = postcard::from_bytes::<ServerMessage>(&msg.data) {
                match msg {
                    ServerMessage::Baseline(snapshot) => {
                        *LAST_SNAPSHOT.lock().unwrap() = Some(snapshot.clone());
                        SNAPSHOT_QUEUE.lock().unwrap().push_back(snapshot);
                    }
                    ServerMessage::Delta(delta) => {
                        let mut last = LAST_SNAPSHOT.lock().unwrap();
                        if let Some(ref base) = *last {
                            if let Ok(snap) = apply_delta(base, &delta) {
                                *last = Some(snap.clone());
                                SNAPSHOT_QUEUE.lock().unwrap().push_back(snap);
                            }
                        }
                    }
                }
            }
        }
        Box::pin(async {})
    }));
}

/// Forward queued [`InputFrame`] events to the network channel each tick.
pub fn send_input_frames(mut reader: EventReader<InputFrame>) {
    if let Some(dc) = DATA_CHANNEL.lock().unwrap().clone() {
        for frame in reader.read() {
            let bytes = match postcard::to_allocvec(frame) {
                Ok(b) => b,
                Err(_) => continue,
            };
            let dc = Arc::clone(&dc);
            spawn_local(async move {
                let _ = dc.send(&Bytes::from(bytes)).await;
            });
        }
    }
}

/// Apply incoming [`Snapshot`] messages by emitting events into the world.
pub fn apply_snapshots(mut writer: EventWriter<Snapshot>) {
    let mut queue = SNAPSHOT_QUEUE.lock().unwrap();
    while let Some(snapshot) = queue.pop_front() {
        writer.send(snapshot);
    }
}

/// Emit queued connection state changes into the world.
pub fn apply_connection_events(mut writer: EventWriter<ConnectionEvent>) {
    let mut events = CONNECTION_EVENTS.lock().unwrap();
    while let Some(ev) = events.pop_front() {
        writer.send(ev);
    }
}
