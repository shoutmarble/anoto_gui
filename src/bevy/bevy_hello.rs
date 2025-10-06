//! This experimental example illustrates how to create widgets using the `bevy_ui_widgets` widget set.
//!
//! These widgets have no inherent styling, so this example also shows how to implement custom styles.
//!
//! The patterns shown here are likely to change substantially as the `bevy_ui_widgets` crate
//! matures, so please exercise caution if you are using this as a reference for your own code,
//! and note that there are still "user experience" issues with this API.

use bevy::{
    asset::AssetPlugin,
    input_focus::tab_navigation::{TabGroup, TabIndex},
    picking::hover::Hovered,
    prelude::*,
    window::{Window, WindowMode, WindowPlugin, WindowPosition, WindowResolution, MonitorSelection, PresentMode},
};
use bevy_ui_widgets::{
    observe, Activate, Button,
    UiWidgetsPlugins,
};
use std::path::PathBuf;
use image::ImageReader;

pub fn bevy_hello() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(AssetPlugin {
                file_path: "../../assets".to_string(),
                ..default()
            }).set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Anoto GUI".to_string(),
                    resolution: WindowResolution::new(864, 583),
                    position: WindowPosition::Automatic, // Changed from Centered to Automatic for better WSL compatibility
                    mode: WindowMode::Windowed,
                    resizable: false,
                    decorations: true,
                    transparent: false,
                    visible: true,
                    present_mode: PresentMode::AutoNoVsync, // Changed to NoVsync for better stability in software rendering
                    ..default()
                }),
                ..default()
            }),
            UiWidgetsPlugins,
        ))
        .init_resource::<SelectedImage>()
        .init_resource::<DialogState>()
        .add_systems(Startup, (setup, startup_check).chain())
        .add_systems(
            Update,
            (
                image_select_system,
                dialog_system,
                update_image_display,
                magnifier_system,
            ),
        )
        .run();
}

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);

/// Marker for the image selection button
#[derive(Component)]
struct ImageSelectButton;

/// Marker for the image display area
#[derive(Component)]
struct ImageDisplay;

/// Marker for placeholder text when no image is selected
#[derive(Component)]
struct PlaceholderText;

/// Marker for the magnifier window
#[derive(Component)]
struct Magnifier;

/// Resource to track the selected image
#[derive(Resource, Default)]
struct SelectedImage {
    path: Option<PathBuf>,
    handle: Option<Handle<Image>>,
    is_loading: bool,
}

/// Resource to track dialog state
#[derive(Resource, Default)]
struct DialogState {
    should_open: bool,
    frame_delay: u32,
}

fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    // ui camera
    commands.spawn(Camera2d);
    commands.spawn(demo_root(&assets));
}

fn startup_check(
    windows: Query<&Window>,
    asset_server: Res<AssetServer>,
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

fn demo_root(asset_server: &AssetServer) -> impl Bundle {
    (
        Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            row_gap: px(20),
            ..default()
        },
        TabGroup::default(),
        children![
            (
                image_select_button(asset_server),
                observe(|_activate: On<Activate>,
                        mut dialog_state: ResMut<DialogState>| {
                    dialog_state.should_open = true;
                    dialog_state.frame_delay = 1;
                }),
            ),
            image_display_area(),
        ],
    )
}

