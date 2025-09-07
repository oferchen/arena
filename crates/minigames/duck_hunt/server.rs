use glam::Vec3;
use serde::{Deserialize, Serialize};
use std::time::Duration;

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

pub trait Leaderboard {
    fn submit_score(&mut self, score: u32);
}

const DUCK_RADIUS: f32 = 0.5;

#[derive(Clone, Serialize, Deserialize)]
pub struct DuckState {
    pub position: Vec3,
    pub velocity: Vec3,
}

pub fn spawn_duck(server: &mut Server, position: Vec3, velocity: Vec3) {
    let state = DuckState { position, velocity };
    server.ducks.push(state.clone());
    // send initial state to clients
    server.broadcast(&state);
}

pub fn replicate(server: &Server, state: &DuckState) {
    server.broadcast(state);
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

pub fn handle_shot(
    server: &Server,
    leaderboard: &mut dyn Leaderboard,
    origin: Vec3,
    direction: Vec3,
    shot_time: Duration,
) -> bool {
    if validate_hit(server, origin, direction, shot_time) {
        leaderboard.submit_score(1);
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
            }],
            snapshot_txs: Vec::new(),
        };

        let hit = validate_hit(&server, Vec3::ZERO, Vec3::X, Duration::from_secs_f32(0.0));
        assert!(!hit);
    }

    struct TestLeaderboard(u32);

    impl Leaderboard for TestLeaderboard {
        fn submit_score(&mut self, score: u32) {
            self.0 += score;
        }
    }

    #[test]
    fn leaderboard_records_hit() {
        let server = Server {
            latency: Duration::from_secs_f32(0.0),
            ducks: vec![DuckState {
                position: Vec3::new(0.0, 0.0, 5.0),
                velocity: Vec3::ZERO,
            }],
            snapshot_txs: Vec::new(),
        };
        let mut lb = TestLeaderboard(0);
        let hit = handle_shot(
            &server,
            &mut lb,
            Vec3::ZERO,
            Vec3::Z,
            Duration::from_secs_f32(0.0),
        );
        assert!(hit);
        assert_eq!(lb.0, 1);
    }
}
