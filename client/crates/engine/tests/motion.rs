use bevy::prelude::*;
use std::time::Duration;
use engine::flight::{FlightController, FlightPlugin};
use engine::vehicle::{VehicleController, VehiclePlugin};

#[test]
fn vehicle_moves_forward() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(VehiclePlugin);
    app.world
        .resource_mut::<Time>()
        .advance_by(Duration::from_secs_f32(0.5));
    let entity = app
        .world
        .spawn((VehicleController { speed: 2.0 }, Transform::default()))
        .id();
    app.world.run_schedule(Update);
    let transform = app.world.get::<Transform>(entity).unwrap();
    assert_eq!(transform.translation.x, 2.0 * 0.5);
}

#[test]
fn flight_moves_up() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(FlightPlugin);
    app.world
        .resource_mut::<Time>()
        .advance_by(Duration::from_secs_f32(2.0));
    let entity = app
        .world
        .spawn((FlightController { lift: 3.0 }, Transform::default()))
        .id();
    app.world.run_schedule(Update);
    let transform = app.world.get::<Transform>(entity).unwrap();
    assert_eq!(transform.translation.y, 3.0 * 2.0);
}

