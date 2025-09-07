use bevy::ecs::schedule::common_conditions::resource_changed;
#[cfg(target_arch = "wasm32")]
use bevy::tasks::{AsyncComputeTaskPool, Task};
use bevy::{input::mouse::MouseMotion, prelude::*, window::CursorGrabMode};
use bevy_rapier3d::prelude::*;
#[cfg(target_arch = "wasm32")]
use futures_lite::future;
#[cfg(target_arch = "wasm32")]
use gloo_timers::future::TimeoutFuture;
#[cfg(not(target_arch = "wasm32"))]
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use platform_api::{AppState, CapabilityFlags, GameModule, ModuleContext, ModuleMetadata};
use serde::Deserialize;
#[cfg(not(target_arch = "wasm32"))]
use std::fs;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc::Receiver;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;

#[cfg(feature = "flight")]
pub mod flight;
#[cfg(feature = "vehicle")]
pub mod vehicle;

#[cfg(feature = "flight")]
use flight::FlightPlugin;
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

pub struct EnginePlugin;

impl Plugin for EnginePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ModuleRegistry>()
            .add_state::<AppState>()
            .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
            .add_systems(Startup, discover_modules)
            .add_systems(OnEnter(AppState::Lobby), setup_lobby)
            .add_systems(OnExit(AppState::Lobby), cleanup_lobby)
            .add_systems(
                Update,
                (lobby_keyboard, player_move, player_look, pad_trigger)
                    .run_if(in_state(AppState::Lobby)),
            )
            .add_systems(Update, exit_to_lobby);

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
    }
}

#[derive(Component)]
struct LobbyEntity;

#[derive(Component)]
struct Player;

#[derive(Component)]
struct PlayerCamera;

#[derive(Component)]
struct Controller {
    yaw: f32,
    pitch: f32,
}

