#![cfg(target_arch = "wasm32")]

use editor::{client::EditorClient, level::Level};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn save_and_load_level() {
    let client = EditorClient::new();
    let level = Level::new("t1", "Test Level");
    client.store_level_locally(&level).await.unwrap();

    let loaded = client
        .load_level_locally("t1")
        .await
        .unwrap()
        .expect("level loaded");
    assert_eq!(loaded.name, "Test Level");
}

#[wasm_bindgen_test]
async fn load_missing_level_returns_none() {
    let client = EditorClient::new();
    assert!(client.load_level_locally("missing").await.unwrap().is_none());
}

#[wasm_bindgen_test]
async fn upgrade_does_not_clear_data() {
    let client = EditorClient::new();
    let level = Level::new("up", "Upgrade Test");
    client.store_level_locally(&level).await.unwrap();
    // Saving again should succeed and preserve data (upgrade path).
    client.store_level_locally(&level).await.unwrap();
    let loaded = client
        .load_level_locally("up")
        .await
        .unwrap()
        .expect("level");
    assert_eq!(loaded.name, "Upgrade Test");
}

