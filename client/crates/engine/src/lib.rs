use anyhow::Error as AnyError;
use bevy::ecs::schedule::common_conditions::resource_changed;
#[cfg(target_arch = "wasm32")]
use bevy::tasks::{AsyncComputeTaskPool, Task};
use bevy::{prelude::*, window::CursorGrabMode};
use bevy_rapier3d::prelude::*;
#[cfg(target_arch = "wasm32")]
use futures_lite::future;
#[cfg(target_arch = "wasm32")]
use gloo_timers::future::TimeoutFuture;
#[cfg(not(target_arch = "wasm32"))]
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use platform_api::{
    AppState, CapabilityFlags, GameModule, ModuleContext, ModuleMetadata, discover_local_modules,
};
#[cfg(target_arch = "wasm32")]
use platform_api::ModuleManifest;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc::Receiver;
use thiserror::Error;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;

pub mod core;
#[cfg(feature = "flight")]
pub mod flight;
pub mod motion;
pub mod net;
#[cfg(feature = "vehicle")]
pub mod vehicle;

use crate::net::NetClientPlugin;
use core::CorePlugin;
#[cfg(feature = "flight")]
use flight::FlightPlugin;
use motion::{Controller, MotionPlugin, Player, PlayerCamera};
use netcode::NetPlugin as NetworkPlugin;
#[cfg(feature = "vehicle")]
use vehicle::VehiclePlugin;

/// Numeric hotkeys usable in the lobby to select modules.
const LOBBY_KEYS: [KeyCode; 9] = [
    KeyCode::Key1,
    KeyCode::Key2,
    KeyCode::Key3,
    KeyCode::Key4,
    KeyCode::Key5,
    KeyCode::Key6,
    KeyCode::Key7,
    KeyCode::Key8,
    KeyCode::Key9,
];

/// Stores metadata for all registered game modules.
#[derive(Resource, Default)]
pub struct ModuleRegistry {
    /// Ordered collection of discovered modules.
    pub modules: Vec<ModuleMetadata>,
}

/// Stores the interpolation factor between fixed simulation steps for smooth rendering.
#[derive(Resource, Default)]
pub struct FrameInterpolation(pub f32);

#[derive(Debug, Error)]
enum EngineError {
    #[error("module enter failed: {0}")]
    ModuleEnter(AnyError),
    #[error("module exit failed: {0}")]
    ModuleExit(AnyError),
    #[cfg(not(target_arch = "wasm32"))]
    #[error("watcher error: {0}")]
    Watcher(#[from] notify::Error),
}

pub struct EnginePlugin;

impl Plugin for EnginePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(NetworkPlugin)
            .add_plugins(CorePlugin)
            .add_plugins(MotionPlugin)
            .init_resource::<ModuleRegistry>()
            .init_resource::<FrameInterpolation>()
            .add_state::<AppState>()
            .add_systems(Startup, discover_modules)
            .add_systems(OnEnter(AppState::Lobby), setup_lobby)
            .add_systems(OnExit(AppState::Lobby), cleanup_lobby)
            .add_systems(Update, lobby_keyboard.run_if(in_state(AppState::Lobby)))
            .add_systems(FixedUpdate, pad_trigger.run_if(in_state(AppState::Lobby)))
            .add_systems(Update, doc_button_system.run_if(in_state(AppState::Lobby)))
            .add_systems(Update, exit_to_lobby)
            .add_systems(Update, update_frame_interpolation);

        #[cfg(target_arch = "wasm32")]
        app.add_systems(Update, apply_discovered_modules);

        hotload_modules(app);

        app.add_systems(
            Update,
            update_lobby_pads
                .run_if(resource_changed::<ModuleRegistry>())
                .run_if(in_state(AppState::Lobby)),
        );

        #[cfg(feature = "vehicle")]
        app.add_plugins(VehiclePlugin);
        #[cfg(feature = "flight")]
        app.add_plugins(FlightPlugin);

        app.add_plugins(NetClientPlugin);
    }
}

#[derive(Component)]
struct LobbyEntity;

