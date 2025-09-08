use bevy::prelude::*;
use bevy::input::gamepad::{Gamepad, GamepadConnection, GamepadConnectionEvent};
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::window::{CursorGrabMode, PrimaryWindow, Window};

/// Tracks whether the primary window pointer is locked.
#[derive(Resource, Default)]
pub struct PointerLock(pub bool);

/// Accumulates raw mouse motion for a single frame.
#[derive(Resource, Default)]
pub struct RawMouseDelta(pub Vec2);

/// List of currently connected gamepads.
#[derive(Resource, Default)]
pub struct ConnectedGamepads(pub Vec<Gamepad>);

/// Handles pointer lock, raw mouse input, and gamepad support.
pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(bevy::input::InputPlugin);
        app.insert_resource(PointerLock::default());
        app.insert_resource(RawMouseDelta::default());
        app.insert_resource(ConnectedGamepads::default());

        app.add_event::<MouseMotion>();
        app.add_event::<MouseWheel>();
        app.add_event::<GamepadConnectionEvent>();

        // read raw mouse motion and manage pointer locking
        app.add_systems(
            Update,
            (
                accumulate_mouse_motion,
                pointer_lock_system,
                gamepad_connection_system,
            ),
        );
    }
}

/// Collect raw mouse motion each frame.
fn accumulate_mouse_motion(
    mut events: EventReader<MouseMotion>,
    mut delta: ResMut<RawMouseDelta>,
) {
    delta.0 = Vec2::ZERO;
    for ev in events.read() {
        delta.0 += ev.delta;
    }
}

/// Handle locking/unlocking the pointer based on mouse/keyboard input.
fn pointer_lock_system(
    buttons: Res<Input<MouseButton>>,
    keys: Res<Input<KeyCode>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut lock: ResMut<PointerLock>,
) {
    let Ok(mut window) = windows.get_single_mut() else { return };

    if buttons.just_pressed(MouseButton::Left) {
        window.cursor.grab_mode = CursorGrabMode::Locked;
        window.cursor.visible = false;
        lock.0 = true;
    }

    if keys.just_pressed(KeyCode::Escape) {
        window.cursor.grab_mode = CursorGrabMode::None;
        window.cursor.visible = true;
        lock.0 = false;
    }
}

/// Track gamepad connections and disconnections.
fn gamepad_connection_system(
    mut events: EventReader<GamepadConnectionEvent>,
    mut connected: ResMut<ConnectedGamepads>,
) {
    for ev in events.read() {
        match ev.connection {
            GamepadConnection::Connected(_) => {
                if !connected.0.contains(&ev.gamepad) {
                    connected.0.push(ev.gamepad);
                }
            }
            GamepadConnection::Disconnected => {
                connected.0.retain(|g| *g != ev.gamepad);
            }
        }
    }
}

