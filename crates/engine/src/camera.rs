use bevy::prelude::*;

use crate::input::{PointerLock, RawMouseDelta};
use bevy::input::mouse::MouseWheel;

/// Basic yaw/pitch camera with adjustable field of view.
#[derive(Resource, Default)]
pub struct CameraSettings {
    pub yaw: f32,
    pub pitch: f32,
    pub fov: f32,
}

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CameraSettings {
            yaw: 0.0,
            pitch: 0.0,
            fov: 60.0,
        });

        app.add_systems(Startup, setup_camera);
        app.add_systems(Update, (update_camera_rot, update_fov));
    }
}

fn setup_camera(mut commands: Commands, settings: Res<CameraSettings>) {
    commands.spawn(Camera3dBundle {
        projection: PerspectiveProjection {
            fov: settings.fov.to_radians(),
            ..Default::default()
        }
        .into(),
        ..Default::default()
    });
}

fn update_camera_rot(
    mut settings: ResMut<CameraSettings>,
    mut query: Query<&mut Transform, With<Camera>>,
    delta: Res<RawMouseDelta>,
    lock: Res<PointerLock>,
) {
    if !lock.0 {
        return;
    }
    let sensitivity = 0.005;
    settings.yaw -= delta.0.x * sensitivity;
    settings.pitch -= delta.0.y * sensitivity;
    settings.pitch = settings.pitch.clamp(-1.54, 1.54);

    let rot = Quat::from_axis_angle(Vec3::Y, settings.yaw)
        * Quat::from_axis_angle(Vec3::X, settings.pitch);

    for mut transform in query.iter_mut() {
        transform.rotation = rot;
    }
}

fn update_fov(
    mut settings: ResMut<CameraSettings>,
    mut query: Query<&mut Projection, With<Camera>>,
    mut wheel: EventReader<MouseWheel>,
) {
    for ev in wheel.read() {
        settings.fov = (settings.fov - ev.y).clamp(10.0, 120.0);
    }

    for mut proj in query.iter_mut() {
        if let Projection::Perspective(ref mut persp) = *proj {
            persp.fov = settings.fov.to_radians();
        }
    }
}

