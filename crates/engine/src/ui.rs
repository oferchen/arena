use bevy::prelude::*;

/// Root HUD node.
#[derive(Component)]
pub struct HudRoot;

/// Basic UI and HUD scaffolding.
pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_ui);
    }
}

fn setup_ui(mut commands: Commands) {
    commands
        .spawn((NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..Default::default()
            },
            ..Default::default()
        }, HudRoot))
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                "HUD",
                TextStyle {
                    font_size: 24.0,
                    color: Color::WHITE,
                    ..Default::default()
                },
            ));
        });
}

