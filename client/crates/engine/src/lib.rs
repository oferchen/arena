use bevy::prelude::*;
pub use platform_api::{GameModule, ModuleCapability, ModuleContext, ModuleMetadata};

#[derive(States, Default, Clone, Eq, PartialEq, Hash, Debug)]
pub enum AppState {
    #[default]
    Lobby,
    DuckHunt,
}

pub struct GameModuleInfo {
    pub metadata: ModuleMetadata,
    pub state: AppState,
}

#[derive(Resource, Default)]
pub struct GameModuleRegistry {
    pub games: Vec<GameModuleInfo>,
}

pub struct EnginePlugin;

impl Plugin for EnginePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameModuleRegistry>()
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
    registry: Res<GameModuleRegistry>,
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

    for (i, info) in registry.games.iter().enumerate() {
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
    registry: Res<GameModuleRegistry>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for (i, info) in registry.games.iter().enumerate() {
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

pub fn register_game_module<M: GameModule + Plugin + Default + 'static>(
    app: &mut App,
    state: AppState,
) {
    let metadata = M::metadata();
    app.world
        .get_resource_mut::<GameModuleRegistry>()
        .expect("EnginePlugin must be added before registering modules")
        .games
        .push(GameModuleInfo { metadata, state });
    let mut ctx = ModuleContext::new(app);
    M::register(&mut ctx);
    app.add_plugins(M::default());
}

pub trait AppExt {
    fn add_game_module<M: GameModule + Plugin + Default + 'static>(
        &mut self,
        state: AppState,
    ) -> &mut Self;
}

impl AppExt for App {
    fn add_game_module<M: GameModule + Plugin + Default + 'static>(
        &mut self,
        state: AppState,
    ) -> &mut Self {
        register_game_module::<M>(self, state);
        self
    }
}

