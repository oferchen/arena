use payments::{EntitlementList, UserId};

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
pub fn ensure_session() -> UserId {
    user_id()
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
#[derive(serde::Deserialize)]
struct GuestSession {
    user_id: String,
    token: String,
}

#[cfg(target_arch = "wasm32")]
pub async fn ensure_session() -> Result<UserId, wasm_bindgen::JsValue> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::Response;
    use serde_wasm_bindgen::from_value;
    use wasm_bindgen::JsValue;

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
    let resp_value = JsFuture::from(window.fetch_with_str("/auth/guest")).await?;
    let resp: Response = resp_value.dyn_into()?;
    let json = JsFuture::from(resp.json()?).await?;
    let guest: GuestSession = from_value(json)?;
    storage.set_item("user_id", &guest.user_id)?;
    storage.set_item("session_token", &guest.token)?;
    UserId::parse_str(&guest.user_id).map_err(|_| JsValue::from_str("invalid user"))
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
