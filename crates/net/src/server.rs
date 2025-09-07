use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use anyhow::Result;
use bytes::Bytes;
use tokio::sync::{
    Mutex,
    mpsc::{self, Receiver, Sender},
};
use webrtc::api::APIBuilder;
use webrtc::api::media_engine::MediaEngine;
use webrtc::data_channel::RTCDataChannel;
use webrtc::data_channel::data_channel_message::DataChannelMessage;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::peer_connection::configuration::RTCConfiguration;

use crate::message::{ClientMessage, InputFrame, ServerMessage};

static DECODE_FAILURES: AtomicUsize = AtomicUsize::new(0);

/// Handles the server side of the WebRTC connection.
pub struct ServerConnector {
    /// Underlying peer connection.
    pub pc: RTCPeerConnection,
    /// Incoming input frames from the client.
    pub input_rx: Receiver<InputFrame>,
    /// Channel used to send snapshots to the client.
    pub snapshot_tx: Sender<ServerMessage>,
    /// Incoming interest mask updates from the client.
    pub interest_rx: Receiver<u64>,
}

impl ServerConnector {
    /// Create a new server connector accepting unreliable data channels.
    pub async fn new() -> Result<Self> {
        let mut m = MediaEngine::default();
        m.register_default_codecs()?;
        let api = APIBuilder::new().with_media_engine(m).build();
        let pc = api.new_peer_connection(RTCConfiguration::default()).await?;
        let (snapshot_tx, snapshot_rx) = mpsc::channel(32);
        let (input_tx, input_rx) = mpsc::channel(32);
        let (interest_tx, interest_rx) = mpsc::channel(8);

        let snapshot_rx = Arc::new(Mutex::new(snapshot_rx));
        pc.on_data_channel(Box::new(move |dc: Arc<RTCDataChannel>| {
            let input_tx = input_tx.clone();
            let interest_tx = interest_tx.clone();
            let snapshot_rx = Arc::clone(&snapshot_rx);
            Box::pin(async move {
                dc.on_message(Box::new(move |msg: DataChannelMessage| {
                    let input_tx = input_tx.clone();
                    let interest_tx = interest_tx.clone();
                    Box::pin(async move {
                        if !msg.is_string {
                            match postcard::from_bytes::<ClientMessage>(&msg.data) {
                                Ok(ClientMessage::Input(frame)) => {
                                    let _ = input_tx.send(frame).await;
                                }
                                Ok(ClientMessage::Interest(mask)) => {
                                    let _ = interest_tx.send(mask).await;
                                }
                                Err(e) => {
                                    let count = DECODE_FAILURES.fetch_add(1, Ordering::Relaxed) + 1;
                                    if count <= 5 || count % 100 == 0 {
                                        bevy::log::warn!("failed to decode client message: {e} ({count} total failures)");
                                    }
                                }
                            }
                        }
                    })
                }));

                let dc_open = Arc::clone(&dc);
                dc.on_open(Box::new(move || {
                    let dc = Arc::clone(&dc_open);
                    let snapshot_rx = Arc::clone(&snapshot_rx);
                    Box::pin(async move {
                        tokio::spawn(async move {
                            let mut rx = snapshot_rx.lock().await;
                            while let Some(msg) = rx.recv().await {
                                if let Ok(bytes) = postcard::to_allocvec(&msg) {
                                    let _ = dc.send(&Bytes::from(bytes)).await;
                                }
                            }
                        });
                    })
                }));
            })
        }));

        Ok(Self {
            pc,
            input_rx,
            snapshot_tx,
            interest_rx,
        })
    }

    /// Close the underlying connection.
    pub async fn close(self) -> Result<()> {
        self.pc.close().await?;
        Ok(())
    }
}
