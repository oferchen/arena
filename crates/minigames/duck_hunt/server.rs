use std::time::Duration;
use bevy::prelude::*;
use net::Server;

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
    let _ = (server, origin, direction, shot_time);
    // TODO: lag compensation and hit validation
    true
}
