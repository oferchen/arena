use payments::EntitlementList;

#[cfg(not(target_arch = "wasm32"))]
pub fn fetch_entitlements() -> Result<Vec<String>, reqwest::Error> {
    reqwest::blocking::get("http://localhost:3000/entitlements/local")
        .and_then(|r| r.json::<EntitlementList>())
        .map(|e| e.entitlements)
}

#[cfg(target_arch = "wasm32")]
pub async fn fetch_entitlements() -> Result<Vec<String>, wasm_bindgen::JsValue> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::Response;
    use serde_wasm_bindgen::from_value;

    let window = web_sys::window().ok_or_else(|| wasm_bindgen::JsValue::from_str("no window"))?;
    let resp_value = JsFuture::from(window.fetch_with_str("/entitlements"))
        .await?;
    let resp: Response = resp_value.dyn_into()?;
    let json = JsFuture::from(resp.json()?).await?;
    let list: EntitlementList = from_value(json)?;
    Ok(list.entitlements)
}
