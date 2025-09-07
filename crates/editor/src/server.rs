use anyhow::{Context, Result, bail};
use bevy_ecs::prelude::Resource;
use platform_api::ModuleContext;
use std::collections::HashSet;

use crate::level::Level;

pub struct EditorServer;

/// Simple registry of asset identifiers available to the editor.
#[derive(Resource, Default)]
pub struct AssetRegistry(pub HashSet<String>);

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

    // Verify referenced assets exist
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

    // Spawn zone validation
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

    // Gameplay/performance checks (basic limits)
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
        references: level.references.clone(),
        spawn_zones: level.spawn_zones.clone(),
        entity_count: level.entity_count,
    });

    Ok(())
}
