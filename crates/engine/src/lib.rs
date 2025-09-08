use bevy::prelude::*;
use bevy::ecs::schedule::{Schedule, ScheduleLabel};
use bevy_rapier3d::prelude::{NoUserData, RapierPhysicsPlugin};

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Network;

pub mod input;
pub mod camera;
pub mod locomotion;
pub mod ui;
pub mod assets;

pub struct EnginePlugin;

impl Plugin for EnginePlugin {
    fn build(&self, app: &mut App) {
        // Deterministic fixed update at 60 Hz
        app.insert_resource(Time::<Fixed>::from_hz(60.0));
        app.add_schedule(Schedule::new(Network));

        // Register core plugins
        app.add_plugins((
            input::InputPlugin,
            camera::CameraPlugin,
            locomotion::LocomotionPlugin,
            ui::UiPlugin,
            assets::AssetPlugin,
        ));
    }
}

/// Hook up lobby scene graph.
#[derive(Component)]
pub struct LobbyRoot;

pub fn lobby_scene(app: &mut App) {
    // create a root entity that other systems can attach to.  This allows
    // modules to extend the lobby scene graph by querying for [`LobbyRoot`]
    // and spawning children under it.
    let root = app
        .world
        .spawn((LobbyRoot, SpatialBundle::default()))
        .id();

    // Populate some very basic geometry so the lobby is visible.  The
    // resources are optional allowing the function to be called in tests where
    // asset storages might not exist.
    let mesh = app
        .world
        .get_resource_mut::<Assets<Mesh>>()
        .map(|mut meshes| meshes.add(Mesh::from(shape::Plane { size: 10.0, subdivisions: 0 })));
    let material = app
        .world
        .get_resource_mut::<Assets<StandardMaterial>>()
        .map(|mut materials| materials.add(Color::rgb(0.3, 0.5, 0.3).into()));

    if let (Some(mesh), Some(material)) = (mesh, material) {
        app.world.entity_mut(root).with_children(|parent| {
            parent.spawn(DirectionalLightBundle::default());
            parent.spawn(PbrBundle {
                mesh,
                material,
                ..default()
            });
        });
    }
}

/// Automatically wire subsystems based on platform capabilities.
///
/// Capability flag mapping:
/// - [`CapabilityFlags::NEEDS_PHYSICS`]  &rarr; enables Rapier physics.
/// - [`CapabilityFlags::USES_HITSCAN`]  &rarr; enables the hitscan subsystem.
/// - [`CapabilityFlags::NEEDS_NAV`]     &rarr; enables navigation/path finding.
/// - [`CapabilityFlags::USES_VEHICLES`] &rarr; enables vehicle dynamics.
/// - [`CapabilityFlags::USES_FLIGHT`]   &rarr; enables flight dynamics.
#[derive(Resource, Default)]
pub struct PhysicsEnabled(pub bool);

#[derive(Resource, Default)]
pub struct HitscanEnabled(pub bool);

#[derive(Resource, Default)]
pub struct NavigationEnabled(pub bool);

#[derive(Resource, Default)]
pub struct VehiclesEnabled(pub bool);

#[derive(Resource, Default)]
pub struct FlightEnabled(pub bool);

pub fn auto_wire(app: &mut App, capabilities: platform_api::CapabilityFlags) {
    use platform_api::CapabilityFlags;

    // Physics subsystem (Rapier).
    if capabilities.contains(CapabilityFlags::NEEDS_PHYSICS) {
        // Add Rapier only once – check for an existing context before adding
        // the plugin to avoid duplicate initialization.
        if app.world.get_resource::<bevy_rapier3d::prelude::RapierContext>().is_none() {
            app.add_plugins(RapierPhysicsPlugin::<NoUserData>::default());
        }
        app.insert_resource(PhysicsEnabled(true));
    } else {
        app.insert_resource(PhysicsEnabled(false));
    }

    // The remaining subsystems are not yet fully implemented in this crate.
    // We expose boolean resources so other plugins can opt‑in when available.
    app.insert_resource(HitscanEnabled(
        capabilities.contains(CapabilityFlags::USES_HITSCAN),
    ));
    app.insert_resource(NavigationEnabled(
        capabilities.contains(CapabilityFlags::NEEDS_NAV),
    ));
    app.insert_resource(VehiclesEnabled(
        capabilities.contains(CapabilityFlags::USES_VEHICLES),
    ));
    app.insert_resource(FlightEnabled(
        capabilities.contains(CapabilityFlags::USES_FLIGHT),
    ));
}
