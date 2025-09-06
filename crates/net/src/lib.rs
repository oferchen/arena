pub mod client;
pub mod message;
pub mod server;

use bevy::prelude::*;

/// Tracks the current simulation frame.
#[derive(Resource, Default)]
pub struct CurrentFrame(pub u32);

fn advance_frame(mut frame: ResMut<CurrentFrame>) {
    frame.0 = frame.0.wrapping_add(1);
}

/// Plugin exposing networking events and systems to Bevy apps.
pub struct NetPlugin;

impl Plugin for NetPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CurrentFrame::default())
            .add_event::<message::InputFrame>()
            .add_event::<message::Snapshot>()
            .add_systems(PreUpdate, advance_frame)
            .add_systems(Update, (client::send_input_frames, client::apply_snapshots));
    }
}
