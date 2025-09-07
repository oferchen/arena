use anyhow::Result;
use platform_api::ModuleContext;

use crate::level::Level;

pub struct EditorServer;

/// Validate the provided level using server-side rules.
#[allow(unused_variables)]
pub fn validate_level(ctx: &mut ModuleContext, level: &Level) -> Result<()> {
    // TODO: perform server-side validation of the level data
    Ok(())
}

/// Hook for playing the level inside the editor environment.
#[allow(unused_variables)]
pub fn play_in_editor(ctx: &mut ModuleContext, level: &Level) -> Result<()> {
    // TODO: bridge between the editor and running modules
    Ok(())
}
