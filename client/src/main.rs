use bevy::prelude::*;
use duck_hunt::DuckHuntPlugin;
use engine::{AppExt, EnginePlugin};
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use reqwest::Client;
use serde_json::json;
use wasm_bindgen_futures::spawn_local;

const AUTH_BASE_URL: &str = "http://localhost:8000/auth";

#[derive(Resource, Clone)]
struct SessionClient {
    client: Client,
}

#[derive(Resource, Default)]
struct Forms {
    register_email: String,
    register_password: String,
    login_email: String,
    login_password: String,
    twofa_code: String,
}

#[derive(Component, PartialEq, Eq, Clone, Copy)]
enum Kiosk {
    Register,
    Login,
    TwoFA,
}

#[derive(Resource, Default)]
struct ActiveKiosk(Option<Kiosk>);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EnginePlugin)
        .add_minigame::<DuckHuntPlugin>()
        .run();
}
