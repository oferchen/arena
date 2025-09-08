use analytics::{Analytics, Event};
use chrono::Utc;
use glam::Vec3;
use leaderboard::{
    LeaderboardService,
    models::{LeaderboardWindow, Run, Score},
};
use rand::{Rng, SeedableRng, rngs::StdRng};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

pub mod net {
    use super::DuckState;
    use net::message::{ServerMessage, Snapshot};
    use postcard;
    use serde::Serialize;
    use std::time::Duration;
    use tokio::sync::mpsc::Sender;

    #[derive(Clone)]
    pub struct Server {
        pub latency: Duration,
        pub ducks: Vec<DuckState>,
        pub snapshot_txs: Vec<Sender<ServerMessage>>,
    }

    impl Server {
        pub fn broadcast<T: Serialize>(&self, msg: &T) {
            if let Ok(data) = postcard::to_allocvec(msg) {
                let snap = Snapshot { frame: 0, data };
                let msg = ServerMessage::Baseline(snap);
                for tx in &self.snapshot_txs {
                    let _ = tx.try_send(msg.clone());
                }
            }
        }

        pub fn latency(&self) -> Duration {
            self.latency
        }

        pub fn ducks(&self) -> &[DuckState] {
            &self.ducks
        }
    }
}

pub use net::Server;

const DUCK_RADIUS: f32 = 0.5;

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct DuckState {
    pub position: Vec3,
    pub velocity: Vec3,
    pub path: Vec<Vec3>,
    pub path_index: usize,
}

pub fn spawn_duck(server: &mut Server, position: Vec3, velocity: Vec3) {
    let path = vec![position, position + velocity];
    let state = DuckState {
        position,
        velocity,
        path,
        path_index: 0,
    };
    server.ducks.push(state.clone());
    // send initial state to clients
    server.broadcast(&state);
}

pub fn spawn_duck_path(server: &mut Server, path: Vec<Vec3>, speed: f32) {
    if path.is_empty() {
        return;
    }
    let position = path[0];
    let mut velocity = Vec3::ZERO;
    if path.len() > 1 {
        velocity = (path[1] - path[0]).normalize_or_zero() * speed;
    }
    let state = DuckState {
        position,
        velocity,
        path,
        path_index: 0,
    };
    server.ducks.push(state.clone());
    server.broadcast(&state);
}

pub fn replicate(server: &Server, state: &DuckState) {
    server.broadcast(state);
}

/// Spawn a deterministic wave of ducks using the provided `seed`.
/// This allows tests and replays to reproduce identical spawns.
pub fn spawn_wave(server: &mut Server, seed: u64, count: usize) {
    let mut rng = StdRng::seed_from_u64(seed);
    for _ in 0..count {
        let position = Vec3::new(rng.gen_range(-5.0..5.0), rng.gen_range(0.5..2.5), 0.0);
        let velocity = Vec3::new(rng.gen_range(-1.0..1.0), 0.0, 0.0);
        spawn_duck(server, position, velocity);
    }
}

pub fn advance_ducks(server: &mut Server, dt: f32) {
    let mut updated = Vec::new();
    for duck in &mut server.ducks {
        if duck.path_index + 1 < duck.path.len() {
            let target = duck.path[duck.path_index + 1];
            let dir = target - duck.position;
            let travel = duck.velocity.length() * dt;
            if dir.length() <= travel {
                duck.position = target;
                duck.path_index += 1;
                if duck.path_index + 1 < duck.path.len() {
                    let next = duck.path[duck.path_index + 1];
                    duck.velocity = (next - target).normalize_or_zero() * duck.velocity.length();
                }
            } else {
                duck.position += dir.normalize_or_zero() * travel;
            }
        } else {
            duck.position += duck.velocity * dt;
        }
        updated.push(duck.clone());
    }
    for duck in &updated {
        server.broadcast(duck);
    }
}

pub fn validate_hit(server: &Server, origin: Vec3, direction: Vec3, shot_time: Duration) -> bool {
    let rewind = shot_time + server.latency();
    let rewind_secs = rewind.as_secs_f32();
    let dir = direction.normalize();

    for duck in server.ducks() {
        let center = duck.position - duck.velocity * rewind_secs;
        if ray_sphere_intersect(origin, dir, center, DUCK_RADIUS) {
            return true;
        }
    }

    false
}

pub fn serialize_replay(origin: Vec3, direction: Vec3, time: f32) -> Vec<u8> {
    #[derive(Serialize, Deserialize)]
    struct Shot {
        origin: [f32; 3],
        direction: [f32; 3],
        time: f32,
    }
    let shot = Shot {
        origin: origin.to_array(),
        direction: direction.to_array(),
        time,
    };
    postcard::to_allocvec(&shot).unwrap_or_default()
}

pub async fn handle_shot(
    server: &Server,
    leaderboard: &LeaderboardService,
    analytics: Option<&Analytics>,
    leaderboard_id: Uuid,
    player_id: Uuid,
    origin: Vec3,
    direction: Vec3,
    shot_time: Duration,
    replay: Vec<u8>,
) -> bool {
    if let Some(a) = analytics {
        a.dispatch(Event::ShotFired);
    }
    if validate_hit(server, origin, direction, shot_time) {
        if let Some(a) = analytics {
            a.dispatch(Event::TargetHit);
            a.dispatch(Event::DamageTaken);
            a.dispatch(Event::Death);
            a.dispatch(Event::CurrencyEarned);
        }
        let run_id = Uuid::new_v4();
        let run = Run {
            id: run_id,
            leaderboard_id,
            player_id,
            replay_path: String::new(),
            created_at: Utc::now(),
            flagged: false,
            replay_index: 0,
        };
        let score = Score {
            id: Uuid::new_v4(),
            run_id,
            player_id,
            points: 1,
            verified: true,
            created_at: Utc::now(),
            window: LeaderboardWindow::AllTime,
        };
        let _ = leaderboard
            .submit_score(leaderboard_id, score, run, replay)
            .await;
        if let Some(a) = analytics {
            a.dispatch(Event::LeaderboardSubmit);
        }
        return true;
    }
    false
}

