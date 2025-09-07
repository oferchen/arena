#![cfg_attr(target_arch = "wasm32", feature(web_worker))]

use bevy::prelude::*;
use duck_hunt::DuckHuntPlugin;
use engine::{AppExt, EnginePlugin};
use null_module::NullModule;
use physics::PhysicsPlugin;

#[cfg(all(target_arch = "wasm32", not(target_os = "emscripten")))]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(PhysicsPlugin)
        .add_plugins(EnginePlugin)
        .add_game_module::<DuckHuntPlugin>()
        .add_game_module::<NullModule>()
        .run();
}