#[derive(Component)]
pub struct LobbyPad {
    pub state: AppState,
}

#[derive(Component)]
pub struct DocPad {
    pub url: &'static str,
}

#[derive(Component)]
pub struct DocButton {
    pub url: &'static str,
}

#[derive(Component)]
pub struct NoModulesSign;

#[derive(Component)]
pub struct LeaderboardScreen;

#[derive(Component)]
pub struct ReplayPedestal;

#[derive(Component)]
pub struct StorePanel;

const HELP_DOCS: [(&str, &str); 5] = [
    ("Netcode", "docs/netcode.md"),
    ("Modules", "docs/modules.md"),
    ("Duck Hunt", "docs/DuckHunt.md"),
    ("Ops", "docs/ops.md"),
    ("Email", "docs/Email.md"),
];

pub fn setup_lobby(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    registry: Res<ModuleRegistry>,
    asset_server: Option<Res<AssetServer>>,
    mut windows: Query<&mut Window>,
) {
    if let Ok(mut window) = windows.get_single_mut() {
        window.cursor.grab_mode = CursorGrabMode::Locked;
        window.cursor.visible = false;
    } else {
        warn!("no window available");
        return;
    }

    commands
        .spawn((
            TransformBundle::from_transform(Transform::from_xyz(0.0, 1.5, 5.0)),
            RigidBody::KinematicPositionBased,
            Collider::capsule_y(0.5, 0.3),
            KinematicCharacterController::default(),
            Controller {
                yaw: 0.0,
                pitch: 0.0,
            },
            Player,
            LobbyEntity,
        ))
        .with_children(|parent| {
            parent.spawn((Camera3dBundle::default(), PlayerCamera));
        });
    commands.spawn((
        DirectionalLightBundle {
            transform: Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        LobbyEntity,
    ));
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Plane {
                size: 10.0,
                subdivisions: 0,
            })),
            material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
            ..default()
        },
        LobbyEntity,
    ));

    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Plane { size: 2.0, subdivisions: 0 })),
            material: materials.add(Color::rgb(0.2, 0.2, 0.8).into()),
            transform: Transform::from_xyz(-4.0, 1.0, -2.0)
                .looking_at(Vec3::new(-4.0, 1.0, -1.0), Vec3::Y),
            ..default()
        },
        LeaderboardScreen,
        LobbyEntity,
    ));

    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: materials.add(Color::rgb(0.8, 0.8, 0.2).into()),
            transform: Transform::from_xyz(4.0, 0.5, -2.0),
            ..default()
        },
        ReplayPedestal,
        LobbyEntity,
    ));

    let pad_mesh = meshes.add(Mesh::from(shape::Cube { size: 1.0 }));
    let pad_material = materials.add(Color::rgb(0.8, 0.2, 0.2).into());
    let font = asset_server
        .as_ref()
        .map(|s| s.load("fonts/FiraSans-Bold.ttf"))
        .unwrap_or_default();

    // Basic store panel showcasing purchasable items.
    commands
        .spawn((
            PbrBundle {
                mesh: pad_mesh.clone(),
                material: pad_material.clone(),
                transform: Transform::from_xyz(0.0, 0.5, -2.5),
                ..default()
            },
            Collider::cuboid(0.5, 0.5, 0.5),
            Sensor,
            ActiveEvents::COLLISION_EVENTS,
            StorePanel,
            LobbyEntity,
        ))
        .with_children(|parent| {
            parent.spawn(Text2dBundle {
                text: Text::from_section(
                    "Store",
                    TextStyle {
                        font: font.clone(),
                        font_size: 20.0,
                        color: Color::WHITE,
                    },
                ),
                transform: Transform::from_xyz(0.0, 0.75, 0.0),
                ..default()
            });
        });

    commands
        .spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    top: Val::Px(8.0),
                    left: Val::Px(8.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(4.0),
                    ..default()
                },
                background_color: BackgroundColor(Color::rgba(0.0, 0.0, 0.0, 0.5)),
                ..default()
            },
            LobbyEntity,
        ))
        .with_children(|parent| {
            for &(label, url) in HELP_DOCS.iter() {
                parent
                    .spawn((
                        ButtonBundle {
                            style: Style {
                                padding: UiRect::all(Val::Px(4.0)),
                                ..default()
                            },
                            background_color: BackgroundColor(Color::rgb(0.15, 0.15, 0.15)),
                            ..default()
                        },
                        DocButton { url },
                    ))
                    .with_children(|button| {
                        button.spawn(TextBundle::from_section(
                            label,
                            TextStyle {
                                font: font.clone(),
                                font_size: 16.0,
                                color: Color::WHITE,
                            },
                        ));
                    });
            }
        });

    if registry.modules.is_empty() {
        for (i, &(label, url)) in HELP_DOCS.iter().enumerate() {
            commands
                .spawn((
                    PbrBundle {
                        mesh: pad_mesh.clone(),
                        material: pad_material.clone(),
                        transform: Transform::from_xyz(i as f32 * 3.0 - 6.0, 0.5, 0.0),
                        ..default()
                    },
                    Collider::cuboid(0.5, 0.5, 0.5),
                    Sensor,
                    ActiveEvents::COLLISION_EVENTS,
                    DocPad { url },
                    LobbyEntity,
                ))
                .with_children(|parent| {
                    parent.spawn(Text2dBundle {
                        text: Text::from_section(
                            label,
                            TextStyle {
                                font: font.clone(),
                                font_size: 20.0,
                                color: Color::WHITE,
                            },
                        ),
                        transform: Transform::from_xyz(0.0, 0.75, 0.0),
                        ..default()
                    });
                });
        }

        commands.spawn((
            Text2dBundle {
                text: Text::from_section(
                    "No modules installed",
                    TextStyle {
                        font: font.clone(),
                        font_size: 40.0,
                        color: Color::WHITE,
                    },
                ),
                transform: Transform::from_xyz(0.0, 2.5, 3.0)
                    .looking_at(Vec3::new(0.0, 1.5, 5.0), Vec3::Y),
                ..default()
            },
            NoModulesSign,
            LobbyEntity,
        ));
    } else {
        for (i, info) in registry.modules.iter().enumerate() {
            if !info.capabilities.contains(CapabilityFlags::LOBBY_PAD) {
                continue;
            }
            commands
                .spawn((
                    PbrBundle {
                        mesh: pad_mesh.clone(),
                        material: pad_material.clone(),
                        transform: Transform::from_xyz(i as f32 * 3.0 - 3.0, 0.5, 0.0),
                        ..default()
                    },
                    Collider::cuboid(0.5, 0.5, 0.5),
                    Sensor,
                    ActiveEvents::COLLISION_EVENTS,
                    LobbyPad {
                        state: info.state.clone(),
                    },
                    LobbyEntity,
                ))
                .with_children(|parent| {
                    let label = if LOBBY_KEYS.get(i).is_some() {
                        format!(
                            "[{}] {} v{}\nPlayers: {}\nPing: 0ms",
                            i + 1,
                            info.name,
                            info.version,
                            info.max_players
                        )
                    } else {
                        format!(
                            "{} v{}\nPlayers: {}\nPing: 0ms",
                            info.name,
                            info.version,
                            info.max_players
                        )
                    };
                    parent.spawn(Text2dBundle {
                        text: Text::from_section(
                            label,
                            TextStyle {
                                font: font.clone(),
                                font_size: 20.0,
                                color: Color::WHITE,
                            },
                        ),
                        transform: Transform::from_xyz(0.0, 0.75, 0.0),
                        ..default()
                    });
                });
        }
    }
}

