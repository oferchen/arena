use std::{collections::HashMap, fs, path::Path};

use anyhow::Result;
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

fn main() -> Result<()> {
    let web = Path::new("web");
    let assets_dir = Path::new("assets");
    let static_dir = Path::new("static");
    fs::create_dir_all(assets_dir)?;
    fs::create_dir_all(static_dir)?;

    let mut manifest: HashMap<String, String> = HashMap::new();
    let mut precache: Vec<String> = Vec::new();

    for entry in WalkDir::new(web).into_iter().filter_map(Result::ok) {
        if entry.file_type().is_file() {
            let path = entry.path();
            let file_name = path.file_name().unwrap().to_string_lossy().to_string();
            if matches!(file_name.as_str(), "index.html" | "sw.js" | "manifest.json") {
                fs::copy(path, static_dir.join(&file_name))?;
                continue;
            }
            let data = fs::read(path)?;
            let hash = Sha256::digest(&data);
            let hash_hex = hex::encode(&hash)[..16].to_string();
            let stem = path.file_stem().unwrap().to_string_lossy();
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            let hashed_name = if ext.is_empty() {
                format!("{stem}-{hash_hex}")
            } else {
                format!("{stem}-{hash_hex}.{ext}")
            };
            let dest = assets_dir.join(&hashed_name);
            fs::write(&dest, data)?;
            manifest.insert(file_name.clone(), hashed_name.clone());
            precache.push(format!("/assets/{hashed_name}"));
        }
    }

    fs::write(assets_dir.join("manifest.json"), serde_json::to_string_pretty(&manifest)?)?;
    fs::write(assets_dir.join("precache.json"), serde_json::to_string_pretty(&precache)?)?;

    Ok(())
}
