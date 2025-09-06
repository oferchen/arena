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
            if file_name == "index.html" {
                fs::copy(path, static_dir.join(&file_name))?;
                continue;
            }
            if file_name == "manifest.json" || file_name == "sw.js" {
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

    // process module assets
    let modules_src = web.join("modules");
    if modules_src.exists() {
        let modules_dest_root = assets_dir.join("modules");
        for module_entry in fs::read_dir(modules_src)? {
            let module_entry = module_entry?;
            if !module_entry.file_type()?.is_dir() {
                continue;
            }
            let module_name = module_entry.file_name().to_string_lossy().to_string();
            let module_dest = modules_dest_root.join(&module_name);
            fs::create_dir_all(&module_dest)?;
            let mut module_manifest: HashMap<String, String> = HashMap::new();
            for asset in WalkDir::new(module_entry.path())
                .into_iter()
                .filter_map(Result::ok)
            {
                if asset.file_type().is_file() {
                    let path = asset.path();
                    let file_name = path.file_name().unwrap().to_string_lossy().to_string();
                    if file_name == "module.toml" {
                        fs::copy(path, module_dest.join(&file_name))?;
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
                    fs::write(module_dest.join(&hashed_name), data)?;
                    module_manifest.insert(file_name, hashed_name);
                }
            }
            fs::write(
                module_dest.join("manifest.json"),
                serde_json::to_string_pretty(&module_manifest)?,
            )?;
        }
    }

    // rewrite manifest.json with hashed icon paths
    let manifest_src = fs::read_to_string(web.join("manifest.json"))?;
    let mut manifest_json: serde_json::Value = serde_json::from_str(&manifest_src)?;
    if let Some(icons) = manifest_json
        .get_mut("icons")
        .and_then(|v| v.as_array_mut())
    {
        for icon in icons.iter_mut() {
            if let Some(src) = icon.get_mut("src") {
                if let Some(src_str) = src.as_str() {
                    let file_name = Path::new(src_str)
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .to_string();
                    if let Some(hashed) = manifest.get(&file_name) {
                        *src = serde_json::Value::String(format!("/assets/{hashed}"));
                    }
                }
            }
        }
    }
    fs::write(
        static_dir.join("manifest.json"),
        serde_json::to_string_pretty(&manifest_json)?,
    )?;

    let precache_json = serde_json::to_string_pretty(&precache)?;
    fs::write(
        assets_dir.join("manifest.json"),
        serde_json::to_string_pretty(&manifest)?,
    )?;
    fs::write(assets_dir.join("precache.json"), &precache_json)?;

    let hash = Sha256::digest(precache_json.as_bytes());
    let manifest_version = hex::encode(&hash)[..16].to_string();
    let sw_src = fs::read_to_string(web.join("sw.js"))?;
    let sw_versioned = sw_src.replace("__PRECACHE_VERSION__", &manifest_version);
    fs::write(static_dir.join("sw.js"), sw_versioned)?;

    Ok(())
}
