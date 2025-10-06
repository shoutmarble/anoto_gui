//! This experimental example illustrates how to create widgets using the `bevy_ui_widgets` widget set.
//!
//! These widgets have no inherent styling, so this example also shows how to implement custom styles.
//!
//! The patterns shown here are likely to change substantially as the `bevy_ui_widgets` crate
//! matures, so please exercise caution if you are using this as a reference for your own code,
//! and note that there are still "user experience" issues with this API.

use bevy::{
    asset::AssetPlugin,
    ecs::system::ParamSet,
    input_focus::tab_navigation::{TabGroup, TabIndex},
    picking::hover::Hovered,
    prelude::*,
    window::{Window, WindowMode, WindowPlugin, WindowPosition, WindowResolution, PresentMode},
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
        .add_systems(Update, image_select_system)
        .add_systems(Update, dialog_system)
        .add_systems(Update, update_image_display)
        .add_systems(Update, magnifier_system)
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
    dimensions: Option<(u32, u32)>,
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

fn demo_root(asset_server: &AssetServer) -> impl Bundle {
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
            // Right column: Button (2/10 width)
            (
                Node {
                    width: percent(20),
                    height: percent(100),
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
        ],
    )
}

fn image_select_button(_asset_server: &AssetServer) -> impl Bundle {
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

#[allow(clippy::type_complexity)]
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
                    dimensions: None,
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
                            selected_image.dimensions = Some((width, height));
                            selected_image.is_loading = false;

                            println!("Image loaded successfully: {}x{}", width, height);
                        }
                        Err(e) => {
                            println!("Failed to decode image: {:?}", e);
                            selected_image.handle = None;
                            selected_image.dimensions = None;
                            selected_image.is_loading = false;
                        }
                    }
                }
                Err(e) => {
                    println!("Failed to open image file: {:?}", e);
                    selected_image.handle = None;
                    selected_image.dimensions = None;
                    selected_image.is_loading = false;
                }
            }
        } else {
            selected_image.handle = None;
            selected_image.dimensions = None;
            selected_image.is_loading = false;
        }
    }

    if !selected_image.is_loading && selected_image.handle.is_some()
        && let Some(handle) = &selected_image.handle {
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
                            width: Val::Percent(100.0), // Fill the entire container width
                            height: Val::Auto, // Auto height to maintain aspect ratio
                            ..default()
                        },
                    ));
                });
            }

            selected_image.handle = None;
        }
}

#[derive(Component)]
struct DisplayedImage;

#[derive(Component)]
struct MagnifierRectangle;

