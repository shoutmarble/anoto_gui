use bevy::{prelude::*, ui::widget::ImageNode};
use bevy_ui_widgets::{Slider, SliderPrecision, SliderRange, SliderValue};

use crate::gui_app::{
    scaling::{ScaleRegion, ScaleToFit},
    state::ZoomSettings,
};

pub const ZOOM_SLIDER_HORIZONTAL_PADDING: f32 = 12.0;

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
pub struct ZoomSlider;

#[derive(Component)]
pub struct ZoomSliderTrack;

#[derive(Component)]
pub struct ZoomSliderThumb;

#[derive(Component)]
pub struct ZoomSliderValue;

#[derive(Component)]
pub struct ZoomPreview;

#[derive(Component)]
pub struct ArrowGridText;

#[derive(Component)]
pub struct DownloadArrowsButton;

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
            parent.spawn((
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
                    Node {
                        width: Val::Percent(80.0),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::FlexStart,
                        align_items: AlignItems::Stretch,
                        row_gap: Val::Px(8.0),
                        ..default()
                    },
                    Name::new("ZoomSliderPanel"),
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Text::new("zoom scale slider"),
                        TextFont {
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));

                    panel
                        .spawn((
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Px(40.0),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::FlexStart,
                                position_type: PositionType::Relative,
                                padding: UiRect::horizontal(Val::Px(
                                    ZOOM_SLIDER_HORIZONTAL_PADDING,
                                )),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.2, 0.2, 0.35)),
                            BorderRadius::all(Val::Px(8.0)),
                            Slider::default(),
                            SliderRange::new(0.0, 100.0),
                            SliderValue(ZoomSettings::default().slider_value()),
                            SliderPrecision(3),
                            ZoomSlider,
                            Name::new("ZoomSlider"),
                        ))
                        .with_children(|slider| {
                            slider
                                .spawn((
                                    Node {
                                        width: Val::Percent(100.0),
                                        height: Val::Px(6.0),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgb(0.35, 0.35, 0.5)),
                                    BorderRadius::all(Val::Px(3.0)),
                                    ZoomSliderTrack,
                                    Name::new("ZoomSliderTrack"),
                                ))
                                .with_children(|track| {
                                    track.spawn((
                                        Node {
                                            width: Val::Px(18.0),
                                            height: Val::Px(18.0),
                                            position_type: PositionType::Absolute,
                                            top: Val::Px(-6.0), // Adjust to center vertically on the track
                                            left: Val::Px(0.0),
                                            ..default()
                                        },
                                        BackgroundColor(Color::srgb(0.8, 0.6, 1.0)),
                                        BorderColor::all(Color::srgb(0.6, 0.4, 1.0)),
                                        BorderRadius::all(Val::Px(9.0)),
                                        ZoomSliderThumb,
                                        Name::new("ZoomSliderThumb"),
                                    ));
                                });
                        });

                    panel.spawn((
                        Text::new(format!("{:.0}%", ZoomSettings::default().slider_value())),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.9, 1.0)),
                        ZoomSliderValue,
                        Name::new("ZoomSliderValue"),
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

            parent
                .spawn((
                    Node {
                        width: Val::Percent(80.0),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(8.0),
                        ..default()
                    },
                    Name::new("ArrowWidget"),
                ))
                .with_children(|widget| {
                    widget.spawn((
                        Text::new(""),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        ArrowGridText,
                        Name::new("ArrowGridText"),
                    ));

                    widget
                        .spawn((
                            Button,
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Px(32.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.3, 0.5, 0.3)),
                            DownloadArrowsButton,
                            Name::new("DownloadArrowsButton"),
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                Text::new("save arrows"),
                                TextFont {
                                    font_size: 16.0,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                        });
                });
        })
        .id()
}
