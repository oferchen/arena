use std::sync::Arc;

use anyhow::Result;
use bevy::prelude::*;
use tokio::sync::mpsc::{self, Receiver, Sender};
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::APIBuilder;
use webrtc::data_channel::RTCDataChannel;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::RTCPeerConnection;

use crate::message::{InputFrame, Snapshot};

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
        let registry = register_default_interceptors(m, None)?;
        let api = APIBuilder::new()
            .with_media_engine(registry.media_engine)
            .with_interceptor_registry(registry.interceptor)
            .build();
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

fn setup_channels(_dc: &Arc<RTCDataChannel>) {}

/// Bevy system stub that would forward client input frames over the network.
pub fn send_input_frames(mut _reader: EventReader<InputFrame>) {}

/// Bevy system stub that applies snapshots received from the network.
pub fn apply_snapshots(mut _writer: EventWriter<Snapshot>) {}
