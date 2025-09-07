use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use engine::{ModuleRegistry, discover_modules};
use log::Level;
use logtest::Logger;
use platform_api::AppState;
use std::fs;
use std::path::Path;

fn test_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_state::<AppState>();
    app.init_resource::<ModuleRegistry>();
    app
}

#[test]
fn skips_modules_with_invalid_state() {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../assets/modules");
    let backup = base.join("backup");
    if backup.exists() {
        fs::remove_dir_all(&backup).unwrap();
    }
    fs::create_dir_all(&backup).unwrap();
    for entry in fs::read_dir(&base).unwrap() {
        let entry = entry.unwrap();
        if entry.file_name() == "backup" {
            continue;
        }
        if entry.file_type().unwrap().is_dir() {
            let name = entry.file_name();
            fs::rename(entry.path(), backup.join(name)).unwrap();
        }
    }

    let invalid = base.join("invalid_state");
    fs::create_dir_all(&invalid).unwrap();
    fs::write(
        invalid.join("module.toml"),
        r#"id = "bad"
name = "Bad State"
version = "1.0.0"
author = "Test"
state = "Unknown"
capabilities = []
"#,
    )
    .unwrap();

    let mut logger = Logger::start();
    let mut app = test_app();
    app.world.run_system_once(discover_modules);

    let registry = app.world.resource::<ModuleRegistry>();
    assert_eq!(registry.modules.len(), 0);
    assert!(logger.any(|r| r.level() == Level::Error && r.args().contains("unknown module state")));

    fs::remove_dir_all(&invalid).unwrap();
    for entry in fs::read_dir(&backup).unwrap() {
        let entry = entry.unwrap();
        fs::rename(entry.path(), base.join(entry.file_name())).unwrap();
    }
    fs::remove_dir_all(backup).unwrap();
}
