use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use engine::{
    discover_modules,
    hotload_modules,
    setup_lobby,
    update_lobby_pads,
    LobbyPad,
    ModuleRegistry,
};
use platform_api::AppState;
use std::fs;
use std::path::Path;

fn test_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_state::<AppState>();
    app.init_resource::<ModuleRegistry>();
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.world.spawn(Window::default());
    app
}

#[test]
fn hotloads_module_manifest_changes() {
    let base = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../assets/modules");
    let backup = base.join("backup");
    if backup.exists() {
        fs::remove_dir_all(&backup).unwrap();
    }
    fs::create_dir_all(&backup).unwrap();
    for entry in fs::read_dir(&base).unwrap() {
        let entry = entry.unwrap();
        if entry.file_name() == "backup" { continue; }
        if entry.file_type().unwrap().is_dir() {
            let name = entry.file_name();
            fs::rename(entry.path(), backup.join(name)).unwrap();
        }
    }
    let mod1 = base.join("hotload_one");
    fs::create_dir_all(&mod1).unwrap();
    fs::write(
        mod1.join("module.toml"),
        r#"id = "m1"
name = "Mod One"
version = "1.0.0"
author = "Test"
state = "DuckHunt"
capabilities = ["LOBBY_PAD"]
"#,
    )
    .unwrap();

    let mut app = test_app();
    app.world.run_system_once(discover_modules);
    hotload_modules(&mut app);
    app.world.run_system_once(setup_lobby);

    let mut pad_query = app.world.query::<&LobbyPad>();
    assert_eq!(pad_query.iter(&app.world).count(), 1);

    // add second module
    let mod2 = base.join("hotload_two");
    fs::create_dir_all(&mod2).unwrap();
    fs::write(
        mod2.join("module.toml"),
        r#"id = "m2"
name = "Mod Two"
version = "1.0.0"
author = "Test"
state = "DuckHunt"
capabilities = ["LOBBY_PAD"]
"#,
    )
    .unwrap();
    app.world.run_system_once(discover_modules);
    app.world.run_system_once(update_lobby_pads);
    assert_eq!(pad_query.iter(&app.world).count(), 2);

    // remove second module
    fs::remove_dir_all(&mod2).unwrap();
    app.world.run_system_once(discover_modules);
    app.world.run_system_once(update_lobby_pads);
    assert_eq!(pad_query.iter(&app.world).count(), 1);

    fs::remove_dir_all(mod1).unwrap();
    for entry in fs::read_dir(&backup).unwrap() {
        let entry = entry.unwrap();
        fs::rename(entry.path(), base.join(entry.file_name())).unwrap();
    }
    fs::remove_dir_all(backup).unwrap();
}

