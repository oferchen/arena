use chrono::Utc;
use glam::Vec3;
use leaderboard::{models::{LeaderboardWindow, Run, Score}, LeaderboardService};
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

pub fn advance_ducks(server: &mut Server, dt: f32) {
    let mut updated = Vec::new();
    for duck in &mut server.ducks {
        duck.position += duck.velocity * dt;
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

pub async fn handle_shot(
    server: &Server,
    leaderboard: &LeaderboardService,
    leaderboard_id: Uuid,
    player_id: Uuid,
    origin: Vec3,
    direction: Vec3,
    shot_time: Duration,
    replay: Vec<u8>,
) -> bool {
    if validate_hit(server, origin, direction, shot_time) {
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

    #[test]
    fn advance_updates_position() {
        let mut server = Server {
            latency: Duration::from_secs_f32(0.0),
            ducks: vec![DuckState {
                position: Vec3::ZERO,
                velocity: Vec3::new(1.0, 0.0, 0.0),
            }],
            snapshot_txs: Vec::new(),
        };
        advance_ducks(&mut server, 1.0);
        assert_eq!(server.ducks[0].position, Vec3::new(1.0, 0.0, 0.0));
    }

    #[tokio::test]
    async fn leaderboard_records_hit() {
        let tmp = tempfile::tempdir().unwrap();
        let service =
            LeaderboardService::new("sqlite::memory:", tmp.path().into()).await.unwrap();
        let server = Server {
            latency: Duration::from_secs_f32(0.0),
            ducks: vec![DuckState {
                position: Vec3::new(0.0, 0.0, 5.0),
                velocity: Vec3::ZERO,
            }],
            snapshot_txs: Vec::new(),
        };
        let leaderboard_id = Uuid::new_v4();
        let player_id = Uuid::new_v4();
        let replay = b"shot".to_vec();
        let hit = handle_shot(
            &server,
            &service,
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
}
