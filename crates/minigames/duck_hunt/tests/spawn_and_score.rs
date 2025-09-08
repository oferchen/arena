use duck_hunt_server::{
    server::{replicate, spawn_duck, spawn_wave, Server, DuckState},
    award_score,
    DuckHuntModule,
    Score,
    Multiplier,
};
use net::message::ServerMessage;
use tokio::sync::mpsc;
use glam::Vec3;
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
fn scoring_accumulates_with_multiplier() {
    let mut world = World::new();
    let mut ctx = ModuleContext::new(&mut world);
    DuckHuntModule::enter(&mut ctx).unwrap();
    award_score(&mut world, 1); // multiplier 1 -> score 1
    award_score(&mut world, 2); // multiplier 2 -> +4
    let score = world.get_resource::<Score>().unwrap();
    let mult = world.get_resource::<Multiplier>().unwrap();
    assert_eq!(score.0, 5);
    assert_eq!(mult.0, 3); // multiplier advanced twice
}

#[tokio::test]
async fn replication_broadcasts_state() {
    let (tx, mut rx) = mpsc::channel(1);
    let mut server = Server { latency: Duration::from_secs(0), ducks: vec![], snapshot_txs: vec![tx] };
    spawn_duck(&mut server, Vec3::ZERO, Vec3::X);
    // initial broadcast from spawn
    rx.try_recv().expect("baseline missing");
    if let Some(duck) = server.ducks.get_mut(0) {
        duck.position = Vec3::new(1.0, 0.0, 0.0);
        let cloned = duck.clone();
        replicate(&server, &cloned);
    }
    match rx.try_recv().expect("no replication") {
        ServerMessage::Baseline(snap) => {
            let d: DuckState = postcard::from_bytes(&snap.data).unwrap();
            assert_eq!(d.position, Vec3::new(1.0, 0.0, 0.0));
        }
        other => panic!("unexpected message: {:?}", other),
    }
}
