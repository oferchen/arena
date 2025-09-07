use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use async_trait::async_trait;
use bevy::prelude::*;
use bytes::Bytes;
use wasm_bindgen_futures::spawn_local;
use webrtc::api::APIBuilder;
use webrtc::api::media_engine::MediaEngine;
use webrtc::data_channel::RTCDataChannel;
use webrtc::data_channel::data_channel_message::DataChannelMessage;
use webrtc::data_channel::data_channel_init::RTCDataChannelInit;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::peer_connection::configuration::RTCConfiguration;

use crate::message::{ClientMessage, InputFrame, ServerMessage, Snapshot, apply_delta};

#[async_trait]
pub trait DataSender: Send + Sync {
    async fn send(&self, data: &Bytes) -> webrtc::error::Result<()>;
}

#[async_trait]
impl DataSender for RTCDataChannel {
    async fn send(&self, data: &Bytes) -> webrtc::error::Result<()> {
        RTCDataChannel::send(self, data).await.map(|_| ())
    }
}

static DATA_CHANNEL: Mutex<Option<Arc<dyn DataSender>>> = Mutex::new(None);
static SNAPSHOT_QUEUE: Mutex<VecDeque<Snapshot>> = Mutex::new(VecDeque::new());
static LAST_SNAPSHOT: Mutex<Option<Snapshot>> = Mutex::new(None);
static CONNECTION_EVENTS: Mutex<VecDeque<ConnectionEvent>> = Mutex::new(VecDeque::new());

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
        let mut cfg = RTCDataChannelInit {
            ordered: Some(false),
            max_retransmits: Some(0),
            ..Default::default()
        };
        let dc = pc
            .create_data_channel("gamedata", Some(cfg))
            .await?;
        setup_channel(&dc);
        let dc_trait: Arc<dyn DataSender> = dc.clone();
        *DATA_CHANNEL.lock().unwrap_or_else(|e| e.into_inner()) = Some(dc_trait);
        Ok(Self { pc, _dc: dc })
    }

    /// Perform signaling over a WebSocket endpoint, exchanging an SDP offer and answer.
    #[cfg(target_arch = "wasm32")]
    pub async fn signal(&self, url: &str) -> Result<()> {
        use futures_channel::oneshot;
        use wasm_bindgen::{JsCast, prelude::*};
        use web_sys::{BinaryType, MessageEvent, WebSocket};
        use webrtc::peer_connection::sdp::sdp_type::RTCSdpType;
        use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

        let offer = self.pc.create_offer(None).await?;
        self.pc.set_local_description(offer.clone()).await?;

        let ws = WebSocket::new(url)?;
        ws.set_binary_type(BinaryType::Arraybuffer);
        let (tx, rx) = oneshot::channel::<String>();

        let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
            if let Ok(text) = e.data().dyn_into::<js_sys::JsString>() {
                let _ = tx.send(text.into());
            }
        }) as Box<dyn FnMut(_)>);
        ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        onmessage.forget();

        let ws_clone = ws.clone();
        let onopen = Closure::wrap(Box::new(move || {
            let _ = ws_clone.send_with_str(&offer.sdp);
        }) as Box<dyn FnMut()>);
        ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
        onopen.forget();

        let answer_sdp = rx.await?;
        let mut answer = RTCSessionDescription::default();
        answer.sdp_type = RTCSdpType::Answer;
        answer.sdp = answer_sdp;
        self.pc.set_remote_description(answer).await?;
        Ok(())
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
            .unwrap_or_else(|e| e.into_inner())
            .push_back(ConnectionEvent::Open);
        Box::pin(async {})
    }));

    dc.on_close(Box::new(|| {
        CONNECTION_EVENTS
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push_back(ConnectionEvent::Closed);
        Box::pin(async {})
    }));

    dc.on_error(Box::new(|e| {
        CONNECTION_EVENTS
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push_back(ConnectionEvent::Error(e.to_string()));
        Box::pin(async {})
    }));

    dc.on_message(Box::new(|msg: DataChannelMessage| {
        if !msg.is_string {
            if let Ok(msg) = postcard::from_bytes::<ServerMessage>(&msg.data) {
                match msg {
                    ServerMessage::Baseline(snapshot) => {
                        *LAST_SNAPSHOT.lock().unwrap_or_else(|e| e.into_inner()) =
                            Some(snapshot.clone());
                        SNAPSHOT_QUEUE
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .push_back(snapshot);
                    }
                    ServerMessage::Delta(delta) => {
                        let mut last = LAST_SNAPSHOT.lock().unwrap_or_else(|e| e.into_inner());
                        if let Some(ref base) = *last {
                            if let Ok(snap) = apply_delta(base, &delta) {
                                *last = Some(snap.clone());
                                SNAPSHOT_QUEUE
                                    .lock()
                                    .unwrap_or_else(|e| e.into_inner())
                                    .push_back(snap);
                            }
                        }
                    }
                }
            }
        }
        Box::pin(async {})
    }));
}

