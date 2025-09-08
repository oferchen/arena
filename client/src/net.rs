use bevy::prelude::*;
#[cfg(target_arch = "wasm32")]
use bevy::tasks::{AsyncComputeTaskPool, Task};
use netcode::client::{ClientConnector, ConnectionEvent};
use platform_api::AppState;

#[cfg(target_arch = "wasm32")]
use futures_lite::future;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;

#[cfg(target_arch = "wasm32")]
#[derive(Resource, Default)]
struct ConnectorResource(Option<ClientConnector>);

#[cfg(target_arch = "wasm32")]
#[derive(Resource)]
struct ConnectorTask(Task<Result<ClientConnector, String>>);

#[cfg(target_arch = "wasm32")]
fn start_connection(mut commands: Commands) {
    let task = AsyncComputeTaskPool::get().spawn_local(async move {
        match ClientConnector::new().await {
            Ok(conn) => match conn.signal("ws://localhost:9001").await {
                Ok(_) => Ok(conn),
                Err(e) => Err(e.to_string()),
            },
            Err(e) => Err(e.to_string()),
        }
    });
    commands.insert_resource(ConnectorTask(task));
}

#[cfg(target_arch = "wasm32")]
fn finish_connection_task(
    mut commands: Commands,
    mut task: Option<ResMut<ConnectorTask>>,
    mut events: EventWriter<ConnectionEvent>,
) {
    if let Some(mut task) = task {
        if let Some(result) = future::block_on(future::poll_once(&mut task.0)) {
            match result {
                Ok(conn) => {
                    commands.insert_resource(ConnectorResource(Some(conn)));
                }
                Err(e) => {
                    events.send(ConnectionEvent::Error(e));
                }
            }
            commands.remove_resource::<ConnectorTask>();
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn close_connector(connector: &mut ResMut<ConnectorResource>) {
    if let Some(conn) = connector.0.take() {
        spawn_local(async move {
            if let Err(e) = conn.close().await {
                bevy::log::error!("failed to close connection: {e}");
            }
        });
    }
}

#[cfg(target_arch = "wasm32")]
fn cleanup_on_exit(mut exit: EventReader<AppExit>, mut connector: ResMut<ConnectorResource>) {
    if exit.read().next().is_some() {
        close_connector(&mut connector);
    }
}

#[cfg(target_arch = "wasm32")]
fn cleanup_on_state_change(
    mut events: EventReader<StateTransitionEvent<AppState>>,
    mut connector: ResMut<ConnectorResource>,
) {
    if events.read().next().is_some() {
        close_connector(&mut connector);
    }
}

pub struct ClientNetPlugin;

impl Plugin for ClientNetPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(target_arch = "wasm32")]
        app.add_systems(Startup, start_connection)
            .add_systems(Update, finish_connection_task)
            .add_systems(Update, (cleanup_on_exit, cleanup_on_state_change));
    }
}