fn cleanup_lobby(mut commands: Commands, q: Query<Entity, With<LobbyEntity>>) {
    for e in &q {
        commands.entity(e).despawn_recursive();
    }
}

pub fn lobby_keyboard(
    keys: Res<Input<KeyCode>>,
    registry: Res<ModuleRegistry>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for (i, info) in registry.modules.iter().enumerate() {
        if let Some(&key) = LOBBY_KEYS.get(i) {
            if keys.just_pressed(key) {
                next_state.set(info.state.clone());
            }
        }
    }
}

fn exit_to_lobby(
    keys: Res<Input<KeyCode>>,
    mut next_state: ResMut<NextState<AppState>>,
    mut windows: Query<&mut Window>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        if let Ok(mut window) = windows.get_single_mut() {
            let locked = window.cursor.grab_mode == CursorGrabMode::Locked;
            if locked {
                window.cursor.grab_mode = CursorGrabMode::None;
                window.cursor.visible = true;
            } else {
                window.cursor.grab_mode = CursorGrabMode::Locked;
                window.cursor.visible = false;
            }
            next_state.set(AppState::Lobby);
        } else {
            warn!("no window available");
            return;
        }
    }
}

fn pad_trigger(
    mut collisions: EventReader<CollisionEvent>,
    player: Query<Entity, With<Player>>,
    pads: Query<&LobbyPad>,
    docs: Query<&DocPad>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    let Ok(player_entity) = player.get_single() else {
        return;
    };
    for ev in collisions.read() {
        if let CollisionEvent::Started(e1, e2, _) = ev {
            let other = if *e1 == player_entity {
                *e2
            } else if *e2 == player_entity {
                *e1
            } else {
                continue;
            };
            if let Ok(pad) = pads.get(other) {
                // Changing the app state triggers the module's `GameModule::enter` hook
                // via the associated state transition.
                next_state.set(pad.state.clone());
            } else if let Ok(doc) = docs.get(other) {
                #[cfg(target_arch = "wasm32")]
                {
                    if let Some(window) = web_sys::window() {
                        let _ = window.open_with_url(doc.url);
                    }
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let _ = doc;
                }
            }
        }
    }
}

