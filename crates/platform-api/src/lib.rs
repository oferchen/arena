use bevy::ecs::world::Mut;
use bevy::prelude::*;
use bitflags::bitflags;
use anyhow::Result;

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
        const NeedsPhysics = 0b0010;
        const UsesHitscan = 0b0100;
        const NeedsNav = 0b1000;
        const UsesVehicles = 0b1_0000;
        const UsesFlight = 0b10_0000;
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
    /// Maximum number of players supported.
    pub max_players: u32,
    /// Icon representing the module.
    pub icon: Handle<Image>,
}

/// Context handed to module hooks giving access to the Bevy [`World`] and other
/// common resources.
pub struct ModuleContext<'a> {
    world: &'a mut World,
}

impl<'a> ModuleContext<'a> {
    /// Create a new context wrapping the provided [`World`].
    pub fn new(world: &'a mut World) -> Self {
        Self { world }
    }

    /// Borrow the underlying [`World`].
    pub fn world(&mut self) -> &mut World {
        self.world
    }

    /// Access asset storage for the given type.
    pub fn assets<A: Asset>(&mut self) -> Mut<Assets<A>> {
        self.world.resource_mut::<Assets<A>>()
    }

    /// Fetch a network resource of the provided type.
    pub fn network<N: Resource>(&mut self) -> Mut<N> {
        self.world.resource_mut::<N>()
    }

    /// Retrieve the [`Time`] resource.
    pub fn time(&self) -> &Time {
        self.world.resource::<Time>()
    }

    /// Access an audio-related resource.
    pub fn audio<A: Resource>(&mut self) -> Mut<A> {
        self.world.resource_mut::<A>()
    }

    /// Access a UI-related resource.
    pub fn ui<U: Resource>(&mut self) -> Mut<U> {
        self.world.resource_mut::<U>()
    }
}

/// Common interface implemented by all game modules.
pub type ServerApp = App;

/// Common interface implemented by all game modules.
pub trait GameModule: Plugin + Sized {
    /// Compile-time identifier for the module.
    const ID: &'static str;

    /// Returns static metadata describing the module.
    fn metadata() -> ModuleMetadata;

    /// Invoked when the module is registered with the engine.
    fn register(_app: &mut App) {}

    /// Invoked when the server initializes the module.
    fn server_register(_app: &mut ServerApp) {}

    /// Called whenever the engine transitions into the module's state.
    fn enter(_context: &mut ModuleContext) -> Result<()> { Ok(()) }

    /// Called whenever the engine leaves the module's state.
    fn exit(_context: &mut ModuleContext) -> Result<()> { Ok(()) }
}
