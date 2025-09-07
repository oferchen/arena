use bevy::prelude::*;
use netcode::{
    CurrentFrame,
    message::{InputFrame, Snapshot},
};

/// Tracks pending inputs and last confirmed frame for client prediction.
#[derive(Resource, Default)]
pub struct PredictionState {
    /// Last snapshot frame acknowledged by the server.
    pub last_confirmed: u32,
    /// Inputs that have been sent but not yet confirmed.
    pub pending: Vec<InputFrame>,
}

/// Generate a new input frame each tick and send it to the network layer.
pub fn client_prediction(
    current: Res<CurrentFrame>,
    mut state: ResMut<PredictionState>,
    mut writer: EventWriter<InputFrame>,
) {
    if current.0 <= state.last_confirmed {
        return;
    }
    let frame = current.0;
    let input = InputFrame {
        frame,
        data: Vec::new(),
    };
    state.pending.push(input.clone());
    writer.send(input);
}

/// Reconcile client state based on snapshots from the server.
pub fn reconcile_snapshots(mut state: ResMut<PredictionState>, mut reader: EventReader<Snapshot>) {
    for snap in reader.read() {
        state.last_confirmed = snap.frame;
        state.pending.retain(|f| f.frame > snap.frame);
    }
}

/// Plugin wiring prediction and reconciliation systems.
pub struct NetClientPlugin;

impl Plugin for NetClientPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PredictionState>()
            .add_systems(Update, client_prediction)
            .add_systems(Update, reconcile_snapshots);
    }
}
