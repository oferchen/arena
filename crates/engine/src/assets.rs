use bevy::asset::AssetPlugin as BevyAssetPlugin;
use bevy::gltf::GltfPlugin;
use bevy::prelude::*;

/// Registers asset loaders such as KTX2/Basis and meshoptimizer.
pub struct AssetPlugin;

impl Plugin for AssetPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((BevyAssetPlugin::default(), GltfPlugin::default()));
    }
}