async fn send_bytes(dc: Arc<dyn DataSender>, bytes: Vec<u8>) {
    if let Err(e) = dc.send(&Bytes::from(bytes)).await {
        bevy::log::error!("failed to send input frame: {e}");
    }
}

/// Forward queued [`InputFrame`] events to the network channel each tick.
pub fn send_input_frames(mut reader: EventReader<InputFrame>) {
    if let Some(dc) = DATA_CHANNEL
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
    {
        for frame in reader.read() {
            let msg = ClientMessage::Input(frame.clone());
            let bytes = match postcard::to_allocvec(&msg) {
                Ok(b) => b,
                Err(_) => continue,
            };
            let dc = Arc::clone(&dc);
            spawn_local(async move {
                send_bytes(dc, bytes).await;
            });
        }
    }
}

/// Update the server with a new interest mask describing which entities this
/// client cares about. The mask is sent over the data channel.
pub fn set_interest_mask(mask: u64) {
    if let Some(dc) = DATA_CHANNEL
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
    {
        let msg = ClientMessage::Interest(mask);
        if let Ok(bytes) = postcard::to_allocvec(&msg) {
            spawn_local(async move {
                send_bytes(dc, bytes).await;
            });
        }
    }
}

/// Apply incoming [`Snapshot`] messages by emitting events into the world.
pub fn apply_snapshots(mut writer: EventWriter<Snapshot>) {
    let mut queue = SNAPSHOT_QUEUE.lock().unwrap_or_else(|e| e.into_inner());
    while let Some(snapshot) = queue.pop_front() {
        writer.send(snapshot);
    }
}

/// Emit queued connection state changes into the world.
pub fn apply_connection_events(mut writer: EventWriter<ConnectionEvent>) {
    let mut events = CONNECTION_EVENTS.lock().unwrap_or_else(|e| e.into_inner());
    while let Some(ev) = events.pop_front() {
        writer.send(ev);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    #[test]
    fn connection_events_mutex_recover_from_poison() {
        let _ = std::panic::catch_unwind(|| {
            let _lock = CONNECTION_EVENTS.lock().unwrap();
            panic!("poisoned");
        });

        CONNECTION_EVENTS
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push_back(ConnectionEvent::Open);
    }

    struct FailingChannel;

    #[async_trait]
    impl DataSender for FailingChannel {
        async fn send(&self, _data: &Bytes) -> webrtc::error::Result<()> {
            Err(webrtc::error::Error::ErrConnectionClosed)
        }
    }

    struct TestWriter(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

    impl std::io::Write for TestWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn logs_error_when_send_fails() {
        let buf = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let writer_buf = buf.clone();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(move || TestWriter(writer_buf.clone()))
            .with_ansi(false)
            .finish();
        let _guard = tracing::subscriber::set_default(subscriber);

        let dc: Arc<dyn DataSender> = Arc::new(FailingChannel);
        super::send_bytes(dc, vec![1]).await;

        let logs = String::from_utf8(buf.lock().unwrap().clone()).unwrap();
        assert!(logs.contains("failed to send input frame"));
    }
}
