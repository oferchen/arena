use anyhow::Result;
use bevy::prelude::*;
use platform_api::{
    AppState, CapabilityFlags, GameModule, ModuleContext, ModuleMetadata, ServerApp,
};

#[derive(Default)]
pub struct NullModule;

impl Plugin for NullModule {
    fn build(&self, _app: &mut App) {}
}

impl GameModule for NullModule {
    const ID: &'static str = "null_module";

    fn metadata() -> ModuleMetadata {
        ModuleMetadata {
            id: Self::ID.to_string(),
            name: "Null Module".to_string(),
            version: "0.1.0".to_string(),
            author: "Unknown".to_string(),
            state: AppState::Lobby,
            capabilities: CapabilityFlags::empty(),
            max_players: 1,
            icon: Handle::default(),
        }
    }

    fn register(_app: &mut App) {}

    fn enter(_ctx: &mut ModuleContext) -> Result<()> {
        Ok(())
    }

    fn exit(_ctx: &mut ModuleContext) -> Result<()> {
        Ok(())
    }

    fn server_register(_app: &mut ServerApp) {}
}