fn doc_button_system(
    mut interactions: Query<(&Interaction, &DocButton), (Changed<Interaction>, With<Button>)>,
) {
    for (interaction, doc) in &mut interactions {
        if *interaction == Interaction::Pressed {
            #[cfg(target_arch = "wasm32")]
            {
                if let Some(window) = web_sys::window() {
                    let _ = window.open_with_url(doc.url);
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = doc;
            }
        }
    }
}

fn update_frame_interpolation(
    fixed_time: Res<Time<Fixed>>,
    mut interpolation: ResMut<FrameInterpolation>,
) {
    interpolation.0 = fixed_time.overstep_percentage();
}

/// Registers a [`GameModule`] and wires its lifecycle hooks.
pub fn register_module<M: GameModule + Default + 'static>(app: &mut App) {
    let info = M::metadata();
    let state = info.state.clone();
    M::register(app);
    if let Some(mut registry) = app.world.get_resource_mut::<ModuleRegistry>() {
        registry.modules.push(info);
    } else {
        warn!("EnginePlugin must be added before registering modules");
    }
    app.add_plugins(M::default());
    app.add_systems(OnEnter(state.clone()), enter_module::<M>);
    app.add_systems(OnExit(state), exit_module::<M>);
}

/// System wrapper that forwards state entry to the module.
fn enter_module<M: GameModule>(world: &mut World) {
    let mut ctx = ModuleContext::new(world);
    if let Err(e) = M::enter(&mut ctx) {
        log::error!("{}", EngineError::ModuleEnter(e));
    }
}

/// System wrapper that forwards state exit to the module.
fn exit_module<M: GameModule>(world: &mut World) {
    let mut ctx = ModuleContext::new(world);
    if let Err(e) = M::exit(&mut ctx) {
        log::error!("{}", EngineError::ModuleExit(e));
    }
}

#[derive(Deserialize)]
#[cfg(target_arch = "wasm32")]
#[derive(Resource)]
struct ModuleDiscoveryTask(Task<Vec<ModuleMetadata>>);

#[cfg(target_arch = "wasm32")]
#[derive(Resource)]
struct ModuleDiscoveryLoop(Task<()>);

