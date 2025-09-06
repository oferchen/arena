use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use engine::{discover_modules, setup_lobby, LobbyPad, ModuleRegistry};
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

#[test]
fn spawns_pads_for_modules() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../assets/modules/test_mod");
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
    {
        let registry = app.world.resource::<ModuleRegistry>();
        assert_eq!(registry.modules.len(), 1);
    }
    app.world.run_system_once(setup_lobby);

    let pad_count = app.world.query::<&LobbyPad>().iter(&app.world).count();
    assert_eq!(pad_count, 1);

    fs::remove_dir_all(manifest_dir).unwrap();
}

#[test]
fn shows_empty_state_when_registry_empty() {
    let mut app = test_app();
    app.world.run_system_once(setup_lobby);

    let mut texts = app.world.query::<&Text>();
    let mut found_msg = false;
    let mut found_link = false;
    for text in texts.iter(&app.world) {
        for section in &text.sections {
            if section.value.contains("No modules installed") {
                found_msg = true;
            }
            if section.value.contains("docs/modules.md") {
                found_link = true;
            }
        }
    }
    assert!(found_msg, "missing empty-state message");
    assert!(found_link, "missing docs link");
}
