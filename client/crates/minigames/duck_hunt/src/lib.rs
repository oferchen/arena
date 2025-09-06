use bevy::prelude::*;
use platform_api::{AppState, CapabilityFlags, GameModule, ModuleContext, ModuleMetadata, ServerApp};

#[derive(Default)]
pub struct DuckHuntPlugin;

impl Plugin for DuckHuntPlugin {
    fn build(&self, _app: &mut App) {}
}

#[derive(Component)]
struct DuckHuntEntity;

fn setup(world: &mut World) {
    world.spawn((Camera3dBundle::default(), DuckHuntEntity));
    let mesh_handle = {
        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        meshes.add(Mesh::from(shape::Cube { size: 1.0 }))
    };
    let material_handle = {
        let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
        materials.add(Color::rgb(0.8, 0.8, 0.3).into())
    };
    world.spawn((
        PbrBundle {
            mesh: mesh_handle,
            material: material_handle,
            ..default()
        },
        DuckHuntEntity,
    ));
}

fn cleanup(world: &mut World) {
    let entities: Vec<_> = {
        let mut q = world.query_filtered::<Entity, With<DuckHuntEntity>>();
        q.iter(world).collect()
    };
    for e in entities {
        world.entity_mut(e).despawn_recursive();
    }
}

impl GameModule for DuckHuntPlugin {
    const ID: &'static str = "duck_hunt";

    fn metadata() -> ModuleMetadata {
        ModuleMetadata {
            id: Self::ID,
            name: "Duck Hunt",
            version: "0.1.0",
            author: "Unknown",
            state: AppState::DuckHunt,
            capabilities: CapabilityFlags::LOBBY_PAD,
        }
    }

    fn register(_app: &mut App) {}

    fn enter(ctx: &mut ModuleContext) {
        setup(ctx.world());
    }

    fn exit(ctx: &mut ModuleContext) {
        cleanup(ctx.world());
    }

    fn server_register(_app: &mut ServerApp) {}
}