pub fn discover_modules(
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))] mut commands: Commands,
    mut registry: ResMut<ModuleRegistry>,
    asset_server: Option<Res<AssetServer>>,
) {
    #[cfg(target_arch = "wasm32")]
    {
        use bevy::asset::load;
        let Some(asset_server) = asset_server else {
            return;
        };
        let asset_server = asset_server.clone();
        let task = AsyncComputeTaskPool::get().spawn(async move {
            let data: String = load(asset_server.as_ref(), "modules.json").await;
            match serde_json::from_str::<Vec<ModuleManifest>>(&data) {
                Ok(manifests) => manifests
                    .into_iter()
                    .filter_map(|manifest| {
                        let state = match manifest.state.as_str() {
                            "Lobby" => AppState::Lobby,
                            "DuckHunt" => AppState::DuckHunt,
                            other => {
                                log::error!(
                                    "unknown module state '{}' for module '{}', skipping",
                                    other,
                                    manifest.id
                                );
                                return None;
                            }
                        };
                        let mut caps = CapabilityFlags::default();
                        for cap in manifest.capabilities {
                            match cap.as_str() {
                                "LOBBY_PAD" => caps |= CapabilityFlags::LOBBY_PAD,
                                "NeedsPhysics" => caps |= CapabilityFlags::NEEDS_PHYSICS,
                                "UsesHitscan" => caps |= CapabilityFlags::USES_HITSCAN,
                                "NeedsNav" => caps |= CapabilityFlags::NEEDS_NAV,
                                "UsesVehicles" => caps |= CapabilityFlags::USES_VEHICLES,
                                "UsesFlight" => caps |= CapabilityFlags::USES_FLIGHT,
                                _ => {}
                            }
                        }
                        Some(ModuleMetadata {
                            id: manifest.id,
                            name: manifest.name,
                            version: manifest.version,
                            author: manifest.author,
                            state,
                            capabilities: caps,
                            max_players: manifest.max_players,
                            icon: Handle::default(),
                        })
                    })
                    .collect::<Vec<_>>(),
                Err(_) => Vec::new(),
            }
        });
        commands.insert_resource(ModuleDiscoveryTask(task));
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = asset_server;
        let _ = commands;
        registry.modules = discover_local_modules();
    }
}

#[cfg(target_arch = "wasm32")]
fn apply_discovered_modules(
    mut commands: Commands,
    mut registry: ResMut<ModuleRegistry>,
    mut task: Option<ResMut<ModuleDiscoveryTask>>,
) {
    if let Some(mut task) = task {
        if let Some(mods) = future::block_on(future::poll_once(&mut task.0)) {
            registry.modules.extend(mods);
            commands.remove_resource::<ModuleDiscoveryTask>();
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Resource)]
struct ModuleWatcher {
    receiver: std::sync::Mutex<Receiver<notify::Result<notify::Event>>>,
    #[allow(dead_code)]
    watcher: RecommendedWatcher,
}

#[cfg(not(target_arch = "wasm32"))]
fn process_module_events(watcher: Res<ModuleWatcher>, mut registry: ResMut<ModuleRegistry>) {
    let mut changed = false;
    if let Ok(rx) = watcher.receiver.lock() {
        while let Ok(_event) = rx.try_recv() {
            changed = true;
        }
    }
    if changed {
        registry.modules = discover_local_modules();
    }
}

pub fn hotload_modules(app: &mut App) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use notify::Config;
        let (tx, rx) = mpsc::channel();
        let mut watcher = match RecommendedWatcher::new(tx, Config::default()) {
            Ok(w) => w,
            Err(e) => {
                log::error!("{}", EngineError::Watcher(e));
                return;
            }
        };
        let modules_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../assets/modules");
        if let Err(e) = watcher.watch(&modules_dir, RecursiveMode::Recursive) {
            log::error!("{}", EngineError::Watcher(e));
            return;
        }
        app.insert_resource(ModuleWatcher {
            receiver: std::sync::Mutex::new(rx),
            watcher,
        });
        app.add_systems(Update, process_module_events);
    }

    #[cfg(target_arch = "wasm32")]
    {
        app.insert_resource(ModuleDiscoveryLoop(spawn_module_discovery_task()));
        app.add_systems(Update, poll_module_discovery_loop);
    }
}