fn ray_sphere_intersect(origin: Vec3, dir: Vec3, center: Vec3, radius: f32) -> bool {
    let m = origin - center;
    let b = m.dot(dir);
    let c = m.length_squared() - radius * radius;
    if c > 0.0 && b > 0.0 {
        return false;
    }
    let discriminant = b * b - c;
    discriminant >= 0.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn stationary_duck_no_latency() {
        let server = Server {
            latency: Duration::from_secs_f32(0.0),
            ducks: vec![DuckState {
                position: Vec3::new(0.0, 0.0, 5.0),
                velocity: Vec3::ZERO,
                path: Vec::new(),
                path_index: 0,
            }],
            snapshot_txs: Vec::new(),
        };

        let hit = validate_hit(&server, Vec3::ZERO, Vec3::Z, Duration::from_secs_f32(0.0));
        assert!(hit);
    }

    #[test]
    fn moving_duck_with_latency() {
        let server = Server {
            latency: Duration::from_secs_f32(0.2),
            ducks: vec![DuckState {
                position: Vec3::new(2.0, 0.0, 5.0),
                velocity: Vec3::new(10.0, 0.0, 0.0),
                path: Vec::new(),
                path_index: 0,
            }],
            snapshot_txs: Vec::new(),
        };

        let hit = validate_hit(&server, Vec3::ZERO, Vec3::Z, Duration::from_secs_f32(0.0));
        assert!(hit);
    }

    #[test]
    fn miss_due_to_direction() {
        let server = Server {
            latency: Duration::from_secs_f32(0.0),
            ducks: vec![DuckState {
                position: Vec3::new(0.0, 0.0, 5.0),
                velocity: Vec3::ZERO,
                path: Vec::new(),
                path_index: 0,
            }],
            snapshot_txs: Vec::new(),
        };

        let hit = validate_hit(&server, Vec3::ZERO, Vec3::X, Duration::from_secs_f32(0.0));
        assert!(!hit);
    }

    #[test]
    fn advance_updates_position() {
        let mut server = Server {
            latency: Duration::from_secs_f32(0.0),
            ducks: vec![DuckState {
                position: Vec3::ZERO,
                velocity: Vec3::new(1.0, 0.0, 0.0),
                path: Vec::new(),
                path_index: 0,
            }],
            snapshot_txs: Vec::new(),
        };
        advance_ducks(&mut server, 1.0);
        assert_eq!(server.ducks[0].position, Vec3::new(1.0, 0.0, 0.0));
    }

    #[tokio::test]
    async fn leaderboard_records_hit() {
        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var("ARENA_REDIS_URL", "redis://127.0.0.1/");
        let service = LeaderboardService::new("127.0.0.1:9042", tmp.path().into())
            .await
            .unwrap();
        let server = Server {
            latency: Duration::from_secs_f32(0.0),
            ducks: vec![DuckState {
                position: Vec3::new(0.0, 0.0, 5.0),
                velocity: Vec3::ZERO,
                path: Vec::new(),
                path_index: 0,
            }],
            snapshot_txs: Vec::new(),
        };
        let leaderboard_id = Uuid::new_v4();
        let player_id = Uuid::new_v4();
        let replay = b"shot".to_vec();
        let hit = handle_shot(
            &server,
            &service,
            None,
            leaderboard_id,
            player_id,
            Vec3::ZERO,
            Vec3::Z,
            Duration::from_secs_f32(0.0),
            replay.clone(),
        )
        .await;
        assert!(hit);
        let scores = service
            .get_scores(leaderboard_id, LeaderboardWindow::AllTime)
            .await;
        assert_eq!(scores.len(), 1);
        assert_eq!(scores[0].points, 1);
        let stored = service.get_replay(scores[0].run_id).await.unwrap();
        assert_eq!(stored, replay);
    }

    #[tokio::test]
    async fn dispatches_analytics_events() {
        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var("ARENA_REDIS_URL", "redis://127.0.0.1/");
        let service = LeaderboardService::new("127.0.0.1:9042", tmp.path().into())
            .await
            .unwrap();
        let server = Server {
            latency: Duration::from_secs_f32(0.0),
            ducks: vec![DuckState {
                position: Vec3::new(0.0, 0.0, 5.0),
                velocity: Vec3::ZERO,
                path: Vec::new(),
                path_index: 0,
            }],
            snapshot_txs: Vec::new(),
        };
        let leaderboard_id = Uuid::new_v4();
        let player_id = Uuid::new_v4();
        let replay = b"shot".to_vec();
        let analytics = Analytics::new(true, None, None);
        let hit = handle_shot(
            &server,
            &service,
            Some(&analytics),
            leaderboard_id,
            player_id,
            Vec3::ZERO,
            Vec3::Z,
            Duration::from_secs_f32(0.0),
            replay,
        )
        .await;
        assert!(hit);
        assert_eq!(
            analytics.events(),
            vec![
                Event::ShotFired,
                Event::TargetHit,
                Event::DamageTaken,
                Event::Death,
                Event::CurrencyEarned,
                Event::LeaderboardSubmit,
            ]
        );
    }

    #[test]
    fn deterministic_replay_serialization() {
        let a = serialize_replay(Vec3::ZERO, Vec3::Z, 0.1);
        let b = serialize_replay(Vec3::ZERO, Vec3::Z, 0.1);
        assert_eq!(a, b);
    }
}
