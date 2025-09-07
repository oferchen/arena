use anyhow::Result;
use bevy_ecs::prelude::Resource;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Resource, Serialize, Deserialize, Default, Debug, Clone)]
pub struct Level {
    pub id: String,
    pub name: String,
}

impl Level {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self { id: id.into(), name: name.into() }
    }
}

/// Persist the level to the assets directory.
pub fn export_level(level: &Level) -> Result<()> {
    let dir = Path::new("assets").join("levels").join(&level.id);
    fs::create_dir_all(&dir)?;
    let path = dir.join("level.toml");
    let toml = toml::to_string_pretty(level)?;
    fs::write(path, toml)?;
    Ok(())
}

/// Export an additional binary referenced by the level.
pub fn export_binary(level_id: &str, name: &str, data: &[u8]) -> Result<()> {
    let dir = Path::new("assets").join("levels").join(level_id);
    fs::create_dir_all(&dir)?;
    let path = dir.join(name);
    fs::write(path, data)?;
    Ok(())
}
