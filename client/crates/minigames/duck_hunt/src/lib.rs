use anyhow::Result;
use bevy::prelude::*;
use bevy::render::mesh::shape::UVSphere;
use net::{
    CurrentFrame,
    client::ConnectionEvent,
    message::{InputFrame, Snapshot},
};
use platform_api::{
    AppState, CapabilityFlags, GameModule, ModuleContext, ModuleMetadata, ServerApp,
};
use serde::{Deserialize, Serialize};

#[derive(Resource, Default)]
struct Score(pub u32);

#[derive(Resource, Default)]
struct RoundTimer(pub Timer);

#[derive(Resource, Default)]
struct DuckSpawnTimer(pub Timer);

#[derive(Component)]
struct Duck {
    spline: Spline,
    t: f32,
}

#[derive(Clone)]
struct Spline {
    points: Vec<Vec3>,
    duration: f32,
}

impl Spline {
    fn sample(&self, segment: usize, t: f32) -> Vec3 {
        if self.points.len() < 2 {
            return Vec3::ZERO;
        }
        let seg = segment.min(self.points.len() - 2);
        let start = self.points[seg];
        let end = self.points[seg + 1];
        start.lerp(end, t.clamp(0.0, 1.0))
    }
}

#[derive(Component)]
struct HudText;

#[derive(Serialize, Deserialize)]
struct Shot {
    origin: [f32; 3],
    direction: [f32; 3],
    time: f32,
}

#[derive(Default)]
pub struct DuckHuntPlugin;

impl Plugin for DuckHuntPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                spawn_ducks,
                move_ducks,
                fire_weapon,
                apply_score_snapshots,
                update_hud,
                update_round_timer,
                log_connection_events,
            ),
        );
    }
}

#[derive(Component)]
struct DuckHuntEntity;

