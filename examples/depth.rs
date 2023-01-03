use bevy::prelude::*;
use bevy_mod_ui_independent_text::*;

fn setup(mut commands: Commands, asset_loader: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());
    commands
        .spawn(NodeBundle {
            style: Style {
                margin: UiRect {
                    left: Val::Px(200.),
                    right: Val::Px(200.),
                    bottom: Val::Px(100.),
                    top: Val::Px(100.),
                },
                flex_grow: 1.,
                ..Default::default()
            },
            background_color: BackgroundColor(Color::NAVY),
            ..Default::default()
        })
        .with_children(|builder| {
            builder.spawn(NodeBundle {
                style: Style {
                    margin: UiRect::all(Val::Px(50.)),
                    flex_grow: 1.,
                    ..Default::default()
                },
                background_color: BackgroundColor(Color::MAROON),
                ..Default::default()
            });
        });
    let labels = [
        ("This label is above the UI", 400., 1.),
        ("This label is in-between", 300., 0.001),
        ("This label is hidden behind", 200., 0.),
    ];
    for (message, y, z) in labels.into_iter() {
        commands.spawn(IndependentTextBundle {
            text: UiText(Text {
                sections: vec![TextSection {
                    value: message.to_string(),
                    style: TextStyle {
                        font: asset_loader.load("Topaz-8.ttf"),
                        font_size: 32.0,
                        color: Color::WHITE,
                    },
                }],
                alignment: TextAlignment::CENTER,
            }),
            transform: Transform::from_translation(Vec3::new(400., y, z)),
            ..Default::default()
        });
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(IndependentTextPlugin)
        .add_startup_system(setup)
        .run();
}
