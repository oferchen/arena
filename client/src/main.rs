use bevy::prelude::*;
use duck_hunt::DuckHuntPlugin;
use engine::{AppExt, EnginePlugin};
use null_module::NullModule;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EnginePlugin)
        .add_game_module::<DuckHuntPlugin>()
        .add_game_module::<NullModule>()
        .run();
}