#[derive(Component)]
pub struct LobbyPad {
    pub state: AppState,
}

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

    let pad_mesh = meshes.add(Mesh::from(shape::Cube { size: 1.0 }));
    let pad_material = materials.add(Color::rgb(0.8, 0.2, 0.2).into());

    if registry.modules.is_empty() {
        let font = asset_server
            .as_ref()
            .map(|s| s.load("fonts/FiraSans-Bold.ttf"))
            .unwrap_or_default();
        commands.spawn((
            Text2dBundle {
                text: Text::from_section(
                    "No modules installed",
                    TextStyle {
                        font: font.clone(),
                        font_size: 30.0,
                        color: Color::WHITE,
                    },
                ),
                transform: Transform::from_xyz(0.0, 1.0, 0.0),
                ..default()
            },
            LobbyEntity,
        ));
        commands.spawn((
            Text2dBundle {
                text: Text::from_section(
                    "See docs/modules.md for installation instructions",
                    TextStyle {
                        font,
                        font_size: 20.0,
                        color: Color::WHITE,
                    },
                ),
                transform: Transform::from_xyz(0.0, 0.5, 0.0),
                ..default()
            },
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
                    let font = asset_server
                        .as_ref()
                        .map(|s| s.load("fonts/FiraSans-Bold.ttf"))
                        .unwrap_or_default();
                    let label = if LOBBY_KEYS.get(i).is_some() {
                        format!("[{}] {} v{}", i + 1, info.name, info.version)
                    } else {
                        format!("{} v{}", info.name, info.version)
                    };
                    parent.spawn(Text2dBundle {
                        text: Text::from_section(
                            label,
                            TextStyle {
                                font,
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

fn player_move(
    time: Res<Time>,
    keys: Res<Input<KeyCode>>,
    mut query: Query<(&Transform, &mut KinematicCharacterController), With<Player>>,
) {
    if let Ok((transform, mut controller)) = query.get_single_mut() {
        let mut direction = Vec3::ZERO;
        if keys.pressed(KeyCode::W) {
            direction += transform.forward();
        }
        if keys.pressed(KeyCode::S) {
            direction -= transform.forward();
        }
        if keys.pressed(KeyCode::A) {
            direction -= transform.right();
        }
        if keys.pressed(KeyCode::D) {
            direction += transform.right();
        }
        controller.translation = Some(direction.normalize_or_zero() * 5.0 * time.delta_seconds());
    }
}

fn player_look(
    mut mouse_motion: EventReader<MouseMotion>,
    mut query: Query<(&mut Controller, &mut Transform), With<Player>>,
    mut cam_query: Query<&mut Transform, With<PlayerCamera>>,
) {
    let Ok((mut controller, mut transform)) = query.get_single_mut() else {
        return;
    };
    let Ok(mut cam_transform) = cam_query.get_single_mut() else {
        return;
    };
    let mut delta = Vec2::ZERO;
    for ev in mouse_motion.read() {
        delta += ev.delta;
    }
    if delta == Vec2::ZERO {
        return;
    }
    controller.yaw -= delta.x * 0.002;
    controller.pitch -= delta.y * 0.002;
    controller.pitch = controller.pitch.clamp(-1.54, 1.54);
    transform.rotation = Quat::from_rotation_y(controller.yaw);
    cam_transform.rotation = Quat::from_rotation_x(controller.pitch);
}

fn pad_trigger(
    mut collisions: EventReader<CollisionEvent>,
    player: Query<Entity, With<Player>>,
    pads: Query<&LobbyPad>,
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
                next_state.set(pad.state.clone());
            }
        }
    }
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
    M::enter(&mut ctx).expect("module enter failed");
}

/// System wrapper that forwards state exit to the module.
fn exit_module<M: GameModule>(world: &mut World) {
    let mut ctx = ModuleContext::new(world);
    M::exit(&mut ctx).expect("module exit failed");
}

#[derive(Deserialize)]
struct ModuleManifest {
    id: String,
    name: String,
    version: String,
    author: String,
    state: String,
    #[serde(default)]
    capabilities: Vec<String>,
    #[serde(default)]
    max_players: u32,
}

#[cfg(not(target_arch = "wasm32"))]
fn read_modules_from_disk() -> Vec<ModuleMetadata> {
    let modules_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../assets/modules");
    let Ok(entries) = fs::read_dir(modules_dir) else {
        return Vec::new();
    };
    let mut mods = Vec::new();
    for entry in entries.flatten() {
        let manifest_path = entry.path().join("module.toml");
        if !manifest_path.exists() {
            continue;
        }
        let Ok(contents) = fs::read_to_string(&manifest_path) else {
            continue;
        };
        let manifest = match toml::from_str::<ModuleManifest>(&contents) {
            Ok(m) => m,
            Err(_) => {
                continue;
            }
        };
        let state = match manifest.state.as_str() {
            "DuckHunt" => AppState::DuckHunt,
            _ => AppState::Lobby,
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
        mods.push(ModuleMetadata {
            id: manifest.id,
            name: manifest.name,
            version: manifest.version,
            author: manifest.author,
            state,
            capabilities: caps,
            max_players: manifest.max_players,
            icon: Handle::default(),
        });
    }
    mods
}

#[cfg(target_arch = "wasm32")]
#[derive(Resource)]
struct ModuleDiscoveryTask(Task<Vec<ModuleMetadata>>);

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
                    .map(|manifest| {
                        let state = match manifest.state.as_str() {
                            "DuckHunt" => AppState::DuckHunt,
                            _ => AppState::Lobby,
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
                        ModuleMetadata {
                            id: manifest.id,
                            name: manifest.name,
                            version: manifest.version,
                            author: manifest.author,
                            state,
                            capabilities: caps,
                            max_players: manifest.max_players,
                            icon: Handle::default(),
                        }
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
        registry.modules = read_modules_from_disk();
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
        registry.modules = read_modules_from_disk();
    }
}

pub fn hotload_modules(app: &mut App) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use notify::Config;
        let (tx, rx) = mpsc::channel();
        let mut watcher = RecommendedWatcher::new(tx, Config::default()).expect("watcher");
        let modules_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../assets/modules");
        watcher
            .watch(&modules_dir, RecursiveMode::Recursive)
            .expect("watch");
        app.insert_resource(ModuleWatcher {
            receiver: std::sync::Mutex::new(rx),
            watcher,
        });
        app.add_systems(Update, process_module_events);
    }

    #[cfg(target_arch = "wasm32")]
    {
        let world_ptr = app.world_mut() as *mut World;
        spawn_local(async move {
            loop {
                TimeoutFuture::new(1000).await;
                unsafe {
                    (*world_ptr).run_system_once(discover_modules);
                }
            }
        });
    }
}

pub fn update_lobby_pads(
    mut commands: Commands,
    registry: Res<ModuleRegistry>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Option<Res<AssetServer>>,
    pads: Query<Entity, With<LobbyPad>>,
) {
    for entity in pads.iter() {
        commands.entity(entity).despawn_recursive();
    }

    let pad_mesh = meshes.add(Mesh::from(shape::Cube { size: 1.0 }));
    let pad_material = materials.add(Color::rgb(0.8, 0.2, 0.2).into());

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
                let font = asset_server
                    .as_ref()
                    .map(|s| s.load("fonts/FiraSans-Bold.ttf"))
                    .unwrap_or_default();
                parent.spawn(Text2dBundle {
                    text: Text::from_section(
                        format!("{} v{}", info.name, info.version),
                        TextStyle {
                            font,
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
