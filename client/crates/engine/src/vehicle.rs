use bevy::prelude::*;

/// Simple vehicle controller that moves entities forward along the X axis.
#[derive(Component)]
pub struct VehicleController {
    /// Units to move per update step.
    pub speed: f32,
}

impl Default for VehicleController {
    fn default() -> Self {
        Self { speed: 1.0 }
    }
}

pub fn vehicle_motion(time: Res<Time>, mut query: Query<(&VehicleController, &mut Transform)>) {
    for (controller, mut transform) in &mut query {
        transform.translation.x += controller.speed * time.delta_seconds();
    }
}

/// Plugin registering vehicle controller systems.
pub struct VehiclePlugin;

impl Plugin for VehiclePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, vehicle_motion);
    }
}

