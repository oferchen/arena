use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

/// Movement input applied to kinematic controllers.
#[derive(Resource, Default)]
pub struct MoveInput(pub Vec3);

/// Marker component for controllable characters.
#[derive(Component)]
pub struct KinematicPlayer {
    pub speed: f32,
}

/// Kinematic character controller built on Rapier.
pub struct LocomotionPlugin;

impl Plugin for LocomotionPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(MoveInput::default());
        app.add_plugins(RapierPhysicsPlugin::<NoUserData>::default());
        app.add_systems(Update, move_character);
    }
}

fn move_character(
    input: Res<MoveInput>,
    mut query: Query<(&KinematicPlayer, &mut KinematicCharacterController)>,
) {
    for (player, mut controller) in query.iter_mut() {
        let mut translation = input.0;
        if translation.length_squared() > 0.0 {
            translation = translation.normalize() * player.speed;
        }
        controller.translation = Some(translation);
    }
}

