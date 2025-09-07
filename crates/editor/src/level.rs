use anyhow::Result;
use bevy_ecs::prelude::Resource;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Resource, Serialize, Deserialize, Default, Debug, Clone)]
pub struct Level {
    pub id: String,
    pub name: String,
    /// External asset identifiers that must exist for the level to load.
    #[serde(default)]
    pub references: Vec<String>,
    /// Areas where players can spawn when the level loads.
    #[serde(default)]
    pub spawn_zones: Vec<SpawnZone>,
    /// Estimated number of entities the level will spawn at runtime.
    #[serde(default)]
    pub entity_count: usize,
}

impl Level {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            references: Vec::new(),
            spawn_zones: Vec::new(),
            entity_count: 0,
        }
    }
}

/// Defines a circular area players may spawn in.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SpawnZone {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
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
