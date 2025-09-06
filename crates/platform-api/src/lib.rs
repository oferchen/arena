use bevy::prelude::*;
use bitflags::bitflags;

#[derive(States, Default, Clone, Eq, PartialEq, Hash, Debug)]
pub enum AppState {
    #[default]
    Lobby,
    DuckHunt,
}

bitflags! {
    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct CapabilityFlags: u32 {
        const LOBBY_PAD = 0b0001;
    }
}

/// Describes a game module and its capabilities.
#[derive(Clone)]
pub struct ModuleMetadata {
    /// Unique string identifier for the module.
    pub id: &'static str,
    /// Human-readable name shown to players.
    pub name: &'static str,
    /// Semver-style version string.
    pub version: &'static str,
    /// Name of the author or organization.
    pub author: &'static str,
    /// The [`AppState`] associated with the module.
    pub state: AppState,
    /// Feature flags implemented by the module.
    pub capabilities: CapabilityFlags,
}

/// Context handed to module hooks giving access to the Bevy [`World`].
pub struct ModuleContext<'a> {
    /// Mutable reference to the game world.
    pub world: &'a mut World,
}

/// Common interface implemented by all game modules.
pub trait GameModule: Plugin + Sized {
    /// Compile-time identifier for the module.
    const ID: &'static str;

    /// Returns static metadata describing the module.
    fn metadata() -> ModuleMetadata;

    /// Invoked when the server initializes the module.
    fn server_register(_context: &mut ModuleContext) {}

    /// Called whenever the engine transitions into the module's state.
    fn enter(_context: &mut ModuleContext) {}

    /// Called whenever the engine leaves the module's state.
    fn exit(_context: &mut ModuleContext) {}
}
