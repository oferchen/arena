#![cfg(target_arch = "wasm32")]

use bevy::prelude::*;
use engine::{ModuleRegistry, hotload_modules};
use gloo_timers::future::TimeoutFuture;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn module_discovery_loop_runs() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<ModuleRegistry>();

    hotload_modules(&mut app);

    TimeoutFuture::new(1100).await;
    app.update();

    let registry = app.world.resource::<ModuleRegistry>();
    assert_eq!(registry.modules.len(), 0);
}
