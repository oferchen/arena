#![cfg_attr(target_arch = "wasm32", feature(web_worker))]

use analytics::{Analytics, Event};
use bevy::prelude::*;
use duck_hunt::DuckHuntPlugin;
use engine::{AppExt, EnginePlugin};
mod entitlements;
mod lobby;
mod net;
use entitlements::{fetch_entitlements, ensure_session};
use null_module::NullModule;
use payments::{EntitlementStore, UserId};
use physics::PhysicsPlugin;
use render::RenderPlugin;

#[cfg(all(target_arch = "wasm32", not(target_os = "emscripten")))]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let enabled = std::env::var("ARENA_ANALYTICS_OPT_OUT").is_err();
    let analytics = Analytics::new(enabled, None, false);
    analytics.dispatch(Event::SessionStart);
    analytics.dispatch(Event::LevelStart { level: 1 });
    let entitlements = EntitlementStore::default();
    let user = ensure_session();
    for sku in fetch_entitlements().unwrap_or_default() {
        entitlements.grant(user, sku);
    }
    let _ = entitlements.has(user, "basic");
    analytics.dispatch(Event::EntitlementChecked);

    // Initialize the Bevy application
    let mut app = App::new();
    app.insert_resource(Time::<Fixed>::from_seconds(1.0 / 60.0));
    app.insert_resource(analytics.clone());
    app.add_plugins(RenderPlugin)
        .add_plugins(PhysicsPlugin)
        .add_plugins(EnginePlugin)
        .add_plugins(net::ClientNetPlugin)
        .add_plugins(lobby::LobbyPlugin);
    if entitlements.has(user, "duck_hunt") {
        app.add_game_module::<DuckHuntPlugin>();
    }
    app.add_game_module::<NullModule>().run();
}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub async fn main() -> Result<(), JsValue> {
    let enabled = std::env::var("ARENA_ANALYTICS_OPT_OUT").is_err();
    let analytics = Analytics::new(enabled, None, false);
    analytics.dispatch(Event::SessionStart);
    analytics.dispatch(Event::LevelStart { level: 1 });
    let entitlements = EntitlementStore::default();
    let user = ensure_session().await?;
    for sku in fetch_entitlements().await.unwrap_or_default() {
        entitlements.grant(user, sku);
    }
    let _ = entitlements.has(user, "basic");
    analytics.dispatch(Event::EntitlementChecked);

    // Initialize the Bevy application
    let mut app = App::new();
    app.insert_resource(Time::<Fixed>::from_seconds(1.0 / 60.0));
    app.insert_resource(analytics.clone());
    app.add_plugins(RenderPlugin)
        .add_plugins(PhysicsPlugin)
        .add_plugins(EnginePlugin)
        .add_plugins(net::ClientNetPlugin)
        .add_plugins(lobby::LobbyPlugin);
    if entitlements.has(user, "duck_hunt") {
        app.add_game_module::<DuckHuntPlugin>();
    }
    app.add_game_module::<NullModule>().run();
    Ok(())
}
