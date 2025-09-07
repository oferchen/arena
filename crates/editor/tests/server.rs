use bevy_ecs::prelude::*;
use editor::{Level, play_in_editor, validate_level};
use editor::{AssetRegistry, Level, SpawnZone, play_in_editor, validate_level};
use platform_api::ModuleContext;

#[test]
fn invalid_level_is_rejected() {
    let mut world = World::new();
    let mut ctx = ModuleContext::new(&mut world);
    let bad = Level::new("", "");
    assert!(validate_level(&mut ctx, &bad).is_err());
}

#[test]
fn play_in_editor_starts_session() {
    let mut world = World::new();
    let mut ctx = ModuleContext::new(&mut world);
    let mut level = Level::new("test-level", "Test Level");
    level.spawn_zones.push(SpawnZone {
        x: 0.0,
        y: 0.0,
        radius: 5.0,
    });

    play_in_editor(&mut ctx, &level).expect("should start editor session");

    let stored = ctx.world().get_resource::<Level>().expect("level missing");
    assert_eq!(stored.id, "test-level");
    assert_eq!(stored.name, "Test Level");
}

#[test]
fn missing_reference_is_rejected() {
    let mut world = World::new();
    world.insert_resource(AssetRegistry::default());
    let mut ctx = ModuleContext::new(&mut world);
    let mut level = Level::new("lvl", "Lvl");
    level.references.push("missing_asset".into());
    assert!(validate_level(&mut ctx, &level).is_err());
}

#[test]
fn illegal_spawn_zone_is_rejected() {
    let mut world = World::new();
    let mut ctx = ModuleContext::new(&mut world);
    let mut level = Level::new("lvl", "Lvl");
    level.spawn_zones.push(SpawnZone {
        x: 2000.0,
        y: 0.0,
        radius: 10.0,
    });
    assert!(validate_level(&mut ctx, &level).is_err());
}

#[test]
fn perf_budget_is_enforced() {
    let mut world = World::new();
    let mut ctx = ModuleContext::new(&mut world);
    let mut level = Level::new("lvl", "Lvl");
    level.spawn_zones.push(SpawnZone {
        x: 0.0,
        y: 0.0,
        radius: 10.0,
    });
    level.entity_count = 2000;
    assert!(validate_level(&mut ctx, &level).is_err());
}
