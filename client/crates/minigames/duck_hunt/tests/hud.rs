use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use duck_hunt::DuckHuntPlugin;
use platform_api::{GameModule, ModuleContext};

#[test]
fn shows_hud_text_on_enter() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());
    app.init_asset::<Font>();

    {
        let mut ctx = ModuleContext::new(&mut app.world);
        DuckHuntPlugin::enter(&mut ctx).unwrap();
    }

    let mut found = false;
    let mut texts = app.world.query::<&Text>();
    for text in texts.iter(&app.world) {
        if text.sections.iter().any(|s| s.value.contains("Score: 0")) {
            found = true;
            break;
        }
    }
    assert!(found, "missing HUD text");
}
