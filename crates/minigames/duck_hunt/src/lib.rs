use anyhow::Result;
use bevy::prelude::*;
use platform_api::{
    AppState, CapabilityFlags, GameModule, ModuleContext, ModuleMetadata, ServerApp,
};

#[path = "../server.rs"]
pub mod server;

#[derive(Resource, Default, Debug)]
pub struct Score(pub u32);

#[derive(Resource)]
pub struct HudProfile {
    pub font: Handle<Font>,
    pub font_size: f32,
    pub color: Color,
}

fn setup(world: &mut World) {
    let Some(asset_server) = world.get_resource::<AssetServer>() else {
        return;
    };
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    world.insert_resource(HudProfile {
        font,
        font_size: 32.0,
        color: Color::WHITE,
    });
    world.insert_resource(Score::default());
}

fn cleanup(world: &mut World) {
    world.remove_resource::<HudProfile>();
    world.remove_resource::<Score>();
}

pub fn award_score(world: &mut World, points: u32) {
    let mut score = world.get_resource_or_insert_with(Score::default);
    score.0 += points;
}

#[derive(Default)]
pub struct DuckHuntModule;

impl Plugin for DuckHuntModule {
    fn build(&self, _app: &mut App) {}
}

impl GameModule for DuckHuntModule {
    const ID: &'static str = "duck_hunt";

    fn metadata() -> ModuleMetadata {
        ModuleMetadata {
            id: Self::ID.to_string(),
            name: "Duck Hunt".to_string(),
            version: "0.1.0".to_string(),
            author: "Unknown".to_string(),
            state: AppState::DuckHunt,
            capabilities: CapabilityFlags::LOBBY_PAD,
            max_players: 4,
            icon: Handle::default(),
        }
    }

    fn register(_app: &mut App) {}

    fn enter(ctx: &mut ModuleContext) -> Result<()> {
        setup(ctx.world());
        Ok(())
    }

    fn exit(ctx: &mut ModuleContext) -> Result<()> {
        cleanup(ctx.world());
        Ok(())
    }

    fn server_register(_app: &mut ServerApp) {}
}
