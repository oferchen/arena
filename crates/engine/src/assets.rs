use bevy::prelude::*;

/// Registers asset loaders such as KTX2/Basis and meshoptimizer.
pub struct AssetPlugin;

impl Plugin for AssetPlugin {
    fn build(&self, _app: &mut App) {
        // TODO: register asset loaders
    }
}
