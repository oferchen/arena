use bevy::prelude::App;
use bitflags::bitflags;

bitflags! {
    /// Bitflags representing the capabilities of a module.
    pub struct ModuleCapability: u32 {
        /// Module has client-side functionality.
        const CLIENT = 0b0001;
        /// Module has server-side functionality.
        const SERVER = 0b0010;
    }
}

/// Metadata describing a module.
pub struct ModuleMetadata {
    pub name: &'static str,
    pub capabilities: ModuleCapability,
}

impl ModuleMetadata {
    pub const fn new(name: &'static str, capabilities: ModuleCapability) -> Self {
        Self { name, capabilities }
    }
}

/// Context object provided to module hooks.
pub struct ModuleContext<'a> {
    pub app: &'a mut App,
}

impl<'a> ModuleContext<'a> {
    pub fn new(app: &'a mut App) -> Self {
        Self { app }
    }
}

/// Trait implemented by all game modules.
pub trait GameModule {
    /// Unique identifier for the module.
    const ID: &'static str;

    /// Static metadata for the module.
    fn metadata() -> ModuleMetadata;

    /// Called during application startup to allow the module to register systems.
    fn register(_ctx: &mut ModuleContext) {}

    /// Called when entering the module's game state.
    fn enter(_ctx: &mut ModuleContext) {}

    /// Called when exiting the module's game state.
    fn exit(_ctx: &mut ModuleContext) {}

    /// Optional hook for server-side registration.
    fn server_register(_ctx: &mut ModuleContext) {}
}
