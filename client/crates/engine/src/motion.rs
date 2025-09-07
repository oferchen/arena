use bevy::{input::mouse::MouseMotion, prelude::*, window::CursorGrabMode};
use bevy_rapier3d::prelude::KinematicCharacterController;
use platform_api::AppState;

#[derive(Component)]
pub struct Player;

#[derive(Component)]
pub struct PlayerCamera;

#[derive(Component)]
pub struct Controller {
    pub yaw: f32,
    pub pitch: f32,
}

pub struct MotionPlugin;

impl Plugin for MotionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (player_move, player_look).run_if(in_state(AppState::Lobby)),
        );
    }
}

fn player_move(
    time: Res<Time>,
    keys: Res<Input<KeyCode>>,
    mut query: Query<(&Transform, &mut KinematicCharacterController), With<Player>>,
) {
    if let Ok((transform, mut controller)) = query.get_single_mut() {
        let mut direction = Vec3::ZERO;
        if keys.pressed(KeyCode::W) {
            direction += transform.forward();
        }
        if keys.pressed(KeyCode::S) {
            direction -= transform.forward();
        }
        if keys.pressed(KeyCode::A) {
            direction -= transform.right();
        }
        if keys.pressed(KeyCode::D) {
            direction += transform.right();
        }
        controller.translation = Some(direction.normalize_or_zero() * 5.0 * time.delta_seconds());
    }
}

fn player_look(
    mut mouse_motion: EventReader<MouseMotion>,
    mut query: Query<(&mut Controller, &mut Transform), With<Player>>,
    mut cam_query: Query<&mut Transform, With<PlayerCamera>>,
    windows: Query<&Window>,
) {
    if let Ok(window) = windows.get_single() {
        if window.cursor.grab_mode != CursorGrabMode::Locked {
            mouse_motion.clear();
            return;
        }
    }

    let Ok((mut controller, mut transform)) = query.get_single_mut() else {
        return;
    };
    let Ok(mut cam_transform) = cam_query.get_single_mut() else {
        return;
    };
    let mut delta = Vec2::ZERO;
    for ev in mouse_motion.read() {
        delta += ev.delta;
    }
    if delta == Vec2::ZERO {
        return;
    }
    controller.yaw -= delta.x * 0.002;
    controller.pitch -= delta.y * 0.002;
    controller.pitch = controller.pitch.clamp(-1.54, 1.54);
    transform.rotation = Quat::from_rotation_y(controller.yaw);
    cam_transform.rotation = Quat::from_rotation_x(controller.pitch);
}
