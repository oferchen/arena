use bevy::prelude::*;
use engine::{ModuleRegistry, register_module};
use log::Level;
use logtest::Logger;
use platform_api::{AppState, CapabilityFlags, GameModule, ModuleContext, ModuleMetadata};

#[derive(Default)]
struct FailingModule;

impl Plugin for FailingModule {
    fn build(&self, _app: &mut App) {}
}

impl GameModule for FailingModule {
    const ID: &'static str = "failing";

    fn metadata() -> ModuleMetadata {
        ModuleMetadata {
            id: "failing".to_string(),
            name: "Failing".to_string(),
            version: "0.1.0".to_string(),
            author: "Test".to_string(),
            state: AppState::DuckHunt,
            capabilities: CapabilityFlags::empty(),
            max_players: 4,
            icon: Handle::default(),
        }
    }

    fn enter(_ctx: &mut ModuleContext) -> anyhow::Result<()> {
        anyhow::bail!("boom")
    }

    fn exit(_ctx: &mut ModuleContext) -> anyhow::Result<()> {
        anyhow::bail!("bust")
    }
}

#[test]
fn logs_module_errors_without_panic() {
    let mut logger = Logger::start();

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_state::<AppState>();
    app.init_resource::<ModuleRegistry>();

    register_module::<FailingModule>(&mut app);

    app.world
        .resource_mut::<NextState<AppState>>()
        .set(AppState::DuckHunt);
    app.update();
    assert!(logger.any(|r| r.level() == Level::Error && r.args().contains("module enter failed")));

    app.world
        .resource_mut::<NextState<AppState>>()
        .set(AppState::Lobby);
    app.update();
    assert!(logger.any(|r| r.level() == Level::Error && r.args().contains("module exit failed")));
}
