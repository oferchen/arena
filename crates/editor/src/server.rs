use anyhow::{bail, Context, Result};
use bevy_app::{AppExit, MainScheduleOrder};
use bevy_ecs::prelude::Resource;
use bevy_ecs::{prelude::*, schedule::Schedules};
use platform_api::{GameModule, ModuleContext, ServerApp};
use std::collections::HashSet;

use crate::level::Level;

pub struct EditorServer;

/// Tracking a running editor play session.
pub struct EditorSession {
    pub app: ServerApp,
}

/// Simple registry of asset identifiers available to the editor.
#[derive(Resource, Default)]
pub struct AssetRegistry(pub HashSet<String>);

/// Perform structural validation on the level definition.
pub fn validate_structural(level: &Level) -> Result<()> {
    if level.id.trim().is_empty() {
        bail!("level id cannot be empty");
    }
    if level.name.trim().is_empty() {
        bail!("level name cannot be empty");
    }
    if level.spawn_zones.is_empty() {
        bail!("level must define at least one spawn zone");
    }
    const WORLD_BOUND: f32 = 1000.0;
    for (i, z) in level.spawn_zones.iter().enumerate() {
        if z.radius <= 0.0 {
            bail!("spawn zone {} has non-positive radius", i);
        }
        if z.x.abs() > WORLD_BOUND || z.y.abs() > WORLD_BOUND {
            bail!("spawn zone {} out of bounds", i);
        }
    }
    Ok(())
}

/// Validate gameplay related concerns such as asset references.
pub fn validate_gameplay(ctx: &mut ModuleContext, level: &Level) -> Result<()> {
    if !level.references.is_empty() {
        let registry = ctx
            .world()
            .get_resource::<AssetRegistry>()
            .context("asset registry missing")?;
        for r in &level.references {
            if !registry.0.contains(r) {
                bail!("missing asset reference: {}", r);
            }
        }
    }
    Ok(())
}

/// Validate performance budgets for the level.
pub fn validate_performance(level: &Level) -> Result<()> {
    if level.id.len() > 64 || level.name.len() > 64 {
        bail!("level metadata too long");
    }
    const ENTITY_LIMIT: usize = 1000;
    if level.entity_count > ENTITY_LIMIT {
        bail!(
            "level exceeds entity budget: {}/{}",
            level.entity_count,
            ENTITY_LIMIT
        );
    }
    Ok(())
}

/// Validate the provided level using server-side rules.
pub fn validate_level(ctx: &mut ModuleContext, level: &Level) -> Result<()> {
    validate_structural(level)?;
    validate_gameplay(ctx, level)?;
    validate_performance(level)?;

    // Ensure no other level is currently active
    if ctx.world().contains_resource::<Level>() {
        bail!("a level is already active");
    }

    Ok(())
}

/// Hook for playing the level inside the editor environment.
#[allow(unused_variables)]
pub fn play_in_editor<M: GameModule + Default>(
    ctx: &mut ModuleContext,
    level: &Level,
) -> Result<()> {
    // Run validation before attempting to play the level
    validate_level(ctx, level)?;

    // Stop any existing session and reclaim its world so we can reload.
    stop_play_in_editor(ctx);

    // Move the editor world into a new server app so modules can operate on the
    // same entities and resources (spawn points, etc.).
    let mut app = ServerApp::new();
    let mut editor_world = World::new();
    std::mem::swap(ctx.world(), &mut editor_world);

    // Carry over scheduling resources required by the app.
    if let Some(schedules) = app.world.remove_resource::<Schedules>() {
        editor_world.insert_resource(schedules);
    }
    if let Some(order) = app.world.remove_resource::<MainScheduleOrder>() {
        editor_world.insert_resource(order);
    }
    app.world = editor_world;
    app.add_event::<AppExit>();

    // Provide the level to the module.
    app.world.insert_resource(level.clone());

    // Register and initialize the requested module.
    M::server_register(&mut app);
    app.add_plugins(M::default());

    // Run a short headless tick loop so the module can initialize.
    for _ in 0..10 {
        app.update();
    }

    // Store the running session in the editor context so it can be stopped or
    // reloaded later.
    ctx.world().insert_non_send_resource(EditorSession { app });
    Ok(())
}

/// Stop the currently running editor session, returning control of the world to
/// the caller.
pub fn stop_play_in_editor(ctx: &mut ModuleContext) {
    if let Some(mut session) = ctx.world().remove_non_send_resource::<EditorSession>() {
        let mut world = World::new();
        std::mem::swap(&mut session.app.world, &mut world);
        std::mem::swap(ctx.world(), &mut world);
    }
    // Ensure any level resource from the play session is cleared.
    ctx.world().remove_resource::<Level>();
}
