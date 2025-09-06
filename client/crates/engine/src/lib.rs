use bevy::{input::mouse::MouseMotion, prelude::*, window::CursorGrabMode};
use bevy_rapier3d::prelude::*;
use platform_api::{AppState, CapabilityFlags, GameModule, ModuleContext, ModuleMetadata};

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
            .add_systems(OnEnter(AppState::Lobby), setup_lobby)
            .add_systems(OnExit(AppState::Lobby), cleanup_lobby)
            .add_systems(
                Update,
                (
                    lobby_keyboard,
                    player_move,
                    player_look,
                    pad_trigger,
                )
                    .run_if(in_state(AppState::Lobby)),
            )
            .add_systems(Update, exit_to_lobby);
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
struct LobbyPad {
    state: AppState,
}

fn setup_lobby(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    registry: Res<ModuleRegistry>,
    asset_server: Res<AssetServer>,
    mut windows: Query<&mut Window>,
) {
    let mut window = windows.single_mut();
    window.cursor.grab_mode = CursorGrabMode::Locked;
    window.cursor.visible = false;

    commands
        .spawn((
            TransformBundle::from_transform(Transform::from_xyz(0.0, 1.5, 5.0)),
            RigidBody::KinematicPositionBased,
            Collider::capsule_y(0.5, 0.3),
            KinematicCharacterController::default(),
            Controller { yaw: 0.0, pitch: 0.0 },
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
            mesh: meshes.add(Mesh::from(shape::Plane { size: 10.0, subdivisions: 0 })),
            material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
            ..default()
        },
        LobbyEntity,
    ));

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
                parent.spawn(Text2dBundle {
                    text: Text::from_section(
                        format!("{} v{}", info.name, info.version),
                        TextStyle {
                            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
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

fn exit_to_lobby(
    keys: Res<Input<KeyCode>>,
    mut next_state: ResMut<NextState<AppState>>,
    mut windows: Query<&mut Window>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        let mut window = windows.single_mut();
        let locked = window.cursor.grab_mode == CursorGrabMode::Locked;
        if locked {
            window.cursor.grab_mode = CursorGrabMode::None;
            window.cursor.visible = true;
        } else {
            window.cursor.grab_mode = CursorGrabMode::Locked;
            window.cursor.visible = false;
        }
        next_state.set(AppState::Lobby);
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
        controller.translation =
            Some(direction.normalize_or_zero() * 5.0 * time.delta_seconds());
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
    {
        let world = &mut app.world;
        M::server_register(&mut ModuleContext { world });
    }
    app.world
        .get_resource_mut::<ModuleRegistry>()
        .expect("EnginePlugin must be added before registering modules")
        .modules
        .push(info);
    app.add_plugins(M::default());
    app.add_systems(OnEnter(state.clone()), enter_module::<M>);
    app.add_systems(OnExit(state), exit_module::<M>);
}

/// System wrapper that forwards state entry to the module.
fn enter_module<M: GameModule>(world: &mut World) {
    M::enter(&mut ModuleContext { world });
}

/// System wrapper that forwards state exit to the module.
fn exit_module<M: GameModule>(world: &mut World) {
    M::exit(&mut ModuleContext { world });
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