#[cfg(target_arch = "wasm32")]
fn spawn_module_discovery_task() -> Task<()> {
    AsyncComputeTaskPool::get().spawn_local(async move {
        TimeoutFuture::new(1000).await;
    })
}

#[cfg(target_arch = "wasm32")]
fn poll_module_discovery_loop(
    mut task: ResMut<ModuleDiscoveryLoop>,
    mut commands: Commands,
    mut registry: ResMut<ModuleRegistry>,
    asset_server: Option<Res<AssetServer>>,
) {
    if future::block_on(future::poll_once(&mut task.0)).is_some() {
        discover_modules(commands, registry, asset_server);
        task.0 = spawn_module_discovery_task();
    }
}

pub fn update_lobby_pads(
    mut commands: Commands,
    registry: Res<ModuleRegistry>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Option<Res<AssetServer>>,
    pads: Query<Entity, Or<(With<LobbyPad>, With<DocPad>, With<NoModulesSign>)>>,
) {
    for entity in pads.iter() {
        commands.entity(entity).despawn_recursive();
    }

    let pad_mesh = meshes.add(Mesh::from(shape::Cube { size: 1.0 }));
    let pad_material = materials.add(Color::rgb(0.8, 0.2, 0.2).into());
    let font = asset_server
        .as_ref()
        .map(|s| s.load("fonts/FiraSans-Bold.ttf"))
        .unwrap_or_default();

    if registry.modules.is_empty() {
        for (i, &(label, url)) in HELP_DOCS.iter().enumerate() {
            commands
                .spawn((
                    PbrBundle {
                        mesh: pad_mesh.clone(),
                        material: pad_material.clone(),
                        transform: Transform::from_xyz(i as f32 * 3.0 - 6.0, 0.5, 0.0),
                        ..default()
                    },
                    Collider::cuboid(0.5, 0.5, 0.5),
                    Sensor,
                    ActiveEvents::COLLISION_EVENTS,
                    DocPad { url },
                    LobbyEntity,
                ))
                .with_children(|parent| {
                    parent.spawn(Text2dBundle {
                        text: Text::from_section(
                            label,
                            TextStyle {
                                font: font.clone(),
                                font_size: 20.0,
                                color: Color::WHITE,
                            },
                        ),
                        transform: Transform::from_xyz(0.0, 0.75, 0.0),
                        ..default()
                    });
                });
        }

        commands.spawn((
            Text2dBundle {
                text: Text::from_section(
                    "No modules installed",
                    TextStyle {
                        font: font.clone(),
                        font_size: 40.0,
                        color: Color::WHITE,
                    },
                ),
                transform: Transform::from_xyz(0.0, 2.5, 3.0)
                    .looking_at(Vec3::new(0.0, 1.5, 5.0), Vec3::Y),
                ..default()
            },
            NoModulesSign,
            LobbyEntity,
        ));
        return;
    }

    for (i, info) in registry.modules.iter().enumerate() {
        if !info.capabilities.contains(CapabilityFlags::LOBBY_PAD) {
            continue;
        }
        commands
            .spawn((
                PbrBundle {
                    mesh: pad_mesh.clone(),
                    material: pad_material.clone(),
                    transform: Transform::from_xyz(i as f32 * 3.0 - 3.0, 0.5, 0.0),
                    ..default()
                },
                Collider::cuboid(0.5, 0.5, 0.5),
                Sensor,
                ActiveEvents::COLLISION_EVENTS,
                LobbyPad {
                    state: info.state.clone(),
                },
                LobbyEntity,
            ))
            .with_children(|parent| {
                parent.spawn(Text2dBundle {
                    text: Text::from_section(
                        format!(
                            "{} v{}\nPlayers: {}\nPing: 0ms",
                            info.name,
                            info.version,
                            info.max_players
                        ),
                        TextStyle {
                            font: font.clone(),
                            font_size: 20.0,
                            color: Color::WHITE,
                        },
                    ),
                    transform: Transform::from_xyz(0.0, 0.6, 0.0),
                    ..default()
                });
            });
    }
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
