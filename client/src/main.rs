#![cfg_attr(target_arch = "wasm32", feature(web_worker))]

use bevy::prelude::*;
use duck_hunt::DuckHuntPlugin;
use engine::{AppExt, EnginePlugin};
use null_module::NullModule;
use physics::PhysicsPlugin;
use render::RenderPlugin;
use payments::EntitlementList;

#[cfg(all(target_arch = "wasm32", not(target_os = "emscripten")))]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[cfg(not(target_arch = "wasm32"))]
fn fetch_entitlements() -> Vec<String> {
    reqwest::blocking::get("http://localhost:3000/entitlements/local")
        .ok()
        .and_then(|r| r.json::<EntitlementList>().ok())
        .map(|e| e.entitlements)
        .unwrap_or_default()
}

#[cfg(target_arch = "wasm32")]
fn fetch_entitlements() -> Vec<String> {
    Vec::new()
}

fn main() {
    let entitlements = fetch_entitlements();
    let mut app = App::new();
    app.add_plugins(RenderPlugin)
        .add_plugins(PhysicsPlugin)
        .add_plugins(EnginePlugin);
    if entitlements.contains(&"duck_hunt".to_string()) {
        app.add_game_module::<DuckHuntPlugin>();
    }
    app.add_game_module::<NullModule>().run();
}
