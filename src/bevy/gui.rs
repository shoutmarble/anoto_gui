//! GUI layout and drawing components for the Anoto GUI

use bevy::prelude::*;
use bevy::picking::hover::Hovered;
use bevy::input_focus::tab_navigation::{TabGroup, TabIndex};
use bevy_ui_widgets::{observe, Activate, Button};
use crate::bevy::utils::*;

/// Creates the main image display area
pub fn image_display_area() -> impl Bundle {
    (
        ImageDisplay,
        Node {
            width: percent(100),
            height: percent(100),
            border: UiRect::all(px(2)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BorderColor::all(Color::srgb(0.5, 0.5, 0.5)),
        BackgroundColor(Color::srgb(0.9, 0.9, 0.9)),
        children![(
            PlaceholderText,
            Text::new("No image selected"),
            TextFont {
                font_size: 16.0,
                ..default()
            },
            TextColor(Color::srgb(0.5, 0.5, 0.5)),
        )],
    )
}

/// Creates the secondary image display area
pub fn secondary_image_display_area() -> impl Bundle {
    (
        SecondaryImageDisplay,
        Node {
            width: percent(100),
            height: percent(100),
            border: UiRect::all(px(2)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BorderColor::all(Color::srgb(1.0, 0.84, 0.0)), // Gold border
        BackgroundColor(Color::srgb(0.9, 0.9, 0.9)),
        children![(
            SecondaryPlaceholderText,
            Text::new("Secondary view"),
            TextFont {
                font_size: 14.0,
                ..default()
            },
            TextColor(Color::srgb(0.5, 0.5, 0.5)),
        )],
    )
}

/// Creates the image selection button
pub fn image_select_button(_asset_server: &AssetServer) -> impl Bundle {
    (
        Node {
            width: px(150),
            height: px(65),
            border: UiRect::all(px(5)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        ImageSelectButton,
        Button,
        Hovered::default(),
        TabIndex(0),
        BorderColor::all(Color::BLACK),
        BorderRadius::MAX,
        BackgroundColor(NORMAL_BUTTON),
        children![(
            Text::new("Load Image"),
            TextFont {
                font: default(),
                font_size: 22.0,
                ..default()
            },
            TextColor(Color::srgb(0.9, 0.9, 0.9)),
            TextShadow::default(),
        )],
    )
}

/// Creates the root UI layout
pub fn demo_root(asset_server: &AssetServer) -> impl Bundle {
    (
        Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Stretch,
            justify_content: JustifyContent::Start,
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            ..default()
        },
        TabGroup::default(),
        children![
            // Left column: Image display (8/10 width)
            (
                Node {
                    width: percent(80),
                    height: percent(100),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                children![image_display_area()],
            ),
            // Right column: Button and secondary image display (2/10 width)
            (
                Node {
                    width: percent(20),
                    height: percent(100),
                    justify_content: JustifyContent::Start,
                    align_items: AlignItems::Center,
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                children![
                    // Load image button at the top
                    (
                        Node {
                            width: percent(100),
                            height: px(80), // Fixed height for button area
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        children![(
                            image_select_button(asset_server),
                            observe(|_activate: On<Activate>,
                                    mut dialog_state: ResMut<DialogState>| {
                                dialog_state.should_open = true;
                                dialog_state.frame_delay = 1;
                            }),
                        )],
                    ),
                    // Secondary image display area below the button
                    (
                        Node {
                            width: percent(100),
                            height: percent(100), // Take remaining space
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        children![secondary_image_display_area()],
                    ),
                ],
            ),
        ],
    )
}

/// Setup function for the GUI
pub fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    // ui camera
    commands.spawn(Camera2d);
    commands.spawn(demo_root(&assets));
}

/// Startup check function
pub fn startup_check(
    windows: Query<&Window>,
    _asset_server: Res<AssetServer>,
) {
    // Check if window was created successfully
    if let Ok(window) = windows.single() {
        println!("Window created successfully: {}x{}", window.width(), window.height());
    } else {
        println!("Warning: Window creation status uncertain");
    }

    // Note: Asset loading will be handled by the UI system when needed
    println!("Bevy application startup completed successfully");
}

/// System that handles image select button interactions
#[allow(clippy::type_complexity)]
pub fn image_select_system(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<ImageSelectButton>),
    >,
    mut dialog_state: ResMut<DialogState>,
) {
    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = BackgroundColor(Color::srgb(0.1, 0.4, 0.6));
                dialog_state.should_open = true;
                dialog_state.frame_delay = 1;
            }
            Interaction::Hovered => {
                *color = BackgroundColor(HOVERED_BUTTON);
            }
            Interaction::None => {
                *color = BackgroundColor(NORMAL_BUTTON);
            }
        }
    }
}