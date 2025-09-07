use bevy::prelude::*;

#[cfg(feature = "webgl2")]
use bevy_webgl2::WebGL2Plugin;

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(DefaultPlugins);

        #[cfg(feature = "webgl2")]
        app.add_plugins(WebGL2Plugin);
    }
}
