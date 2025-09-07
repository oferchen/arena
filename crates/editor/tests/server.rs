use bevy_ecs::prelude::*;
use editor::{
    export_level,
    play_in_editor,
    stop_play_in_editor,
    validate_level,
    AssetRegistry,
    EditorSession,
    Level,
    SpawnZone,
};
use null_module::NullModule;
use platform_api::ModuleContext;

#[derive(Component, Clone, Debug, PartialEq)]
struct TestComponent(pub i32);

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

    play_in_editor::<NullModule>(&mut ctx, &level).expect("should start editor session");

    let session = ctx
        .world()
        .get_non_send_resource::<EditorSession>()
        .expect("session missing");
    let stored = session
        .app
        .world
        .get_resource::<Level>()
        .expect("level missing");
    assert_eq!(stored.id, "test-level");
    assert_eq!(stored.name, "Test Level");

    stop_play_in_editor(&mut ctx);
}

#[test]
fn round_trip_export_play_modify_replay() {
    let mut world = World::new();
    let mut ctx = ModuleContext::new(&mut world);

    // initial editor setup
    let entity = ctx.world().spawn(TestComponent(1)).id();
    let mut level = Level::new("roundtrip", "Round Trip");
    level.spawn_zones.push(SpawnZone {
        x: 0.0,
        y: 0.0,
        radius: 5.0,
    });
    export_level(&level).unwrap();

    play_in_editor::<NullModule>(&mut ctx, &level).unwrap();

    // modify during play
    {
        let mut session = ctx
            .world()
            .get_non_send_resource_mut::<EditorSession>()
            .unwrap();
        let mut comp = session.app.world.get_mut::<TestComponent>(entity).unwrap();
        comp.0 = 2;
    }

    // stop and modify again in editor
    stop_play_in_editor(&mut ctx);
    {
        let mut comp = ctx.world().get_mut::<TestComponent>(entity).unwrap();
        assert_eq!(comp.0, 2);
        comp.0 = 3;
    }
    level.name = "Round Trip 2".into();
    export_level(&level).unwrap();

    // replay
    play_in_editor::<NullModule>(&mut ctx, &level).unwrap();
    {
        let session = ctx
            .world()
            .get_non_send_resource::<EditorSession>()
            .unwrap();
        let comp = session.app.world.get::<TestComponent>(entity).unwrap();
        assert_eq!(comp.0, 3);
        let stored = session.app.world.get_resource::<Level>().unwrap();
        assert_eq!(stored.name, "Round Trip 2");
    }
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
