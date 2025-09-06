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

#[derive(Clone)]
pub struct ModuleMetadata {
    pub name: &'static str,
    pub state: AppState,
    pub capabilities: CapabilityFlags,
}

pub struct ModuleContext<'a> {
    pub app: &'a mut App,
}

pub trait GameModule: Plugin + Sized {
    fn metadata() -> ModuleMetadata;
    fn register(_context: &mut ModuleContext) {}
}
