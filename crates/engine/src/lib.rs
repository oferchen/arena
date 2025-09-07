use bevy::prelude::*;
use bevy::ecs::schedule::{Schedule, ScheduleLabel};

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Network;

pub mod input;
pub mod camera;
pub mod locomotion;
pub mod ui;
pub mod assets;

pub struct EnginePlugin;

impl Plugin for EnginePlugin {
    fn build(&self, app: &mut App) {
        // Deterministic fixed update at 60 Hz
        app.insert_resource(Time::<Fixed>::from_hz(60.0));
        app.add_schedule(Schedule::new(Network));

        // Register core plugins
        app.add_plugins((
            input::InputPlugin,
            camera::CameraPlugin,
            locomotion::LocomotionPlugin,
            ui::UiPlugin,
            assets::AssetPlugin,
        ));
    }
}

/// Hook up lobby scene graph.
pub fn lobby_scene(app: &mut App) {
    let _ = app;
    // TODO: provide lobby scene graph hooks
}

/// Automatically wire subsystems based on platform capabilities.
pub fn auto_wire(app: &mut App, capabilities: platform_api::CapabilityFlags) {
    let _ = (app, capabilities);
    // TODO: enable subsystems based on capabilities
}
