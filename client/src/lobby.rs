use bevy::prelude::*;
use bevy::window::CursorGrabMode;
use bevy_rapier3d::prelude::*;
use engine::motion::{Controller, Player, PlayerCamera};
use engine::{LobbyPad, ModuleRegistry, lobby_keyboard};
use platform_api::AppState;

#[derive(Component)]
struct LobbyEntity;

/// Spawns lobby scene, pads, and pointer-lock movement.
fn setup_lobby(
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
    }

    // player with camera
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

    // lighting and floor
    commands.spawn((
        DirectionalLightBundle {
            ..Default::default()
        },
        LobbyEntity,
    ));
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Plane {
                size: 20.0,
                subdivisions: 0,
            })),
            material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
            ..default()
        },
        LobbyEntity,
    ));

    let pad_mesh = meshes.add(Mesh::from(shape::Cube { size: 1.0 }));
    let pad_material = materials.add(Color::rgb(0.8, 0.2, 0.2).into());
    let font = asset_server
        .as_ref()
        .map(|s| s.load("fonts/FiraSans-Bold.ttf"))
        .unwrap_or_default();

    if registry.modules.is_empty() {
        commands.spawn((
            Text2dBundle {
                text: Text::from_section(
                    "No modules installed – see Docs pads for setup instructions",
                    TextStyle {
                        font: font.clone(),
                        font_size: 40.0,
                        color: Color::WHITE,
                    },
                ),
                transform: Transform::from_xyz(0.0, 2.0, 0.0),
                ..default()
            },
            LobbyEntity,
        ));
        return;
    }

    for (i, info) in registry.modules.iter().enumerate() {
        let x = i as f32 * 3.0 - (registry.modules.len() as f32);
        commands
            .spawn((
                PbrBundle {
                    mesh: pad_mesh.clone(),
                    material: pad_material.clone(),
                    transform: Transform::from_xyz(x, 0.5, 0.0),
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
                            info.name, info.version, info.max_players
                        ),
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

    // simple leaderboard placeholder
    commands.spawn((
        Text2dBundle {
            text: Text::from_section(
                "Leaderboard coming soon",
                TextStyle {
                    font,
                    font_size: 24.0,
                    color: Color::WHITE,
                },
            ),
            transform: Transform::from_xyz(-4.0, 2.0, -2.0),
            ..default()
        },
        LobbyEntity,
    ));
}

/// Remove lobby entities and release pointer lock.
fn cleanup_lobby(
    mut commands: Commands,
    q: Query<Entity, With<LobbyEntity>>,
    mut windows: Query<&mut Window>,
) {
    if let Ok(mut window) = windows.get_single_mut() {
        window.cursor.grab_mode = CursorGrabMode::None;
        window.cursor.visible = true;
    }
    for e in &q {
        commands.entity(e).despawn_recursive();
    }
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
        }
    }
}

pub struct LobbyPlugin;

impl Plugin for LobbyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Lobby), setup_lobby)
            .add_systems(OnExit(AppState::Lobby), cleanup_lobby)
            .add_systems(Update, lobby_keyboard.run_if(in_state(AppState::Lobby)))
            .add_systems(FixedUpdate, pad_trigger.run_if(in_state(AppState::Lobby)))
            .add_systems(Update, exit_to_lobby);
    }
}
