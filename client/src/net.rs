use bevy::prelude::*;
use netcode::client::ClientConnector;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;

#[cfg(target_arch = "wasm32")]
fn start_connection() {
    spawn_local(async move {
        if let Ok(conn) = ClientConnector::new().await {
            let _ = conn.signal("ws://localhost:9001").await;
            std::mem::forget(conn);
        }
    });
}

pub struct ClientNetPlugin;

impl Plugin for ClientNetPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(target_arch = "wasm32")]
        app.add_systems(Startup, start_connection);
    }
}
