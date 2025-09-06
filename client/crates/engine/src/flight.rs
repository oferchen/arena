use bevy::prelude::*;

/// Simple flight controller that moves entities upward along the Y axis.
#[derive(Component)]
pub struct FlightController {
    /// Units to move per update step.
    pub lift: f32,
}

impl Default for FlightController {
    fn default() -> Self {
        Self { lift: 1.0 }
    }
}

pub fn flight_motion(mut query: Query<(&FlightController, &mut Transform)>) {
    for (controller, mut transform) in &mut query {
        transform.translation.y += controller.lift;
    }
}

/// Plugin registering flight controller systems.
pub struct FlightPlugin;

impl Plugin for FlightPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, flight_motion);
    }
}

