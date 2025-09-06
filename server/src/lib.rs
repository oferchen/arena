pub mod email;

use bevy::prelude::*;
use platform_api::{GameModule, ModuleContext, ServerApp};

pub fn register_module<M: GameModule + Default + 'static>(app: &mut ServerApp) {
    let state = M::metadata().state.clone();
    M::server_register(app);
    app.add_plugins(M::default());
    app.add_systems(OnEnter(state.clone()), enter_module::<M>);
    app.add_systems(OnExit(state), exit_module::<M>);
}

fn enter_module<M: GameModule>(world: &mut World) {
    let mut ctx = ModuleContext::new(world);
    M::enter(&mut ctx).expect("module enter failed");
}

fn exit_module<M: GameModule>(world: &mut World) {
    let mut ctx = ModuleContext::new(world);
    M::exit(&mut ctx).expect("module exit failed");
}

pub trait ServerAppExt {
    fn add_game_module<M: GameModule + Default + 'static>(&mut self) -> &mut Self;
}

impl ServerAppExt for ServerApp {
    fn add_game_module<M: GameModule + Default + 'static>(&mut self) -> &mut Self {
        register_module::<M>(self);
        self
    }
}
