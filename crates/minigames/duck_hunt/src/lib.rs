use anyhow::Result;
use bevy::prelude::*;
use platform_api::{
    AppState, CapabilityFlags, GameModule, ModuleContext, ModuleMetadata, ServerApp,
};

#[path = "../server.rs"]
pub mod server;

#[derive(Resource, Default, Debug)]
pub struct Score(pub u32);

#[derive(Resource, Debug)]
pub struct Multiplier(pub u32);

#[derive(Resource, Debug)]
pub struct Ammo(pub u32);

#[derive(Resource, Debug)]
pub struct RoundTimer {
    pub remaining: f32,
}

#[derive(Resource, Debug)]
pub struct Rtt(pub f32);

#[derive(Resource)]
pub struct HudProfile {
    pub font: Handle<Font>,
    pub font_size: f32,
    pub color: Color,
    pub timer: f32,
    pub score: u32,
    pub multiplier: u32,
    pub ammo: u32,
    pub rtt: f32,
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
        timer: 0.0,
        score: 0,
        multiplier: 1,
        ammo: 0,
        rtt: 0.0,
    });
    world.insert_resource(Score::default());
    world.insert_resource(Multiplier(1));
    world.insert_resource(Ammo(0));
    world.insert_resource(RoundTimer { remaining: 0.0 });
    world.insert_resource(Rtt(0.0));
}

fn cleanup(world: &mut World) {
    world.remove_resource::<HudProfile>();
    world.remove_resource::<Score>();
    world.remove_resource::<Multiplier>();
    world.remove_resource::<Ammo>();
    world.remove_resource::<RoundTimer>();
    world.remove_resource::<Rtt>();
}

pub fn award_score(world: &mut World, points: u32) {
    let mult_value = {
        let mut mult = world.get_resource_or_insert_with(|| Multiplier(1));
        let val = mult.0;
        mult.0 += 1;
        val
    };
    {
        let mut score = world.get_resource_or_insert_with(Score::default);
        score.0 += points * mult_value;
    }
    let score_val = world.get_resource::<Score>().map(|s| s.0).unwrap_or(0);
    if let Some(mut hud) = world.get_resource_mut::<HudProfile>() {
        hud.score = score_val;
        hud.multiplier = mult_value + 1;
    }
}

pub fn start_round(world: &mut World, duration: f32, ammo: u32) {
    world.insert_resource(RoundTimer { remaining: duration });
    world.insert_resource(Multiplier(1));
    world.insert_resource(Ammo(ammo));
    if let Some(mut hud) = world.get_resource_mut::<HudProfile>() {
        hud.timer = duration;
        hud.multiplier = 1;
        hud.ammo = ammo;
    }
}

pub fn tick_round(world: &mut World, dt: f32) {
    let mut finished = false;
    if let Some(mut timer) = world.get_resource_mut::<RoundTimer>() {
        if timer.remaining > 0.0 {
            timer.remaining = (timer.remaining - dt).max(0.0);
            if timer.remaining == 0.0 {
                finished = true;
            }
            let remaining = timer.remaining;
            drop(timer);
            if let Some(mut hud) = world.get_resource_mut::<HudProfile>() {
                hud.timer = remaining;
            }
        }
    }
    if finished {
        if let Some(mut mult) = world.get_resource_mut::<Multiplier>() {
            mult.0 = 1;
        }
        if let Some(mut hud) = world.get_resource_mut::<HudProfile>() {
            hud.multiplier = 1;
        }
    }
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
