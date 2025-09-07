use bevy_ecs::prelude::*;
use editor::{Level, play_in_editor, validate_level};
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
    let level = Level::new("test-level", "Test Level");

    play_in_editor(&mut ctx, &level).expect("should start editor session");

    let stored = ctx.world().get_resource::<Level>().expect("level missing");
    assert_eq!(stored.id, "test-level");
    assert_eq!(stored.name, "Test Level");
}
