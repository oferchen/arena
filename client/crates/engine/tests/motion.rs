use bevy::prelude::*;
use engine::flight::{FlightController, FlightPlugin};
use engine::vehicle::{VehicleController, VehiclePlugin};

#[test]
fn vehicle_moves_forward() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(VehiclePlugin);
    let entity = app
        .world
        .spawn((VehicleController { speed: 2.0 }, Transform::default()))
        .id();
    app.update();
    let transform = app.world.get::<Transform>(entity).unwrap();
    assert_eq!(transform.translation.x, 2.0);
}

#[test]
fn flight_moves_up() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(FlightPlugin);
    let entity = app
        .world
        .spawn((FlightController { lift: 3.0 }, Transform::default()))
        .id();
    app.update();
    let transform = app.world.get::<Transform>(entity).unwrap();
    assert_eq!(transform.translation.y, 3.0);
}

