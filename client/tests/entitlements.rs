#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::*;
use client::entitlements::{fetch_entitlements, user_id};

wasm_bindgen_test_configure!(run_in_node);

#[wasm_bindgen(module = "/tests/entitlements.js")]
extern "C" {
    fn mock_entitlements(expected: &str);
}

#[wasm_bindgen_test]
async fn loads_mocked_entitlements() {
    let user = user_id().unwrap();
    let base_url = "";
    mock_entitlements(&format!("{base_url}/entitlements/{user}"));
    let ents = fetch_entitlements(base_url).await.unwrap();
    assert_eq!(ents, vec!["duck_hunt".to_string()]);
}
