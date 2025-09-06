pub mod client;
pub mod message;
pub mod server;

use bevy::prelude::*;

/// Plugin exposing networking events and systems to Bevy apps.
pub struct NetPlugin;

impl Plugin for NetPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<message::InputFrame>()
            .add_event::<message::Snapshot>()
            .add_systems(Update, (client::send_input_frames, client::apply_snapshots));
    }
}
