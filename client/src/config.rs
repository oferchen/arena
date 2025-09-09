use bevy::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Resource, Clone, Deserialize)]
pub struct RuntimeConfig {
    pub signal_url: String,
    pub api_base_url: String,
    #[serde(default)]
    pub feature_flags: HashMap<String, bool>,
    #[serde(default)]
    pub ice_servers: Vec<String>,
    #[serde(default)]
    pub analytics_enabled: bool,
    #[serde(default)]
    pub analytics_opt_out: bool,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            signal_url: String::new(),
            api_base_url: String::new(),
            feature_flags: HashMap::new(),
            ice_servers: Vec::new(),
            analytics_enabled: false,
            analytics_opt_out: false,
        }
    }
}

impl RuntimeConfig {
    #[cfg(target_arch = "wasm32")]
    pub async fn load() -> Self {
        use wasm_bindgen::JsCast;
        use wasm_bindgen_futures::JsFuture;
        use web_sys::window;

        let window = match window() {
            Some(win) => win,
            None => return Self::default(),
        };

        let resp_value = match JsFuture::from(window.fetch_with_str("/config.json")).await {
            Ok(v) => v,
            Err(e) => {
                web_sys::console::error_1(&e);
                return Self::default();
            }
        };

        let resp: web_sys::Response = match resp_value.dyn_into() {
            Ok(r) => r,
            Err(e) => {
                web_sys::console::error_1(&e);
                return Self::default();
            }
        };

        let text_promise = match resp.text() {
            Ok(promise) => promise,
            Err(e) => {
                web_sys::console::error_1(&e);
                return Self::default();
            }
        };

        let text = match JsFuture::from(text_promise).await {
            Ok(v) => v.as_string().unwrap_or_default(),
            Err(e) => {
                web_sys::console::error_1(&e);
                return Self::default();
            }
        };

        serde_json::from_str(&text).unwrap_or_else(|err| {
            web_sys::console::error_1(&err.to_string().into());
            Self::default()
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn load() -> Self {
        match reqwest::get("/config.json").await {
            Ok(resp) => match resp.json::<RuntimeConfig>().await {
                Ok(cfg) => cfg,
                Err(err) => {
                    eprintln!("Failed to parse /config.json: {err}");
                    Self::load_sync()
                }
            },
            Err(err) => {
                eprintln!("Failed to fetch /config.json: {err}");
                Self::load_sync()
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_sync() -> Self {
        match std::fs::read_to_string("config.json") {
            Ok(contents) => serde_json::from_str(&contents).unwrap_or_else(|err| {
                eprintln!("Failed to parse config.json: {err}");
                Self::default()
            }),
            Err(err) => {
                eprintln!("Failed to read config.json: {err}");
                Self::default()
            }
        }
    }
}
