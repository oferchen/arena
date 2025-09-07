use bevy::prelude::*;

/// Basic yaw/pitch camera with adjustable field of view.
#[derive(Resource, Default)]
pub struct CameraSettings {
    pub yaw: f32,
    pub pitch: f32,
    pub fov: f32,
}

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, _app: &mut App) {
        // TODO: implement camera control systems
    }
}
