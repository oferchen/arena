use duck_hunt_server::{server::{spawn_wave, Server}, award_score, DuckHuntModule, Score};
use platform_api::{ModuleContext, GameModule};
use bevy::prelude::World;
use std::time::Duration;

#[test]
fn spawns_are_deterministic() {
    let mut server_a = Server { latency: Duration::from_secs(0), ducks: vec![], snapshot_txs: Vec::new() };
    spawn_wave(&mut server_a, 42, 3);
    let ducks_a = server_a.ducks.clone();

    let mut server_b = Server { latency: Duration::from_secs(0), ducks: vec![], snapshot_txs: Vec::new() };
    spawn_wave(&mut server_b, 42, 3);
    assert_eq!(ducks_a, server_b.ducks);
}

#[test]
fn scoring_accumulates() {
    let mut world = World::new();
    let mut ctx = ModuleContext::new(&mut world);
    DuckHuntModule::enter(&mut ctx).unwrap();
    award_score(&mut world, 1);
    award_score(&mut world, 2);
    let score = world.get_resource::<Score>().unwrap();
    assert_eq!(score.0, 3);
}