fn image_select_button(asset_server: &AssetServer) -> impl Bundle {
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

fn image_display_area() -> impl Bundle {
    (
        ImageDisplay,
        Node {
            width: px(300),
            height: px(200),
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

fn image_select_system(
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

fn dialog_system(
    mut dialog_state: ResMut<DialogState>,
    mut commands: Commands,
) {
    if dialog_state.should_open {
        if dialog_state.frame_delay > 0 {
            dialog_state.frame_delay -= 1;
        } else {
            dialog_state.should_open = false;

            if let Some(file) = rfd::FileDialog::new()
                .add_filter("Images", &["png", "jpg", "jpeg", "gif", "bmp", "tga"])
                .set_directory(".")
                .pick_file() {
                let path = file;
                println!("Selected image: {:?}", path);

                commands.insert_resource(SelectedImage {
                    path: Some(path),
                    handle: None,
                    is_loading: false,
                });
            }
        }
    }
}

fn update_image_display(
    mut commands: Commands,
    mut selected_image: ResMut<SelectedImage>,
    mut last_path: Local<Option<PathBuf>>,
    image_display_query: Query<Entity, With<ImageDisplay>>,
    displayed_images: Query<Entity, With<DisplayedImage>>,
    placeholder_text: Query<Entity, With<PlaceholderText>>,
    mut images: ResMut<Assets<Image>>,
) {
    if selected_image.path != *last_path && !selected_image.is_loading {
        *last_path = selected_image.path.clone();

        // Remove old displayed images and placeholder text
        for entity in displayed_images.iter() {
            commands.entity(entity).despawn();
        }

        if let Some(path) = &selected_image.path {
            println!("Loading image from path: {:?}", path);

            match ImageReader::open(path) {
                Ok(reader) => {
                    match reader.decode() {
                        Ok(img) => {
                            let rgba_img = img.to_rgba8();
                            let (width, height) = rgba_img.dimensions();

                            let bevy_image = Image::new(
                                bevy::render::render_resource::Extent3d {
                                    width,
                                    height,
                                    depth_or_array_layers: 1,
                                },
                                bevy::render::render_resource::TextureDimension::D2,
                                rgba_img.into_raw(),
                                bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
                                Default::default(),
                            );

                            let image_handle = images.add(bevy_image);
                            selected_image.handle = Some(image_handle);
                            selected_image.is_loading = false;

                            println!("Image loaded successfully: {}x{}", width, height);
                        }
                        Err(e) => {
                            println!("Failed to decode image: {:?}", e);
                            selected_image.handle = None;
                            selected_image.is_loading = false;
                        }
                    }
                }
                Err(e) => {
                    println!("Failed to open image file: {:?}", e);
                    selected_image.handle = None;
                    selected_image.is_loading = false;
                }
            }
        } else {
            selected_image.handle = None;
            selected_image.is_loading = false;
        }
    }

    if !selected_image.is_loading && selected_image.handle.is_some() {
        if let Some(handle) = &selected_image.handle {
            println!("Displaying loaded image...");

            // Remove placeholder text
            for text_entity in placeholder_text.iter() {
                commands.entity(text_entity).despawn();
            }

            for container_entity in image_display_query.iter() {
                commands.entity(container_entity).with_children(|parent| {
                    parent.spawn((
                        ImageNode::new(handle.clone()),
                        DisplayedImage,
                        Hovered::default(),
                        Node {
                            width: Val::Px(280.0),
                            height: Val::Px(180.0),
                            margin: UiRect::all(Val::Px(10.0)),
                            ..default()
                        },
                    ));
                });
            }

            selected_image.handle = None;
        }
    }
}

#[derive(Component)]
struct DisplayedImage;

fn magnifier_system(
    mut commands: Commands,
    mut magnifier_query: Query<(Entity, &mut Node, &mut ImageNode), With<Magnifier>>,
    displayed_image_query: Query<(), With<DisplayedImage>>,
    window_query: Query<&Window>,
    selected_image: Res<SelectedImage>,
    mut last_hover_state: Local<bool>,
) {
    let Ok(window) = window_query.single() else { return };

    let mut is_hovering = false;
    let mut mouse_pos = Vec2::ZERO;

    // Check if mouse is hovering over the displayed image area
    if let Some(cursor_pos) = window.cursor_position() {
        mouse_pos = cursor_pos;

        // Calculate if mouse is within the image display area
        // The UI is centered, display area is 300x200
        let window_size = Vec2::new(window.width(), window.height());
        let display_area_size = Vec2::new(300.0, 200.0);
        let display_area_min = (window_size - display_area_size) / 2.0;
        let display_area_max = display_area_min + display_area_size;

        if cursor_pos.x >= display_area_min.x && cursor_pos.x <= display_area_max.x &&
           cursor_pos.y >= display_area_min.y && cursor_pos.y <= display_area_max.y {
            // Check if there's actually a displayed image
            if !displayed_image_query.is_empty() {
                is_hovering = true;
            }
        }
    }

    if is_hovering != *last_hover_state {
        *last_hover_state = is_hovering;

        if is_hovering {
            // Create magnifier if it doesn't exist and we have an image
            if magnifier_query.is_empty() && selected_image.handle.is_some() {
                commands.spawn((
                    Magnifier,
                    Node {
                        width: Val::Px(200.0),
                        height: Val::Px(200.0),
                        position_type: PositionType::Absolute,
                        ..default()
                    },
                    BorderColor::all(Color::BLACK),
                    BorderRadius::all(Val::Px(5.0)),
                    BackgroundColor(Color::WHITE),
                    ImageNode {
                        image: selected_image.handle.clone().unwrap(),
                        ..default()
                    },
                    ZIndex(10),
                ));
            }
        } else {
            // Remove magnifier when not hovering
            for (entity, _, _) in magnifier_query.iter() {
                commands.entity(entity).despawn();
            }
        }
    }

    if is_hovering && selected_image.handle.is_some() {
        // Update magnifier position and UV coordinates
        for (_, mut node, mut image_node) in magnifier_query.iter_mut() {
            // Position magnifier near mouse cursor (offset to avoid covering the cursor)
            node.left = Val::Px(mouse_pos.x + 20.0);
            node.top = Val::Px(mouse_pos.y + 20.0);

            // Calculate UV coordinates for the magnified area
            // The displayed image is positioned within the UI layout
            let window_size = Vec2::new(window.width(), window.height());
            let display_area_size = Vec2::new(300.0, 200.0);
            let image_size = Vec2::new(280.0, 180.0);

            // Calculate image position (centered in window)
            let display_area_pos = (window_size - display_area_size) / 2.0;
            let image_left = display_area_pos.x + 10.0; // 10px margin
            let image_top = display_area_pos.y + 10.0;  // 10px margin

            // Check if mouse is within image bounds
            if mouse_pos.x >= image_left && mouse_pos.x <= image_left + image_size.x &&
               mouse_pos.y >= image_top && mouse_pos.y <= image_top + image_size.y {

                // Mouse position relative to image (0-1 range)
                let relative_x = (mouse_pos.x - image_left) / image_size.x;
                let relative_y = (mouse_pos.y - image_top) / image_size.y;

                // 100px x 100px area in UV space
                let uv_width = 100.0 / image_size.x;   // 100px / 280px
                let uv_height = 100.0 / image_size.y;  // 100px / 180px

                // Center the UV rectangle on the mouse position
                let uv_min_x = (relative_x - uv_width / 2.0).clamp(0.0, 1.0 - uv_width);
                let uv_min_y = (relative_y - uv_height / 2.0).clamp(0.0, 1.0 - uv_height);
                let uv_max_x = uv_min_x + uv_width;
                let uv_max_y = uv_min_y + uv_height;

                // Set UV rect for magnification
                image_node.rect = Some(Rect::new(uv_min_x, uv_min_y, uv_max_x, uv_max_y));
            }
        }
    }
}