use axum::{Json, extract::Extension};
use serde::Serialize;
use std::collections::HashMap;

use crate::{IceServerConfig, ResolvedConfig};

/// Public configuration returned to clients.
#[derive(Serialize)]
pub struct ConfigResponse {
    /// WebSocket signaling URL
    pub signal_url: String,
    /// Base URL for API requests
    pub api_base_url: String,
    /// Whether analytics collection is enabled
    #[serde(default)]
    pub analytics_enabled: bool,
    /// Whether analytics collection is opted out
    #[serde(default)]
    pub analytics_opt_out: bool,
    /// Feature flags exposed to the client
    pub feature_flags: HashMap<String, bool>,
    /// ICE servers used for establishing peer connections
    #[serde(default)]
    pub ice_servers: Vec<IceServerConfig>,
}

/// HTTP handler that returns public configuration as JSON.
pub async fn get_config(Extension(cfg): Extension<ResolvedConfig>) -> Json<ConfigResponse> {
    let cfg = ConfigResponse {
        signal_url: cfg.signaling_ws_url.clone(),
        api_base_url: cfg.public_base_url.clone(),
        analytics_enabled: cfg.analytics_enabled,
        analytics_opt_out: cfg.analytics_opt_out,
        feature_flags: cfg.feature_flags.clone(),
        ice_servers: cfg.ice_servers.clone(),
    };
    Json(cfg)
}