fn setup(world: &mut World) {
    world.spawn((Camera3dBundle::default(), DuckHuntEntity));

    world.insert_resource(Score(0));
    world.insert_resource(RoundTimer(Timer::from_seconds(90.0, TimerMode::Once)));
    world.insert_resource(DuckSpawnTimer(Timer::from_seconds(
        2.0,
        TimerMode::Repeating,
    )));

    let Some(asset_server) = world.get_resource::<AssetServer>() else {
        return;
    };
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");

    world.spawn((
        TextBundle::from_section(
            "Score: 0\nTime: 90",
            TextStyle {
                font,
                font_size: 24.0,
                color: Color::WHITE,
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            left: Val::Px(10.0),
            top: Val::Px(10.0),
            ..default()
        }),
        HudText,
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

    world.remove_resource::<Score>();
    world.remove_resource::<RoundTimer>();
    world.remove_resource::<DuckSpawnTimer>();
}

impl GameModule for DuckHuntPlugin {
    const ID: &'static str = "duck_hunt";

    fn metadata() -> ModuleMetadata {
        ModuleMetadata {
            id: Self::ID.to_string(),
            name: "Duck Hunt".to_string(),
            version: "0.1.0".to_string(),
            author: "Unknown".to_string(),
            state: AppState::DuckHunt,
            capabilities: CapabilityFlags::LOBBY_PAD,
            max_players: 4,
            icon: Handle::default(),
        }
    }

    fn register(_app: &mut App) {}

    fn enter(ctx: &mut ModuleContext) -> Result<()> {
        setup(ctx.world());
        Ok(())
    }

    fn exit(ctx: &mut ModuleContext) -> Result<()> {
        cleanup(ctx.world());
        Ok(())
    }

    fn server_register(_app: &mut ServerApp) {}
}

fn spawn_ducks(
    mut commands: Commands,
    time: Res<Time>,
    mut timer: ResMut<DuckSpawnTimer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        let mesh = meshes.add(Mesh::from(UVSphere {
            radius: 0.25,
            ..default()
        }));
        let material = materials.add(Color::rgb(0.2, 0.8, 0.2).into());
        commands.spawn((
            PbrBundle {
                mesh,
                material,
                transform: Transform::from_translation(Vec3::new(-5.0, 0.5, 0.0)),
                ..default()
            },
            Duck {
                spline: Spline {
                    points: vec![Vec3::new(-5.0, 0.5, 0.0), Vec3::new(5.0, 2.0, 0.0)],
                    duration: 5.0,
                },
                t: 0.0,
            },
            DuckHuntEntity,
        ));
    }
}

fn move_ducks(
    time: Res<Time>,
    mut q: Query<(Entity, &mut Transform, &mut Duck)>,
    mut commands: Commands,
) {
    for (e, mut transform, mut duck) in &mut q {
        duck.t += time.delta_seconds() / duck.spline.duration;
        if duck.t >= 1.0 {
            commands.entity(e).despawn_recursive();
        } else {
            let segments = duck.spline.points.len().saturating_sub(1) as f32;
            let seg_t = duck.t * segments;
            let segment = seg_t.floor() as usize;
            let local_t = seg_t - segment as f32;
            transform.translation = duck.spline.sample(segment, local_t);
        }
    }
}

fn fire_weapon(
    buttons: Res<Input<MouseButton>>,
    q: Query<(Entity, &Transform), With<Duck>>,
    camera: Query<&Transform, With<Camera3d>>,
    time: Res<Time>,
    mut commands: Commands,
    mut writer: EventWriter<InputFrame>,
    frame: Res<CurrentFrame>,
) {
    if buttons.just_pressed(MouseButton::Left) {
        if let Ok(cam) = camera.get_single() {
            let shot = Shot {
                origin: cam.translation.to_array(),
                direction: cam.forward().to_array(),
                time: time.elapsed_seconds_f64() as f32,
            };
            if let Ok(data) = postcard::to_allocvec(&shot) {
                writer.send(InputFrame {
                    frame: frame.0,
                    data,
                });
            }
        }
        if let Some((entity, _)) = q.iter().next() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn update_hud(score: Res<Score>, timer: Res<RoundTimer>, mut q: Query<&mut Text, With<HudText>>) {
    if score.is_changed() || timer.is_changed() {
        for mut text in &mut q {
            let remaining = timer.0.remaining_secs().ceil() as u32;
            text.sections[0].value = format!("Score: {}\nTime: {remaining}", score.0);
        }
    }
}

fn apply_score_snapshots(mut reader: EventReader<Snapshot>, mut score: ResMut<Score>) {
    for snap in reader.read() {
        if let Ok(scores) = postcard::from_bytes::<Vec<u32>>(&snap.data) {
            score.0 = scores.get(0).copied().unwrap_or(0);
        }
    }
}

fn update_round_timer(
    time: Res<Time>,
    mut timer: ResMut<RoundTimer>,
    mut q: Query<Entity, With<Duck>>,
    mut commands: Commands,
) {
    if timer.0.tick(time.delta()).finished() {
        for e in &mut q {
            commands.entity(e).despawn_recursive();
        }
    }
}

fn log_connection_events(mut events: EventReader<ConnectionEvent>) {
    for ev in events.read() {
        info!("connection event: {ev:?}");

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_at(spline: &Spline, t: f32) -> Vec3 {
        let segments = spline.points.len() - 1;
        let seg_t = t * segments as f32;
        let segment = seg_t.floor().min((segments - 1) as f32) as usize;
        let local_t = seg_t - segment as f32;
        spline.sample(segment, local_t)
    }

    #[test]
    fn spline_handles_multiple_segments() {
        let spline = Spline {
            points: vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(1.0, 1.0, 0.0),
                Vec3::new(2.0, 1.0, 0.0),
            ],
            duration: 1.0,
        };
        let checks = [
            (0.0, Vec3::new(0.0, 0.0, 0.0)),
            (1.0 / 3.0, Vec3::new(1.0, 0.0, 0.0)),
            (2.0 / 3.0, Vec3::new(1.0, 1.0, 0.0)),
            (1.0, Vec3::new(2.0, 1.0, 0.0)),
            (1.0 / 6.0, Vec3::new(0.5, 0.0, 0.0)),
            (0.5, Vec3::new(1.0, 0.5, 0.0)),
            (5.0 / 6.0, Vec3::new(1.5, 1.0, 0.0)),
        ];
        for (t, expected) in checks {
            assert!(sample_at(&spline, t).distance(expected) < 1e-5);
        }
    }
}
