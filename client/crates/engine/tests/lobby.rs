use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use engine::{
    DocPad,
    LobbyPad,
    ModuleRegistry,
    discover_modules,
    setup_lobby,
    LeaderboardScreen,
    ReplayPedestal,
};
use std::fs;
use std::path::Path;

fn test_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<ModuleRegistry>();
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.world.spawn(Window::default());
    app
}

fn app_without_window() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<ModuleRegistry>();
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app
}

#[test]
fn spawns_pads_for_modules() {
    let manifest_dir =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../assets/modules/test_mod");
    fs::create_dir_all(&manifest_dir).unwrap();
    fs::write(
        manifest_dir.join("module.toml"),
        r#"id = "duck_hunt"
name = "Duck Hunt"
version = "1.0.0"
author = "Test"
state = "DuckHunt"
capabilities = ["LOBBY_PAD"]
"#,
    )
    .unwrap();

    let mut app = test_app();
    app.world.run_system_once(discover_modules);
    let module_count = {
        let registry = app.world.resource::<ModuleRegistry>();
        assert!(registry.modules.len() >= 1);
        registry.modules.len()
    };
    app.world.run_system_once(setup_lobby);

    let pad_count = app.world.query::<&LobbyPad>().iter(&app.world).count();
    assert_eq!(pad_count, module_count);

    fs::remove_dir_all(manifest_dir).unwrap();
}

#[test]
fn spawns_help_pads_when_registry_empty() {
    let mut app = test_app();
    app.world.run_system_once(setup_lobby);

    let pads: Vec<_> = app.world.query::<&DocPad>().iter(&app.world).collect();
    assert_eq!(pads.len(), 5);
    let expected = [
        "docs/netcode.md",
        "docs/modules.md",
        "docs/DuckHunt.md",
        "docs/ops.md",
        "docs/Email.md",
    ];
    for url in expected {
        assert!(
            pads.iter().any(|pad| pad.url == url),
            "missing pad for {url}"
        );
    }

    let signage_present = app.world.query::<&Text>().iter(&app.world).any(|t| {
        t.sections
            .iter()
            .any(|s| s.value == "No modules installed – see Docs pads for setup instructions")
    });
    assert!(signage_present, "missing no-modules signage");
}

#[test]
fn setup_lobby_handles_missing_window() {
    let mut app = app_without_window();
    app.world.run_system_once(setup_lobby);

    // no entities should be spawned without a window
    assert_eq!(app.world.iter_entities().count(), 0);
}

#[test]
fn lobby_spawns_leaderboard_and_pedestal() {
    let mut app = test_app();
    app.world.run_system_once(setup_lobby);
    let screens = app.world.query::<&LeaderboardScreen>().iter(&app.world).count();
    let pedestals = app.world.query::<&ReplayPedestal>().iter(&app.world).count();
    assert_eq!(screens, 1);
    assert_eq!(pedestals, 1);
}
