use bevy::prelude::*;
use engine::{AppState, Minigame, MinigameInfo};

pub struct DuckHuntPlugin;

impl Plugin for DuckHuntPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::DuckHunt), setup)
            .add_systems(OnExit(AppState::DuckHunt), cleanup);
    }
}

#[derive(Component)]
struct DuckHuntEntity;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((Camera3dBundle::default(), DuckHuntEntity));
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: materials.add(Color::rgb(0.8, 0.8, 0.3).into()),
            ..default()
        },
        DuckHuntEntity,
    ));
}

fn cleanup(mut commands: Commands, q: Query<Entity, With<DuckHuntEntity>>) {
    for e in &q {
        commands.entity(e).despawn_recursive();
    }
}

impl Minigame for DuckHuntPlugin {
    fn info() -> MinigameInfo {
        MinigameInfo {
            name: "Duck Hunt",
            state: AppState::DuckHunt,
        }
    }
}
