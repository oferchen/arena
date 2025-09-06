use std::sync::Arc;

use anyhow::Result;
use bytes::Bytes;
use tokio::sync::{mpsc::{self, Receiver, Sender}, Mutex};
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::APIBuilder;
use webrtc::data_channel::data_channel_message::DataChannelMessage;
use webrtc::data_channel::RTCDataChannel;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::RTCPeerConnection;

use crate::message::{InputFrame, Snapshot};

/// Handles the server side of the WebRTC connection.
pub struct ServerConnector {
    pc: RTCPeerConnection,
    _input_rx: Receiver<InputFrame>,
    _snapshot_tx: Sender<Snapshot>,
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

        let snapshot_rx = Arc::new(Mutex::new(snapshot_rx));
        pc.on_data_channel(Box::new(move |dc: Arc<RTCDataChannel>| {
            let input_tx = input_tx.clone();
            let snapshot_rx = Arc::clone(&snapshot_rx);
            Box::pin(async move {
                dc.on_message(Box::new(move |msg: DataChannelMessage| {
                    let input_tx = input_tx.clone();
                    Box::pin(async move {
                        if !msg.is_string {
                            if let Ok(frame) = postcard::from_bytes::<InputFrame>(&msg.data) {
                                let _ = input_tx.send(frame).await;
                            }
                        }
                    })
                }));

                dc.on_open(Box::new(move || {
                    let dc = Arc::clone(&dc);
                    let snapshot_rx = Arc::clone(&snapshot_rx);
                    Box::pin(async move {
                        tokio::spawn(async move {
                            let mut rx = snapshot_rx.lock().await;
                            while let Some(snapshot) = rx.recv().await {
                                if let Ok(bytes) = postcard::to_allocvec(&snapshot) {
                                    let _ = dc.send(&Bytes::from(bytes)).await;
                                }
                            }
                        });
                    })
                }));
            })
        }));

        Ok(Self { pc, _input_rx: input_rx, _snapshot_tx: snapshot_tx })
    }

    /// Close the underlying connection.
    pub async fn close(self) -> Result<()> {
        self.pc.close().await?;
        Ok(())
    }
}
