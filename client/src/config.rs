use bevy::prelude::*;

#[derive(Resource, Clone)]
pub struct RuntimeConfig {
    pub signal_url: String,
    pub api_base_url: String,
}

impl RuntimeConfig {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load() -> Self {
        Self {
            signal_url: std::env::var("ARENA_SIGNAL_URL")
                .unwrap_or_else(|_| "ws://localhost:3000/signal".to_string()),
            api_base_url: std::env::var("ARENA_API_URL")
                .unwrap_or_else(|_| "http://localhost:3000".to_string()),
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn load() -> Self {
        Self {
            signal_url: std::env::var("ARENA_SIGNAL_URL")
                .unwrap_or_else(|_| "ws://localhost:3000/signal".to_string()),
            api_base_url: std::env::var("ARENA_API_URL")
                .unwrap_or_else(|_| "http://localhost:3000".to_string()),
        }
    }
}

