use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use physics::PhysicsPlugin;

#[test]
fn no_stray_controller_after_startup() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, PhysicsPlugin));

    // Run startup systems
    app.update();

    let count = app
        .world
        .query::<&KinematicCharacterController>()
        .iter(&app.world)
        .count();
    assert_eq!(count, 0, "unexpected KinematicCharacterController spawned");
}
