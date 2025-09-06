use bevy::prelude::*;
use engine::{AppState, GameModule, ModuleCapability, ModuleContext, ModuleMetadata};

#[derive(Default)]
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

impl GameModule for DuckHuntPlugin {
    const ID: &'static str = "duck_hunt";

    fn metadata() -> ModuleMetadata {
        ModuleMetadata::new("Duck Hunt", ModuleCapability::CLIENT)
    }

    fn register(_ctx: &mut ModuleContext) {}

    fn enter(_ctx: &mut ModuleContext) {}

    fn exit(_ctx: &mut ModuleContext) {}
}
