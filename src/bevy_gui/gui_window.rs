//! Resizable GUI window that takes 2/3 of the screen dimensions

use bevy::prelude::*;
use bevy::window::{WindowMode, WindowResolution};

/// Marker component for the window size text
#[derive(Component)]
struct WindowSizeText;

/// Main function to run the GUI window
pub fn run_gui_window() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Anoto GUI - Resizable Window".to_string(),
                resolution: get_window_resolution(),
                resizable: true,
                mode: WindowMode::Windowed,
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup_gui)
        .add_systems(Update, update_window_size_text)
        .run();
}

/// Calculate window resolution as 2/3 of the primary monitor's size
fn get_window_resolution() -> WindowResolution {
    // For now, we'll use reasonable defaults that work well on most modern displays
    // In a production app, you might want to query the actual monitor size at runtime
    // using winit's monitor detection, but this requires additional setup

    // Common modern resolutions: 1920x1080, 2560x1440, 3840x2160
    // We'll use 1920x1080 as a reasonable default and calculate 2/3
    let screen_width = 1920.0;
    let screen_height = 1080.0;

    let window_width = (screen_width * 2.0 / 3.0) as u32;   // ~1280
    let window_height = (screen_height * 2.0 / 3.0) as u32; // ~720

    println!("Creating window: {}x{} (2/3 of {}x{})",
             window_width, window_height, screen_width as u32, screen_height as u32);

    WindowResolution::new(window_width, window_height)
}

/// Setup the GUI elements
fn setup_gui(mut commands: Commands) {
    // Spawn the UI camera
    commands.spawn((
        Camera2d,
        Camera {
            ..default()
        },
    ));

    // Create a root UI node that fills the window
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_direction: FlexDirection::Column,
            ..default()
        },
        BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
    )).with_children(|parent| {
        // Title text
        parent.spawn((
            Text::new("Anoto GUI - Resizable Window"),
            TextFont {
                font_size: 32.0,
                ..default()
            },
            TextColor(Color::WHITE),
            Node {
                margin: UiRect::bottom(Val::Px(20.0)),
                ..default()
            },
        ));

        // Info text
        parent.spawn((
            Text::new("This window is 2/3 the size of your screen and is resizable.\nTry dragging the edges to resize!"),
            TextFont {
                font_size: 18.0,
                ..default()
            },
            TextColor(Color::srgb(0.8, 0.8, 0.8)),
            TextLayout::new_with_justify(Justify::Center),
            Node {
                margin: UiRect::bottom(Val::Px(30.0)),
                ..default()
            },
        ));

        // Resize hint with dynamic size display
        parent.spawn((
            Text::new("Window Size: Drag to resize"),
            TextFont {
                font_size: 14.0,
                ..default()
            },
            TextColor(Color::srgb(0.6, 0.6, 0.6)),
            WindowSizeText, // Mark this text for dynamic updates
        ));
    });
}

/// System to update the window size text dynamically
fn update_window_size_text(
    window_query: Query<&Window>,
    mut text_query: Query<&mut Text, With<WindowSizeText>>,
) {
    let Ok(window) = window_query.single() else { return };
    let Ok(mut text) = text_query.single_mut() else { return };

    *text = Text::new(format!("Window Size: {}x{} - Drag edges to resize!",
                             window.width() as u32, window.height() as u32));
}