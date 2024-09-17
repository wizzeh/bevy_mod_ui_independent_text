use bevy::prelude::*;
use bevy_mod_ui_independent_text::*;

fn setup(mut commands: Commands, asset_loader: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());
    commands.spawn(IndependentTextBundle {
        text: UiText(Text {
            sections: vec![TextSection {
                value: "Hello, world".to_string(),
                style: TextStyle {
                    font: asset_loader.load("Topaz-8.ttf"),
                    font_size: 32.0,
                    color: Color::WHITE,
                },
            }],
            justify: JustifyText::Center,
            linebreak_behavior: bevy::text::BreakLineOn::WordBoundary,
        }),
        transform: Transform {
            translation: Vec3::new(400., 300., 100.),
            rotation: Quat::from_rotation_z(std::f32::consts::PI / 8.),
            ..Default::default()
        },
        ..Default::default()
    });
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(IndependentTextPlugin)
        .add_systems(Startup, setup)
        .run();
}
