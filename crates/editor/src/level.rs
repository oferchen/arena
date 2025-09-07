use anyhow::Result;
use bevy_ecs::prelude::Resource;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::{cmp::Ordering, fs};

#[derive(Resource, Serialize, Deserialize, Default, Debug, Clone)]
pub struct Level {
    pub id: String,
    pub name: String,
    /// External asset identifiers that must exist for the level to load.
    #[serde(default)]
    pub references: Vec<String>,
    /// Brushes that make up the CSG representation of the level.
    #[serde(default)]
    pub brushes: Vec<Brush>,
    /// Areas where players can spawn when the level loads.
    #[serde(default)]
    pub spawn_zones: Vec<SpawnZone>,
    /// Estimated number of entities the level will spawn at runtime.
    #[serde(default)]
    pub entity_count: usize,
    /// Mapping of exported asset names to their hashed filenames.
    #[serde(default)]
    pub assets: Vec<HashedAsset>,
    /// Tagged portal surfaces for visibility culling.
    #[serde(default)]
    pub portals: Vec<Portal>,
    /// Surfaces that block visibility and are used for occlusion.
    #[serde(default)]
    pub occluders: Vec<Occluder>,
}

impl Level {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            references: Vec::new(),
            brushes: Vec::new(),
            spawn_zones: Vec::new(),
            entity_count: 0,
            assets: Vec::new(),
            portals: Vec::new(),
            occluders: Vec::new(),
        }
    }

    /// Add a constructive solid geometry brush to the level.
    pub fn add_brush(&mut self, brush: Brush) {
        self.brushes.push(brush);
    }

    /// Apply a basic UV unwrap to all brushes.
    pub fn unwrap_uvs(&mut self) {
        for b in &mut self.brushes {
            b.uv = Some(Uv { u: 0.0, v: 0.0 });
        }
    }

    /// Tag a surface as a visibility portal.
    pub fn tag_portal(&mut self, portal: Portal) {
        self.portals.push(portal);
    }

    /// Tag a surface as an occluder.
    pub fn tag_occluder(&mut self, occ: Occluder) {
        self.occluders.push(occ);
    }

    /// Register an asset by its original name and hashed identifier.
    pub fn add_asset(&mut self, name: impl Into<String>, hash: impl Into<String>) {
        self.assets.push(HashedAsset {
            name: name.into(),
            hash: hash.into(),
        });
    }
}

/// Defines a circular area players may spawn in.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SpawnZone {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
}

/// Describes a basic CSG brush with an operation and optional UV coordinates.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Brush {
    pub op: CsgOp,
    #[serde(default)]
    pub uv: Option<Uv>,
}

/// Boolean operation applied by a brush.
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum CsgOp {
    Add,
    Subtract,
}

/// Simplified UV parameters assigned during texturing.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Uv {
    pub u: f32,
    pub v: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Portal {
    pub id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Occluder {
    pub id: String,
}

/// Mapping of original asset names to their hashed filenames.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HashedAsset {
    pub name: String,
    pub hash: String,
}

/// Persist the level to the assets directory.
pub fn export_level(level: &Level) -> Result<()> {
    let mut lvl = level.clone();
    lvl.references.sort();
    lvl.assets.sort_by(|a, b| a.name.cmp(&b.name));
    lvl.brushes
        .sort_by(|a, b| format!("{:?}{:?}", a.op, a.uv).cmp(&format!("{:?}{:?}", b.op, b.uv)));
    lvl.spawn_zones.sort_by(|a, b| {
        a.x.partial_cmp(&b.x)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.y.partial_cmp(&b.y).unwrap_or(Ordering::Equal))
    });
    lvl.portals.sort_by(|a, b| a.id.cmp(&b.id));
    lvl.occluders.sort_by(|a, b| a.id.cmp(&b.id));
    let dir = Path::new("assets").join("levels").join(&lvl.id);
    fs::create_dir_all(&dir)?;
    let path = dir.join("level.toml");
    let toml = toml::to_string_pretty(&lvl)?;
    fs::write(path, toml)?;
    Ok(())
}

/// Export an additional binary referenced by the level.
pub fn export_binary(level_id: &str, _name: &str, data: &[u8]) -> Result<String> {
    let dir = Path::new("assets").join("levels").join(level_id);
    fs::create_dir_all(&dir)?;
    let hash = {
        let digest = Sha256::digest(data);
        hex::encode(digest)
    };
    let path = dir.join(&hash);
    fs::write(path, data)?;
    Ok(hash)
}
