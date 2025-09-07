use anyhow::{Result, bail};
use platform_api::ModuleContext;

use crate::level::Level;

pub struct EditorServer;

/// Validate the provided level using server-side rules.
#[allow(unused_variables)]
pub fn validate_level(ctx: &mut ModuleContext, level: &Level) -> Result<()> {
    // Structural checks
    if level.id.trim().is_empty() {
        bail!("level id cannot be empty");
    }
    if level.name.trim().is_empty() {
        bail!("level name cannot be empty");
    }

    // Gameplay/performance checks (basic limits)
    if level.id.len() > 64 || level.name.len() > 64 {
        bail!("level metadata too long");
    }

    // Ensure no other level is currently active
    if ctx.world().contains_resource::<Level>() {
        bail!("a level is already active");
    }

    Ok(())
}

/// Hook for playing the level inside the editor environment.
#[allow(unused_variables)]
pub fn play_in_editor(ctx: &mut ModuleContext, level: &Level) -> Result<()> {
    // Run validation before attempting to play the level
    validate_level(ctx, level)?;

    // Spawn the level into a sandboxed session represented by the world. A real
    // implementation would spin up the appropriate module here; we simply store
    // the level as a resource to indicate an active session.
    ctx.world().insert_resource(Level {
        id: level.id.clone(),
        name: level.name.clone(),
    });

    Ok(())
}
