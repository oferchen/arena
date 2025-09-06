
use bevy::prelude::*;
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
        .add_plugins(EguiPlugin)
        .insert_resource(SessionClient {
            client: Client::builder().cookie_store(true).build().expect("client"),
        })
        .insert_resource(Forms::default())
        .insert_resource(ActiveKiosk::default())
        .add_systems(Startup, setup)
        .add_systems(Update, (handle_keys, kiosk_ui))
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // light
    commands.spawn(DirectionalLightBundle {
        transform: Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // ground
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size: 10.0, subdivisions: 0 })),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        ..default()
    });

    let kiosk_mesh = meshes.add(Mesh::from(shape::Cube { size: 1.0 }));
    let kiosk_material = materials.add(Color::rgb(0.8, 0.2, 0.2).into());

    commands.spawn((
        PbrBundle {
            mesh: kiosk_mesh.clone(),
            material: kiosk_material.clone(),
            transform: Transform::from_xyz(-3.0, 0.5, 0.0),
            ..default()
        },
        Kiosk::Register,
    ));
    commands.spawn((
        PbrBundle {
            mesh: kiosk_mesh.clone(),
            material: kiosk_material.clone(),
            transform: Transform::from_xyz(0.0, 0.5, 0.0),
            ..default()
        },
        Kiosk::Login,
    ));
    commands.spawn((
        PbrBundle {
            mesh: kiosk_mesh.clone(),
            material: kiosk_material.clone(),
            transform: Transform::from_xyz(3.0, 0.5, 0.0),
            ..default()
        },
        Kiosk::TwoFA,
    ));
}

fn handle_keys(keys: Res<Input<KeyCode>>, mut active: ResMut<ActiveKiosk>) {
    if keys.just_pressed(KeyCode::Key1) {
        active.0 = Some(Kiosk::Register);
    } else if keys.just_pressed(KeyCode::Key2) {
        active.0 = Some(Kiosk::Login);
    } else if keys.just_pressed(KeyCode::Key3) {
        active.0 = Some(Kiosk::TwoFA);
    }
}

fn kiosk_ui(
    mut contexts: EguiContexts,
    mut active: ResMut<ActiveKiosk>,
    mut forms: ResMut<Forms>,
    client: Res<SessionClient>,
) {
    if let Some(kiosk) = active.0 {
        let ctx = contexts.ctx_mut();
        let mut close = false;
        match kiosk {
            Kiosk::Register => {
                egui::Window::new("Register").show(ctx, |ui| {
                    ui.label("Email");
                    ui.text_edit_singleline(&mut forms.register_email);
                    ui.label("Password");
                    ui.text_edit_singleline(&mut forms.register_password);
                    if ui.button("Submit").clicked() {
                        let client = client.client.clone();
                        let email = forms.register_email.clone();
                        let password = forms.register_password.clone();
                        spawn_local(send_register(client, email, password));
                        close = true;
                    }
                    if ui.button("Close").clicked() {
                        close = true;
                    }
                });
            }
            Kiosk::Login => {
                egui::Window::new("Login").show(ctx, |ui| {
                    ui.label("Email");
                    ui.text_edit_singleline(&mut forms.login_email);
                    ui.label("Password");
                    ui.text_edit_singleline(&mut forms.login_password);
                    if ui.button("Submit").clicked() {
                        let client = client.client.clone();
                        let email = forms.login_email.clone();
                        let password = forms.login_password.clone();
                        spawn_local(send_login(client, email, password));
                        close = true;
                    }
                    if ui.button("Close").clicked() {
                        close = true;
                    }
                });
            }
            Kiosk::TwoFA => {
                egui::Window::new("2FA").show(ctx, |ui| {
                    ui.label("Code");
                    ui.text_edit_singleline(&mut forms.twofa_code);
                    if ui.button("Submit").clicked() {
                        let client = client.client.clone();
                        let code = forms.twofa_code.clone();
                        spawn_local(send_twofa(client, code));
                        close = true;
                    }
                    if ui.button("Close").clicked() {
                        close = true;
                    }
                });
            }
        }
        if close {
            active.0 = None;
        }
    }
}

async fn send_register(client: Client, email: String, password: String) {
    let _ = client
        .post(format!("{}/register", AUTH_BASE_URL))
        .json(&json!({ "email": email, "password": password }))
        .send()
        .await;
}

async fn send_login(client: Client, email: String, password: String) {
    let _ = client
        .post(format!("{}/login", AUTH_BASE_URL))
        .json(&json!({ "email": email, "password": password }))
        .send()
        .await;
}

async fn send_twofa(client: Client, code: String) {
    let _ = client
        .post(format!("{}/2fa", AUTH_BASE_URL))
        .json(&json!({ "code": code }))
        .send()
        .await;
}
