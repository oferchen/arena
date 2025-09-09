use purchases::{EntitlementList, UserId};
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
pub fn fetch_entitlements(base_url: &str) -> Result<Vec<String>, reqwest::Error> {
    if base_url.is_empty() {
        return Ok(Vec::new());
    }
    let user = user_id();
    reqwest::blocking::get(format!("{base_url}/entitlements/{user}"))
        .and_then(|r| r.json::<EntitlementList>())
        .map(|e| e.entitlements)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn ensure_session() -> UserId {
    user_id()
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Serialize)]
struct Claim<'a> {
    sku: &'a str,
}

#[cfg(not(target_arch = "wasm32"))]
pub fn claim_entitlement(base_url: &str, sku: &str) -> Result<(), reqwest::Error> {
    if base_url.is_empty() {
        return Ok(());
    }
    let user = user_id();
    let client = reqwest::blocking::Client::new();
    client
        .post(format!("{base_url}/store/claim"))
        .header("X-Session", user.to_string())
        .json(&Claim { sku })
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
pub async fn fetch_entitlements(base_url: &str) -> Result<Vec<String>, wasm_bindgen::JsValue> {
    use serde_wasm_bindgen::from_value;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::Response;

    let user = user_id()?;
    let window = web_sys::window().ok_or_else(|| wasm_bindgen::JsValue::from_str("no window"))?;
    let url = format!("{base_url}/entitlements/{user}");
    let resp_value = JsFuture::from(window.fetch_with_str(&url)).await?;
    let resp: Response = resp_value.dyn_into()?;
    let json = JsFuture::from(resp.json()?).await?;
    let list: EntitlementList = from_value(json)?;
    Ok(list.entitlements)
}

#[cfg(target_arch = "wasm32")]
#[derive(serde::Deserialize)]
struct GuestSession {
    user_id: String,
    token: String,
}

#[cfg(target_arch = "wasm32")]
pub async fn ensure_session(base_url: &str) -> Result<UserId, wasm_bindgen::JsValue> {
    use serde_wasm_bindgen::from_value;
    use wasm_bindgen::JsCast;
    use wasm_bindgen::JsValue;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::Response;

    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let storage = window
        .local_storage()?
        .ok_or_else(|| JsValue::from_str("no local storage"))?;
    if let Ok(Some(id)) = storage.get_item("user_id") {
        if let Ok(Some(_token)) = storage.get_item("session_token") {
            if let Ok(uuid) = UserId::parse_str(&id) {
                return Ok(uuid);
            }
        }
    }
    let url = format!("{base_url}/auth/guest");
    let resp_value = JsFuture::from(window.fetch_with_str(&url)).await?;
    let resp: Response = resp_value.dyn_into()?;
    let json = JsFuture::from(resp.json()?).await?;
    let guest: GuestSession = from_value(json)?;
    storage.set_item("user_id", &guest.user_id)?;
    storage.set_item("session_token", &guest.token)?;
    UserId::parse_str(&guest.user_id).map_err(|_| JsValue::from_str("invalid user"))
}

#[cfg(target_arch = "wasm32")]
#[derive(Serialize)]
struct Claim<'a> {
    sku: &'a str,
}

#[cfg(target_arch = "wasm32")]
pub async fn claim_entitlement(base_url: &str, sku: &str) -> Result<(), wasm_bindgen::JsValue> {
    use serde_wasm_bindgen::to_value;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Request, RequestInit, RequestMode, Response};
    let window = web_sys::window().ok_or_else(|| wasm_bindgen::JsValue::from_str("no window"))?;
    let storage = window
        .local_storage()?
        .ok_or_else(|| wasm_bindgen::JsValue::from_str("no local storage"))?;
    let token = storage
        .get_item("session_token")?
        .ok_or_else(|| wasm_bindgen::JsValue::from_str("no session"))?;
    let mut opts = RequestInit::new();
    opts.method("POST");
    opts.mode(RequestMode::Cors);
    opts.body(Some(&to_value(&Claim { sku })?));
    let request = Request::new_with_str_and_init(&format!("{base_url}/store/claim"), &opts)?;
    request.headers().set("content-type", "application/json")?;
    request.headers().set("X-Session", &token)?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
    let _resp: Response = resp_value.dyn_into()?;
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub fn upgrade(user_id: &str, token: &str) -> Result<(), wasm_bindgen::JsValue> {
    use wasm_bindgen::JsValue;
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let storage = window
        .local_storage()?
        .ok_or_else(|| JsValue::from_str("no local storage"))?;
    storage.set_item("user_id", user_id)?;
    storage.set_item("session_token", token)?;
    Ok(())
}
