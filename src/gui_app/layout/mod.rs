use bevy::{prelude::*, ui::widget::ImageNode};

use crate::gui_app::{
    scaling::{ScaleRegion, ScaleToFit},
    state::ZoomSettings,
};

pub struct LayoutPlugin;

#[derive(Component)]
pub struct LeftPanel;

#[derive(Component)]
pub struct RightPanel;

#[derive(Component)]
pub struct MainImage;

#[derive(Component)]
pub struct ZoomSquare;

#[derive(Component)]
pub struct LoadImageButton;

#[derive(Component)]
pub struct ZoomSizer;

#[derive(Component)]
pub struct ZoomSizerLabel;

#[derive(Component)]
pub struct ZoomPreview;

impl Plugin for LayoutPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_ui_root);
    }
}

fn setup_ui_root(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(Color::srgb_u8(6, 6, 6)),
            ..default()
        },
    ));

    let root = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Stretch,
                flex_direction: FlexDirection::Row,
                ..default()
            },
            BackgroundColor(Color::srgb_u8(18, 18, 18)),
            Name::new("Root"),
        ))
        .id();

    let left_panel = spawn_left_panel(&mut commands);
    let right_panel = spawn_right_panel(&mut commands);

    commands.entity(root).add_child(left_panel);
    commands.entity(root).add_child(right_panel);
}

fn spawn_left_panel(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            Node {
                width: Val::Percent(65.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                position_type: PositionType::Relative,
                ..default()
            },
            BackgroundColor(Color::srgb_u8(28, 28, 28)),
            LeftPanel,
            Name::new("LeftPanel"),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        width: Val::Percent(90.0),
                        height: Val::Percent(90.0),
                        ..default()
                    },
                    BackgroundColor(Color::BLACK),
                    ImageNode::default(),
                    MainImage,
                    ScaleToFit::new(ScaleRegion::LeftPanelImage),
                    Name::new("MainImage"),
                ));

            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(120.0),
                    height: Val::Px(120.0),
                    left: Val::Px(0.0),
                    bottom: Val::Px(0.0),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                BorderColor::all(Color::srgb(0.0, 0.6, 0.9)),
                ZoomSquare,
                Visibility::Hidden,
                Name::new("ZoomSquare"),
            ));
        })
        .id()
}

fn spawn_right_panel(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            Node {
                width: Val::Percent(35.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Start,
                align_items: AlignItems::Center,
                row_gap: Val::Px(24.0),
                padding: UiRect::axes(Val::Px(20.0), Val::Px(20.0)),
                ..default()
            },
            BackgroundColor(Color::srgb_u8(38, 38, 38)),
            RightPanel,
            Name::new("RightPanel"),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Percent(80.0),
                        height: Val::Px(48.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.4, 0.2, 0.8)),
                    LoadImageButton,
                    Name::new("LoadImageButton"),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("load image"),
                        TextFont {
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });

            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Percent(80.0),
                        height: Val::Px(72.0),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(6.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.3, 0.3, 0.5)),
                    ZoomSizer,
                    Name::new("ZoomSizer"),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("scale"),
                        TextFont {
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                    parent.spawn((
                        Text::new(format!("{:.0}px", ZoomSettings::default().square_size)),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.8, 0.8, 0.9)),
                        ZoomSizerLabel,
                    ));
                });

            parent.spawn((
                Text::new("preview"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));

            parent.spawn((
                Node {
                    width: Val::Percent(80.0),
                    height: Val::Percent(35.0),
                    ..default()
                },
                BackgroundColor(Color::BLACK),
                ImageNode::default(),
                ZoomPreview,
                ScaleToFit::new(ScaleRegion::PreviewPanel),
                Name::new("ZoomPreview"),
            ));
        })
        .id()
}
