use anyhow::Result;
use tokio::sync::mpsc::{self, Receiver, Sender};
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::APIBuilder;
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
        let registry = register_default_interceptors(m, None)?;
        let api = APIBuilder::new()
            .with_media_engine(registry.media_engine)
            .with_interceptor_registry(registry.interceptor)
            .build();
        let pc = api.new_peer_connection(RTCConfiguration::default()).await?;
        let (_snapshot_tx, _snapshot_rx) = mpsc::channel(32);
        let (_input_tx, _input_rx) = mpsc::channel(32);
        Ok(Self {
            pc,
            _input_rx,
            _snapshot_tx,
        })
    }

    /// Close the underlying connection.
    pub async fn close(self) -> Result<()> {
        self.pc.close().await?;
        Ok(())
    }
}
