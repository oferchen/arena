#![cfg_attr(target_arch = "wasm32", feature(web_worker))]

use analytics::{Analytics, Event};
use bevy::prelude::*;
use duck_hunt::DuckHuntPlugin;
use engine::{AppExt, EnginePlugin};
mod entitlements;
mod lobby;
mod net;
mod config;
use entitlements::{claim_entitlement, fetch_entitlements, ensure_session};
use config::RuntimeConfig;
use null_module::NullModule;
use std::collections::HashSet;
use physics::PhysicsPlugin;
use render::RenderPlugin;
use futures_lite::future;

#[cfg(all(target_arch = "wasm32", not(target_os = "emscripten")))]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let config = future::block_on(RuntimeConfig::load());
    let enabled = config.analytics_enabled && !config.analytics_opt_out;
    let analytics = Analytics::new(enabled, None, None);
    analytics.dispatch(Event::SessionStart);
    analytics.dispatch(Event::LevelStart { level: 1 });
    let mut entitlements: HashSet<String> =
        fetch_entitlements(&config.api_base_url).unwrap_or_default().into_iter().collect();
    let _ = ensure_session();
    let _ = claim_entitlement(&config.api_base_url, "basic");
    let _ = entitlements.contains("basic");
    analytics.dispatch(Event::EntitlementChecked);

    // Initialize the Bevy application
    let mut app = App::new();
    app.insert_resource(Time::<Fixed>::from_seconds(1.0 / 60.0));
    app.insert_resource(analytics.clone());
    app.insert_resource(config.clone());
    app.add_plugins(RenderPlugin)
        .add_plugins(PhysicsPlugin)
        .add_plugins(EnginePlugin)
        .add_plugins(net::ClientNetPlugin)
        .add_plugins(lobby::LobbyPlugin);
    if entitlements.contains("duck_hunt") {
        app.add_game_module::<DuckHuntPlugin>();
    }
    app.add_game_module::<NullModule>().run();
}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub async fn main() -> Result<(), JsValue> {
    let config = RuntimeConfig::load().await;
    let enabled = config.analytics_enabled && !config.analytics_opt_out;
    let analytics = Analytics::new(enabled, None, None);
    analytics.dispatch(Event::SessionStart);
    analytics.dispatch(Event::LevelStart { level: 1 });
    let mut entitlements: HashSet<String> =
        fetch_entitlements(&config.api_base_url).await.unwrap_or_default().into_iter().collect();
    let _user = ensure_session(&config.api_base_url).await?;
    let _ = claim_entitlement(&config.api_base_url, "basic").await;
    let _ = entitlements.contains("basic");
    analytics.dispatch(Event::EntitlementChecked);

    // Initialize the Bevy application
    let mut app = App::new();
    app.insert_resource(Time::<Fixed>::from_seconds(1.0 / 60.0));
    app.insert_resource(analytics.clone());
    app.insert_resource(config.clone());
    app.add_plugins(RenderPlugin)
        .add_plugins(PhysicsPlugin)
        .add_plugins(EnginePlugin)
        .add_plugins(net::ClientNetPlugin)
        .add_plugins(lobby::LobbyPlugin);
    if entitlements.contains("duck_hunt") {
        app.add_game_module::<DuckHuntPlugin>();
    }
    app.add_game_module::<NullModule>().run();
    Ok(())
}
