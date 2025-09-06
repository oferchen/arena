use std::time::Duration;
use glam::Vec3;

pub mod net {
    use super::DuckState;
    use std::time::Duration;

    #[derive(Clone)]
    pub struct Server {
        pub latency: Duration,
        pub ducks: Vec<DuckState>,
    }

    impl Server {
        pub fn broadcast<T>(&self, _msg: &T) {}

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

#[derive(Clone)]
pub struct DuckState {
    pub position: Vec3,
    pub velocity: Vec3,
}

pub fn spawn_duck(server: &mut Server, position: Vec3, velocity: Vec3) {
    let state = DuckState { position, velocity };
    // send initial state to clients
    server.broadcast(&state);
}

pub fn replicate(server: &mut Server, state: &DuckState) {
    server.broadcast(state);
}

pub fn validate_hit(
    server: &Server,
    origin: Vec3,
    direction: Vec3,
    shot_time: Duration,
) -> bool {
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
        };

        let hit = validate_hit(&server, Vec3::ZERO, Vec3::X, Duration::from_secs_f32(0.0));
        assert!(!hit);
    }
}