fn magnifier_system(
    mut commands: Commands,
    mut param_set: ParamSet<(
        Query<(Entity, &mut Node, &mut ImageNode), With<Magnifier>>,
        Query<(&ImageNode, &Node), With<DisplayedImage>>,
        Query<(Entity, &mut Node), With<MagnifierRectangle>>,
    )>,
    window_query: Query<&Window>,
    selected_image: Res<SelectedImage>,
) {
    let Ok(window) = window_query.single() else { return };

    let mut is_hovering = false;
    let mut mouse_pos = Vec2::ZERO;
    let mut image_handle: Option<Handle<Image>> = None;
    let mut actual_image_bounds = Rect::default();

    // Check if mouse is hovering over the displayed image area
    if let Some(cursor_pos) = window.cursor_position() {
        mouse_pos = cursor_pos;

        // Get the actual bounds of the displayed image
        if let Ok((displayed_image, _image_node)) = param_set.p1().single() {
            // Calculate the actual rendered size and position of the image
            // Image is now in the left column (80% of window width) and fills 100% of that column
            let window_width = window.width();
            let window_height = window.height();
            
            // Left column takes 80% of window width
            let left_column_width = window_width * 0.8;
            
            // Get actual image dimensions if available
            let (image_width, image_height) = if let Some((img_width, img_height)) = selected_image.dimensions {
                // Calculate displayed dimensions maintaining aspect ratio
                let aspect_ratio = img_width as f32 / img_height as f32;
                let displayed_width = left_column_width;
                let displayed_height = displayed_width / aspect_ratio;
                (displayed_width, displayed_height)
            } else {
                // Fallback to estimated dimensions if not available
                let displayed_width = left_column_width;
                let estimated_height = displayed_width * 0.6; // Assuming roughly 16:10 aspect ratio
                (displayed_width, estimated_height)
            };
            
            // Image starts at the left edge of the left column
            let image_left = 0.0;
            let image_top = (window_height - image_height) / 2.0; // Center vertically
            let image_right = image_left + image_width;
            let image_bottom = image_top + image_height;
            
            actual_image_bounds = Rect::new(image_left, image_top, image_right, image_bottom);
            
            // Check if mouse is within the actual image bounds
            if cursor_pos.x >= image_left && cursor_pos.x <= image_right &&
               cursor_pos.y >= image_top && cursor_pos.y <= image_bottom {
                is_hovering = true;
                image_handle = Some(displayed_image.image.clone());
            }
        }
    }

    if is_hovering {
        if param_set.p0().is_empty() && image_handle.is_some() {
            // Calculate initial UV rect for the current mouse position
            let image_left = actual_image_bounds.min.x;
            let image_top = actual_image_bounds.min.y;
            let image_width = actual_image_bounds.width();
            let image_height = actual_image_bounds.height();
            
            let relative_x = ((mouse_pos.x - image_left) / image_width).clamp(0.0, 1.0);
            let relative_y = ((mouse_pos.y - image_top) / image_height).clamp(0.0, 1.0);
            
            let magnify_area_px = 50.0;
            let uv_width = magnify_area_px / image_width;
            let uv_height = magnify_area_px / image_height;
            
            let uv_center_x = relative_x;
            let uv_center_y = relative_y;
            let uv_min_x = (uv_center_x - uv_width / 2.0).max(0.0);
            let uv_min_y = (uv_center_y - uv_height / 2.0).max(0.0);
            let uv_max_x = (uv_center_x + uv_width / 2.0).min(1.0);
            let uv_max_y = (uv_center_y + uv_height / 2.0).min(1.0);
            
            commands.spawn((
                Magnifier,
                Node {
                    width: Val::Px(200.0),
                    height: Val::Px(200.0),
                    position_type: PositionType::Absolute,
                    left: Val::Px(mouse_pos.x + 20.0),
                    top: Val::Px(mouse_pos.y + 20.0),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BorderColor::all(Color::BLACK),
                BorderRadius::all(Val::Px(5.0)),
                ImageNode {
                    image: image_handle.clone().unwrap(),
                    rect: Some(Rect::new(uv_min_x, uv_min_y, uv_max_x, uv_max_y)),
                    flip_y: true,
                    ..default()
                },
                ZIndex(1000),
            ));
            
            // Create rectangle overlay on the original image
            let rect_size = 50.0;
            let rect_left = mouse_pos.x - rect_size / 2.0;
            let rect_top = mouse_pos.y - rect_size / 2.0;
            
            commands.spawn((
                MagnifierRectangle,
                Node {
                    width: Val::Px(50.0),
                    height: Val::Px(50.0),
                    position_type: PositionType::Absolute,
                    left: Val::Px(rect_left),
                    top: Val::Px(rect_top),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BorderColor::all(Color::srgb(1.0, 0.0, 0.0)), // Red border
                BackgroundColor(Color::NONE), // Transparent background
                ZIndex(999), // Just below magnifier
            ));
        } else if !param_set.p0().is_empty() {
            // Update existing magnifier and rectangle
            // Use the actual image bounds we calculated above
            let image_left = actual_image_bounds.min.x;
            let image_top = actual_image_bounds.min.y;
            let image_width = actual_image_bounds.width();
            let image_height = actual_image_bounds.height();
            
            // Calculate new UV rect
            let relative_x = ((mouse_pos.x - image_left) / image_width).clamp(0.0, 1.0);
            let relative_y = ((mouse_pos.y - image_top) / image_height).clamp(0.0, 1.0);

            let magnify_area_px = 50.0;
            let uv_width = magnify_area_px / image_width;
            let uv_height = magnify_area_px / image_height;

            let uv_center_x = relative_x;
            let uv_center_y = relative_y;
            let uv_min_x = (uv_center_x - uv_width / 2.0).max(0.0);
            let uv_min_y = (uv_center_y - uv_height / 2.0).max(0.0);
            let uv_max_x = (uv_center_x + uv_width / 2.0).min(1.0);
            let uv_max_y = (uv_center_y + uv_height / 2.0).min(1.0);
            
            let new_rect = Rect::new(uv_min_x, uv_min_y, uv_max_x, uv_max_y);
            
            // Recreate magnifier with new rect
            for (entity, node, _) in param_set.p0().iter() {
                commands.entity(entity).despawn();
                
                commands.spawn((
                    Magnifier,
                    Node {
                        width: Val::Px(200.0),
                        height: Val::Px(200.0),
                        position_type: PositionType::Absolute,
                        left: node.left,
                        top: node.top,
                        border: node.border,
                        ..default()
                    },
                    BorderColor::all(Color::BLACK),
                    BorderRadius::all(Val::Px(5.0)),
                    ImageNode {
                        image: image_handle.clone().unwrap(),
                        rect: Some(new_rect),
                        flip_y: true,
                        ..default()
                    },
                    ZIndex(1000),
                ));
            }
            
            // Update rectangle overlay position
            for (_, mut rect_node) in param_set.p2().iter_mut() {
                // Position rectangle centered on mouse position (we already know mouse is within bounds)
                let rect_size = 50.0;
                let rect_left = mouse_pos.x - rect_size / 2.0;
                let rect_top = mouse_pos.y - rect_size / 2.0;
                
                rect_node.left = Val::Px(rect_left);
                rect_node.top = Val::Px(rect_top);
            }
        }
    } else {
        // Remove magnifier and rectangle when not hovering
        for (entity, _, _) in param_set.p0().iter() {
            commands.entity(entity).despawn();
        }
        for (entity, _) in param_set.p2().iter() {
            commands.entity(entity).despawn();
        }
    }
}