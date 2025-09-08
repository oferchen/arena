use payments::{EntitlementList, UserId};
use serde::Serialize;

#[cfg(not(target_arch = "wasm32"))]
pub fn user_id() -> UserId {
    use std::sync::OnceLock;

    static USER: OnceLock<UserId> = OnceLock::new();
    *USER.get_or_init(|| {
        std::env::var("ARENA_USER_ID")
            .ok()
            .and_then(|s| UserId::parse_str(&s).ok())
            .unwrap_or_else(UserId::new_v4)
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub fn fetch_entitlements() -> Result<Vec<String>, reqwest::Error> {
    let user = user_id();
    reqwest::blocking::get(format!("http://localhost:3000/entitlements/{user}"))
        .and_then(|r| r.json::<EntitlementList>())
        .map(|e| e.entitlements)
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Serialize)]
struct ClaimRequest<'a> {
    sku: &'a str,
}

#[cfg(not(target_arch = "wasm32"))]
pub fn claim_entitlement(sku: &str) -> Result<(), reqwest::Error> {
    let user = user_id();
    let req = ClaimRequest { sku };
    reqwest::blocking::Client::new()
        .post("http://localhost:3000/store/claim")
        .header("X-Session", user.to_string())
        .json(&req)
        .send()
        .map(|_| ())
}

#[cfg(target_arch = "wasm32")]
pub fn user_id() -> Result<UserId, wasm_bindgen::JsValue> {
    use wasm_bindgen::JsValue;

    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let storage = window
        .local_storage()?
        .ok_or_else(|| JsValue::from_str("no local storage"))?;
    if let Ok(Some(id)) = storage.get_item("user_id") {
        if let Ok(uuid) = UserId::parse_str(&id) {
            return Ok(uuid);
        }
    }
    let id = UserId::new_v4();
    storage.set_item("user_id", &id.to_string())?;
    Ok(id)
}

#[cfg(target_arch = "wasm32")]
pub async fn fetch_entitlements() -> Result<Vec<String>, wasm_bindgen::JsValue> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::Response;
    use serde_wasm_bindgen::from_value;

    let user = user_id()?;
    let window = web_sys::window().ok_or_else(|| wasm_bindgen::JsValue::from_str("no window"))?;
    let resp_value = JsFuture::from(window.fetch_with_str(&format!("/entitlements/{user}")))
        .await?;
    let resp: Response = resp_value.dyn_into()?;
    let json = JsFuture::from(resp.json()?).await?;
    let list: EntitlementList = from_value(json)?;
    Ok(list.entitlements)
}

#[cfg(target_arch = "wasm32")]
#[derive(Serialize)]
struct ClaimRequest<'a> {
    sku: &'a str,
}

#[cfg(target_arch = "wasm32")]
pub async fn claim_entitlement(sku: &str) -> Result<(), wasm_bindgen::JsValue> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen::JsValue;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Request, RequestInit};

    let user = user_id()?;
    let mut opts = RequestInit::new();
    opts.method("POST");
    let body = serde_json::to_string(&ClaimRequest { sku })
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    opts.body(Some(&JsValue::from_str(&body)));
    let request = Request::new_with_str_and_init("/store/claim", &opts)?;
    let headers = request.headers();
    headers.set("Content-Type", "application/json")?;
    headers.set("X-Session", &user.to_string())?;
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    JsFuture::from(window.fetch_with_request(&request)).await?;
    Ok(())
}
