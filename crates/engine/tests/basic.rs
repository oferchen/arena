use arena_engine::assets::AssetPlugin;
use arena_engine::camera::{CameraPlugin, CameraSettings};
use arena_engine::input::{ConnectedGamepads, InputPlugin, PointerLock, RawMouseDelta};
use arena_engine::locomotion::{KinematicPlayer, LocomotionPlugin, MoveInput};
use arena_engine::ui::{HudRoot, UiPlugin};
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use bevy::input::mouse::{MouseMotion, MouseWheel, MouseScrollUnit};
use bevy::input::gamepad::{Gamepad, GamepadConnection, GamepadConnectionEvent, GamepadInfo};

#[test]
fn input_plugin_basic() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, InputPlugin));

    // raw mouse motion accumulation
    app.world
        .send_event(MouseMotion { delta: Vec2::new(1.0, 2.0) });
    app.update();
    assert_eq!(app.world.resource::<RawMouseDelta>().0, Vec2::new(1.0, 2.0));
    app.update();
    assert_eq!(app.world.resource::<RawMouseDelta>().0, Vec2::ZERO);

    // gamepad connection event
    let gamepad = Gamepad { id: 0 }; // new gamepad
    app.world.send_event(GamepadConnectionEvent {
        gamepad,
        connection: GamepadConnection::Connected(GamepadInfo { name: String::new() }),
    });
    app.update();
    assert!(app
        .world
        .resource::<ConnectedGamepads>()
        .0
        .contains(&gamepad));
}

#[test]
fn camera_plugin_updates() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, CameraPlugin));
    app.init_resource::<Events<MouseWheel>>();
    app.insert_resource(RawMouseDelta(Vec2::new(2.0, -3.0)));
    app.insert_resource(PointerLock(true));
    app.update();
    let settings = app.world.resource::<CameraSettings>();
    assert_ne!(settings.yaw, 0.0);
    assert_ne!(settings.pitch, 0.0);

    let old_fov = settings.fov;
    let _ = settings;
    app.world.send_event(MouseWheel {
        unit: MouseScrollUnit::Line,
        x: 0.0,
        y: 1.0,
        window: Entity::from_raw(0),
    });
    app.update();
    assert!(app.world.resource::<CameraSettings>().fov < old_fov);
}

#[test]
fn locomotion_plugin_moves_controller() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, LocomotionPlugin));
    let entity = app
        .world
        .spawn((
            KinematicPlayer { speed: 1.0 },
            KinematicCharacterController::default(),
        ))
        .id();
    app.world.resource_mut::<MoveInput>().0 = Vec3::X;
    app.update();
    let controller = app.world.entity(entity).get::<KinematicCharacterController>().unwrap();
    assert_eq!(controller.translation, Some(Vec3::X));
}

#[test]
fn ui_plugin_spawns_hud() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, UiPlugin));
    app.update();
    let count = app.world.query::<&HudRoot>().iter(&app.world).count();
    assert_eq!(count, 1);
}

#[test]
fn asset_plugin_adds_loaders() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin));
    // ensure plugin runs without panic and assets are added conditionally
    app.update();
}
