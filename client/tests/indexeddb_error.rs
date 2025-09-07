#![cfg(target_arch = "wasm32")]

use editor::{client::EditorClient, level::Level};
use wasm_bindgen::prelude::*;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_node);

#[wasm_bindgen(module = "/tests/indexeddb_error.js")]
extern "C" {
    fn setup_indexeddb_error();
}

#[wasm_bindgen_test]
async fn store_level_fails_without_indexeddb() {
    setup_indexeddb_error();
    let client = EditorClient::new();
    let level = Level::new("e1", "Err");
    assert!(client.store_level_locally(&level).await.is_err());
}

#[wasm_bindgen_test]
async fn load_level_fails_without_indexeddb() {
    setup_indexeddb_error();
    let client = EditorClient::new();
    assert!(client.load_level_locally("e1").await.is_err());
}
