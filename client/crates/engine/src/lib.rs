use bevy::prelude::*;
use platform_api::{AppState, CapabilityFlags, GameModule, ModuleMetadata};

#[derive(Resource, Default)]
pub struct ModuleRegistry {
    pub modules: Vec<ModuleMetadata>,
}

pub struct EnginePlugin;

impl Plugin for EnginePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ModuleRegistry>()
            .add_state::<AppState>()
            .add_systems(OnEnter(AppState::Lobby), setup_lobby)
            .add_systems(OnExit(AppState::Lobby), cleanup_lobby)
            .add_systems(
                Update,
                lobby_keyboard.run_if(in_state(AppState::Lobby)),
            )
            .add_systems(
                Update,
                exit_to_lobby.run_if(not(in_state(AppState::Lobby))),
            );
    }
}

#[derive(Component)]
struct LobbyEntity;

#[derive(Component)]
struct Cabinet {
    state: AppState,
    index: usize,
}

fn setup_lobby(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    registry: Res<ModuleRegistry>,
) {
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        LobbyEntity,
    ));
    commands.spawn((
        DirectionalLightBundle {
            transform: Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        LobbyEntity,
    ));
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Plane { size: 10.0, subdivisions: 0 })),
            material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
            ..default()
        },
        LobbyEntity,
    ));

    let cabinet_mesh = meshes.add(Mesh::from(shape::Cube { size: 1.0 }));
    let cabinet_material = materials.add(Color::rgb(0.8, 0.2, 0.2).into());

    for (i, info) in registry.modules.iter().enumerate() {
        if !info.capabilities.contains(CapabilityFlags::LOBBY_PAD) {
            continue;
        }
        commands.spawn((
            PbrBundle {
                mesh: cabinet_mesh.clone(),
                material: cabinet_material.clone(),
                transform: Transform::from_xyz(i as f32 * 3.0 - 3.0, 0.5, 0.0),
                ..default()
            },
            Cabinet {
                state: info.state.clone(),
                index: i,
            },
            LobbyEntity,
        ));
    }
}

fn cleanup_lobby(mut commands: Commands, q: Query<Entity, With<LobbyEntity>>) {
    for e in &q {
        commands.entity(e).despawn_recursive();
    }
}

fn lobby_keyboard(
    keys: Res<Input<KeyCode>>,
    registry: Res<ModuleRegistry>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for (i, info) in registry.modules.iter().enumerate() {
        let key = match i {
            0 => KeyCode::Key1,
            1 => KeyCode::Key2,
            2 => KeyCode::Key3,
            3 => KeyCode::Key4,
            4 => KeyCode::Key5,
            _ => continue,
        };
        if keys.just_pressed(key) {
            next_state.set(info.state.clone());
        }
    }
}

fn exit_to_lobby(keys: Res<Input<KeyCode>>, mut next_state: ResMut<NextState<AppState>>) {
    if keys.just_pressed(KeyCode::Escape) {
        next_state.set(AppState::Lobby);
    }
}

pub fn register_module<M: GameModule + Default + 'static>(app: &mut App) {
    let info = M::metadata();
    app.world
        .get_resource_mut::<ModuleRegistry>()
        .expect("EnginePlugin must be added before registering modules")
        .modules
        .push(info);
    app.add_plugins(M::default());
}

pub fn hotload_modules(_app: &mut App) {
    // Placeholder for future dynamic loading support
}

pub trait AppExt {
    fn add_game_module<M: GameModule + Default + 'static>(&mut self) -> &mut Self;
}

impl AppExt for App {
    fn add_game_module<M: GameModule + Default + 'static>(&mut self) -> &mut Self {
        register_module::<M>(self);
        self
    }
}
