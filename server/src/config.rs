use axum::{extract::Extension, Json};
use serde::Serialize;
use std::collections::HashMap;

use crate::ResolvedConfig;

/// Public configuration returned to clients.
#[derive(Serialize)]
pub struct ConfigResponse {
    /// WebSocket signaling URL
    pub signal_url: String,
    /// Base URL for API requests
    pub api_base_url: String,
    /// Feature flags exposed to the client
    pub feature_flags: HashMap<String, bool>,
    /// ICE servers used for establishing peer connections
    #[serde(default)]
    pub ice_servers: Vec<String>,
}

/// HTTP handler that returns public configuration as JSON.
pub async fn get_config(Extension(cfg): Extension<ResolvedConfig>) -> Json<ConfigResponse> {
    let cfg = ConfigResponse {
        signal_url: cfg.signaling_ws_url.clone(),
        api_base_url: cfg.public_base_url.clone(),
        feature_flags: HashMap::new(),
        ice_servers: Vec::new(),
    };
    Json(cfg)
}

