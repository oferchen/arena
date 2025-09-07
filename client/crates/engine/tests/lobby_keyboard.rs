use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use engine::{ModuleRegistry, lobby_keyboard};
use platform_api::{AppState, CapabilityFlags, ModuleMetadata};

#[test]
fn lobby_keyboard_supports_more_than_five_modules() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_state::<AppState>();
    app.insert_resource(Input::<KeyCode>::default());

    let mut registry = ModuleRegistry::default();
    for i in 0..6 {
        registry.modules.push(ModuleMetadata {
            id: format!("m{}", i),
            name: format!("Mod {}", i),
            version: "1.0.0".into(),
            author: "Test".into(),
            state: if i == 5 {
                AppState::DuckHunt
            } else {
                AppState::Lobby
            },
            capabilities: CapabilityFlags::empty(),
            max_players: 0,
            icon: Handle::default(),
        });
    }
    app.insert_resource(registry);

    {
        let mut input = app.world.resource_mut::<Input<KeyCode>>();
        input.press(KeyCode::Key6);
    }
    app.world.run_system_once(lobby_keyboard);

    let next_state = app.world.resource::<NextState<AppState>>();
    assert_eq!(next_state.0, Some(AppState::DuckHunt));
}
