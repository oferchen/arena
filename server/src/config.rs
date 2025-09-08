use axum::Json;
use serde::Serialize;
use std::collections::HashMap;

/// Public configuration returned to clients.
#[derive(Serialize)]
pub struct ConfigResponse {
    /// WebSocket signaling path
    pub signaling_path: String,
    /// Base URL for API requests
    pub api_base_url: String,
    /// Feature flags exposed to the client
    pub feature_flags: HashMap<String, bool>,
}

/// HTTP handler that returns public configuration as JSON.
pub async fn get_config() -> Json<ConfigResponse> {
    let cfg = ConfigResponse {
        signaling_path: "/signal".to_string(),
        api_base_url: "/".to_string(),
        feature_flags: HashMap::new(),
    };
    Json(cfg)
}

