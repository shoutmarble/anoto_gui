//! This experimental example illustrates how to create widgets using the `bevy_ui_widgets` widget set.
//!
//! These widgets have no inherent styling, so this example also shows how to implement custom styles.
//!
//! The patterns shown here are likely to change substantially as the `bevy_ui_widgets` crate
//! matures, so please exercise caution if you are using this as a reference for your own code,
//! and note that there are still "user experience" issues with this API.

use bevy::{
    asset::{AssetPlugin, Handle},
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
use image::{DynamicImage, GenericImageView, ImageReader};

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
                    position: WindowPosition::At(IVec2::new(100, 50)), // Position 20px higher than typical default
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
        .init_resource::<OriginalImage>()
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

/// Marker for the secondary image display area
#[derive(Component)]
struct SecondaryImageDisplay;

/// Marker for placeholder text in secondary area when no image is selected
#[derive(Component)]
struct SecondaryPlaceholderText;

/// Marker for displayed images
#[derive(Component)]
struct DisplayedImage;

/// Marker for secondary displayed images
#[derive(Component)]
struct SecondaryDisplayedImage;

/// Marker for the magnifier rectangle overlay
#[derive(Component)]
struct MagnifierRectangle;

/// Marker for the blue border around magnified area
#[derive(Component)]
struct MagnifierBlueBorder;

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

fn secondary_image_display_area() -> impl Bundle {
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

/// Resource to store the original image buffer
#[derive(Resource, Default)]
struct OriginalImage(Option<DynamicImage>);

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
    secondary_image_display_query: Query<Entity, With<SecondaryImageDisplay>>,
    displayed_images: Query<Entity, With<DisplayedImage>>,
    secondary_displayed_images: Query<Entity, With<SecondaryDisplayedImage>>,
    placeholder_text: Query<Entity, With<PlaceholderText>>,
    secondary_placeholder_text: Query<Entity, With<SecondaryPlaceholderText>>,
    window_query: Query<&Window>,
    mut images: ResMut<Assets<Image>>,
) {
    if selected_image.path != *last_path && !selected_image.is_loading {
        *last_path = selected_image.path.clone();

        // Remove old displayed images and placeholder text
        for entity in displayed_images.iter() {
            commands.entity(entity).despawn();
        }
        for entity in secondary_displayed_images.iter() {
            commands.entity(entity).despawn();
        }

        if let Some(path) = &selected_image.path {
            println!("Loading image from path: {:?}", path);

            match ImageReader::open(path) {
                Ok(reader) => {
                    match reader.decode() {
                        Ok(img) => {
                            commands.insert_resource(OriginalImage(Some(img.clone())));
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
                            commands.insert_resource(OriginalImage(None));
                            selected_image.handle = None;
                            selected_image.dimensions = None;
                            selected_image.is_loading = false;
                        }
                    }
                }
                Err(e) => {
                    println!("Failed to open image file: {:?}", e);
                    commands.insert_resource(OriginalImage(None));
                    selected_image.handle = None;
                    selected_image.dimensions = None;
                    selected_image.is_loading = false;
                }
            }
        } else {
            commands.insert_resource(OriginalImage(None));
            selected_image.handle = None;
            selected_image.dimensions = None;
            selected_image.is_loading = false;
        }
    }

    if !selected_image.is_loading && selected_image.handle.is_some()
        && let Some(handle) = &selected_image.handle {
            println!("Displaying loaded image...");

            // Get window dimensions
            let Ok(window) = window_query.single() else { return };
            let window_width = window.width();
            let window_height = window.height();

            // Calculate available space for main display (80% width, accounting for 2px border)
            let main_available_width = (window_width * 0.8) - 4.0;
            let main_available_height = window_height - 4.0;

            // Calculate available space for secondary display (20% width, accounting for 2px border)
            let secondary_available_width = (window_width * 0.2) - 4.0;
            let secondary_available_height = window_height - 4.0;

            // Get image dimensions
            let (img_width, img_height) = selected_image.dimensions.unwrap_or((100, 100));
            let img_width_f = img_width as f32;
            let img_height_f = img_height as f32;

            // Remove placeholder text
            for text_entity in placeholder_text.iter() {
                commands.entity(text_entity).despawn();
            }
            for text_entity in secondary_placeholder_text.iter() {
                commands.entity(text_entity).despawn();
            }

            // Spawn main image with scaling and border
            let main_scale = (main_available_width / img_width_f).min(main_available_height / img_height_f);
            let main_border_color = if main_scale < 1.0 {
                Color::srgb(1.0, 0.0, 0.0) // Red for scaled down
            } else if main_scale > 1.0 {
                Color::srgb(0.0, 1.0, 0.0) // Green for scaled up
            } else {
                Color::srgb(0.5, 0.5, 0.5) // Default gray for no scaling
            };

            for container_entity in image_display_query.iter() {
                commands.entity(container_entity).with_children(|parent| {
                    parent.spawn((
                        ImageNode::new(handle.clone()),
                        DisplayedImage,
                        Hovered::default(),
                        Node {
                            width: Val::Px(img_width_f * main_scale),
                            height: Val::Px(img_height_f * main_scale),
                            border: UiRect::all(px(2)),
                            ..default()
                        },
                        BorderColor::all(main_border_color),
                    ));
                });
            }

            // Spawn secondary image with scaling and border
            let secondary_scale = (secondary_available_width / img_width_f).min(secondary_available_height / img_height_f);
            let secondary_border_color = if secondary_scale < 1.0 {
                Color::srgb(1.0, 0.0, 0.0) // Red for scaled down
            } else if secondary_scale > 1.0 {
                Color::srgb(0.0, 1.0, 0.0) // Green for scaled up
            } else {
                Color::srgb(0.5, 0.5, 0.5) // Default gray for no scaling
            };

            for container_entity in secondary_image_display_query.iter() {
                commands.entity(container_entity).with_children(|parent| {
                    parent.spawn((
                        ImageNode::new(handle.clone()),
                        SecondaryDisplayedImage,
                        Hovered::default(),
                        Node {
                            width: Val::Px(img_width_f * secondary_scale),
                            height: Val::Px(img_height_f * secondary_scale),
                            border: UiRect::all(px(2)),
                            ..default()
                        },
                        BorderColor::all(secondary_border_color),
                    ));
                });
            }

            selected_image.handle = None;
        }
}

fn magnifier_system(
    mut commands: Commands,
    mut param_set: ParamSet<(
        Query<(Entity, &mut Node, &mut ImageNode), With<Magnifier>>,
        Query<(&ImageNode, &Node), With<DisplayedImage>>,
        Query<(Entity, &mut Node, &mut ImageNode), With<MagnifierRectangle>>,
        Query<&mut ImageNode, With<SecondaryDisplayedImage>>,
        Query<(Entity, &mut Node, &mut ImageNode), With<MagnifierBlueBorder>>,
    )>,
    window_query: Query<&Window>,
    selected_image: Res<SelectedImage>,
    mut images: ResMut<Assets<Image>>,
    original_image: Res<OriginalImage>,
) {
    let Ok(window) = window_query.single() else { return };

    let mut is_hovering = false;
    let mut mouse_pos = Vec2::ZERO;
    let mut image_handle: Option<Handle<Image>> = None;
    let mut actual_image_bounds = Rect::default();

    // Check if mouse is hovering over either displayed image area
    if let Some(cursor_pos) = window.cursor_position() {
        mouse_pos = cursor_pos;

        let window_width = window.width();
        let window_height = window.height();

        // Check main image area (left column, 80% width)
        let left_column_width = window_width * 0.8;
        let (main_image_width, main_image_height) = if let Some((img_width, img_height)) = selected_image.dimensions {
            let aspect_ratio = img_width as f32 / img_height as f32;
            let displayed_width = left_column_width;
            let displayed_height = displayed_width / aspect_ratio;
            (displayed_width, displayed_height)
        } else {
            let displayed_width = left_column_width;
            let estimated_height = displayed_width * 0.6;
            (displayed_width, estimated_height)
        };

        let main_image_left = 0.0;
        let main_image_top = (window_height - main_image_height) / 2.0;
        let main_image_right = main_image_left + main_image_width;
        let main_image_bottom = main_image_top + main_image_height;

        // Check secondary image area (right column, 20% width)
        let right_column_start = window_width * 0.8;
        let right_column_width = window_width * 0.2;
        let (secondary_image_width, secondary_image_height) = if let Some((img_width, img_height)) = selected_image.dimensions {
            let aspect_ratio = img_width as f32 / img_height as f32;
            let displayed_width = right_column_width;
            let displayed_height = displayed_width / aspect_ratio;
            (displayed_width, displayed_height)
        } else {
            let displayed_width = right_column_width;
            let estimated_height = displayed_width * 0.6;
            (displayed_width, estimated_height)
        };

        let secondary_image_left = right_column_start;
        let secondary_image_top = (window_height - secondary_image_height) / 2.0;
        let secondary_image_right = secondary_image_left + secondary_image_width;
        let secondary_image_bottom = secondary_image_top + secondary_image_height;

        // Check which area the mouse is over
        if cursor_pos.x >= main_image_left && cursor_pos.x <= main_image_right &&
           cursor_pos.y >= main_image_top && cursor_pos.y <= main_image_bottom {
            is_hovering = true;
            actual_image_bounds = Rect::new(main_image_left, main_image_top, main_image_right, main_image_bottom);
            // Get image handle from any displayed image
            if let Some((displayed_image, _)) = param_set.p1().iter().next() {
                image_handle = Some(displayed_image.image.clone());
            }
        } else if cursor_pos.x >= secondary_image_left && cursor_pos.x <= secondary_image_right &&
                  cursor_pos.y >= secondary_image_top && cursor_pos.y <= secondary_image_bottom {
            is_hovering = true;
            actual_image_bounds = Rect::new(secondary_image_left, secondary_image_top, secondary_image_right, secondary_image_bottom);
            // Get image handle from any displayed image
            if let Some((displayed_image, _)) = param_set.p1().iter().next() {
                image_handle = Some(displayed_image.image.clone());
            }
        }

        // Update secondary image display with magnified content if hovering over main image
        if cursor_pos.x >= main_image_left && cursor_pos.x <= main_image_right &&
           cursor_pos.y >= main_image_top && cursor_pos.y <= main_image_bottom {
            let relative_x = ((mouse_pos.x - main_image_left) / main_image_width).clamp(0.0, 1.0);
            let relative_y = ((mouse_pos.y - main_image_top) / main_image_height).clamp(0.0, 1.0);

            let magnify_area_px = 100.0;
            let uv_width = magnify_area_px / main_image_width;
            let uv_height = magnify_area_px / main_image_height;

            let uv_center_x = relative_x;
            let uv_center_y = relative_y;
            let uv_min_x = (uv_center_x - uv_width / 2.0).max(0.0);
            let uv_min_y = (uv_center_y - uv_height / 2.0).max(0.0);
            let uv_max_x = (uv_center_x + uv_width / 2.0).min(1.0);
            let uv_max_y = (uv_center_y + uv_height / 2.0).min(1.0);

            // Update secondary image display
            for mut secondary_image_node in param_set.p3().iter_mut() {
                secondary_image_node.rect = Some(Rect {
                    min: Vec2::new(uv_min_x, uv_min_y),
                    max: Vec2::new(uv_max_x, uv_max_y),
                });
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

            let magnify_area_px = 100.0; // Crop 100x100 area for 1:1 display in 100x100 popup
            let uv_width = magnify_area_px / image_width;
            let uv_height = magnify_area_px / image_height;
            
            let uv_center_x = relative_x;
            let uv_center_y = relative_y;
            let uv_min_x = (uv_center_x - uv_width / 2.0).max(0.0);
            let uv_min_y = (uv_center_y - uv_height / 2.0).max(0.0);
            let uv_max_x = (uv_center_x + uv_width / 2.0).min(1.0);
            let uv_max_y = (uv_center_y + uv_height / 2.0).min(1.0);

            // Debug output for magnifier creation
            println!("Creating magnifier - UV coords: min=({:.3}, {:.3}) max=({:.3}, {:.3})", uv_min_x, uv_min_y, uv_max_x, uv_max_y);
            
            if let Some(ref img) = original_image.0 {
                let img_width_f = img.width() as f32;
                let img_height_f = img.height() as f32;
                let pixel_x = relative_x * img_width_f;
                let pixel_y = relative_y * img_height_f;
                let half_crop = magnify_area_px / 2.0;
                let start_x = (pixel_x - half_crop).max(0.0) as u32;
                let start_y = (pixel_y - half_crop).max(0.0) as u32;
                let crop_w = ((start_x + magnify_area_px as u32).min(img.width()) - start_x).max(1);
                let crop_h = ((start_y + magnify_area_px as u32).min(img.height()) - start_y).max(1);
                let cropped = img.view(start_x, start_y, crop_w, crop_h).to_image();
                let rgba_cropped = cropped.into_raw();
                let bevy_cropped = Image::new(
                    bevy::render::render_resource::Extent3d {
                        width: crop_w,
                        height: crop_h,
                        depth_or_array_layers: 1,
                    },
                    bevy::render::render_resource::TextureDimension::D2,
                    rgba_cropped,
                    bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
                    Default::default(),
                );
                let cropped_handle = images.add(bevy_cropped);
                
                commands.spawn((
                    Magnifier,
                    Node {
                        width: Val::Px(100.0),
                        height: Val::Px(100.0),
                        position_type: PositionType::Absolute,
                        left: Val::Px(mouse_pos.x - 50.0), // Center the 100x100 magnifier at mouse position
                        top: Val::Px(mouse_pos.y - 50.0),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BorderColor::all(Color::BLACK),
                    ZIndex(1000), // Ensure magnifier appears on top
                    ImageNode::new(cropped_handle.clone()),
                ));
                
                // Create blue border - 400x400 pixels positioned to the right of red square + 5px
                commands.spawn((
                    MagnifierBlueBorder,
                    Node {
                        width: Val::Px(400.0), // 400x400 pixels
                        height: Val::Px(400.0),
                        position_type: PositionType::Absolute,
                        left: Val::Px(mouse_pos.x + 55.0), // Right of red square + 5px (red right edge at mouse_pos.x + 50)
                        top: Val::Px(mouse_pos.y - 200.0), // Centered vertically on mouse position
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BorderColor::all(Color::srgb(0.0, 0.0, 1.0)), // Blue border
                    BackgroundColor(Color::NONE), // Transparent background
                    ZIndex(999), // Just below magnifier
                    ImageNode::new(cropped_handle.clone()), // Show the magnified content
                ));
            }
            
            // Red square acts as transparent hover - no image content, just border
            commands.spawn((
                MagnifierRectangle,
                Node {
                    width: Val::Px(100.0), // 100x100 pixels
                    height: Val::Px(100.0),
                    position_type: PositionType::Absolute,
                    left: Val::Px(mouse_pos.x - 50.0), // Centered at mouse position
                    top: Val::Px(mouse_pos.y - 50.0),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BorderColor::all(Color::srgb(1.0, 0.0, 0.0)), // Red border
                BackgroundColor(Color::NONE), // Transparent background
                ZIndex(1001), // On top of everything
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

            let magnify_area_px = 100.0; // Crop 100x100 area for 1:1 display in 100x100 popup
            let _uv_width = magnify_area_px / image_width;
            let _uv_height = magnify_area_px / image_height;

            // Calculate pixel coordinates for cropping
            if let Some(ref img) = original_image.0 {
                let img_width_f = img.width() as f32;
                let img_height_f = img.height() as f32;
                let pixel_x = relative_x * img_width_f;
                let pixel_y = relative_y * img_height_f;
                let half_crop = magnify_area_px / 2.0;
                let start_x = (pixel_x - half_crop).max(0.0) as u32;
                let start_y = (pixel_y - half_crop).max(0.0) as u32;
                let crop_w = ((start_x + magnify_area_px as u32).min(img.width()) - start_x).max(1);
                let crop_h = ((start_y + magnify_area_px as u32).min(img.height()) - start_y).max(1);
                let cropped = img.view(start_x, start_y, crop_w, crop_h).to_image();
                let rgba_cropped = cropped.into_raw();
                let bevy_cropped = Image::new(
                    bevy::render::render_resource::Extent3d {
                        width: crop_w,
                        height: crop_h,
                        depth_or_array_layers: 1,
                    },
                    bevy::render::render_resource::TextureDimension::D2,
                    rgba_cropped,
                    bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
                    Default::default(),
                );
                let cropped_handle = images.add(bevy_cropped);
                
                // Update existing magnifier
                for (_entity, mut node, mut image_node) in param_set.p0().iter_mut() {
                    image_node.image = cropped_handle.clone();
                    image_node.rect = None;
                    
                    // Calculate red rectangle dimensions for blue border positioning
                    let (_rect_width, _rect_height) = if let Some((img_width, img_height)) = selected_image.dimensions {
                        let window_width = window.width();
                        let left_column_width = window_width * 0.8;
                        let aspect_ratio = img_width as f32 / img_height as f32;
                        let displayed_width = left_column_width;
                        let displayed_height = displayed_width / aspect_ratio;
                        
                        let scale_x = displayed_width / img_width as f32;
                        let scale_y = displayed_height / img_height as f32;
                        (100.0 * scale_x, 100.0 * scale_y)
                    } else {
                        (100.0, 100.0)
                    };
                    
                    // Position magnifier centered at mouse position
                    let mut magnifier_left = mouse_pos.x - 50.0;
                    let mut magnifier_top = mouse_pos.y - 50.0;
                    
                    // Adjust position if magnifier would go off-screen
                    if magnifier_left + 100.0 > window.width() {
                        magnifier_left = window.width() - 100.0;
                    }
                    if magnifier_left < 0.0 {
                        magnifier_left = 0.0;
                    }
                    if magnifier_top + 100.0 > window.height() {
                        magnifier_top = window.height() - 100.0;
                    }
                    if magnifier_top < 0.0 {
                        magnifier_top = 0.0;
                    }
                    
                    node.left = Val::Px(magnifier_left);
                    node.top = Val::Px(magnifier_top);
                }
                
                // Update blue border position
                for (_entity, mut border_node, mut border_image) in param_set.p4().iter_mut() {
                    border_image.image = cropped_handle.clone();
                    border_image.rect = None;
                    
                // Position blue border to the right of red square + 5px
                let mut border_left = mouse_pos.x + 55.0; // Red square right edge + 5px
                let mut border_top = mouse_pos.y - 200.0; // Centered vertically (400/2 = 200)
                
                // Adjust position if it would go off-screen
                if border_left + 400.0 > window.width() {
                    border_left = mouse_pos.x - 50.0 - 400.0 - 5.0; // Move to left side of red square
                }
                if border_top + 400.0 > window.height() {
                    border_top = window.height() - 400.0; // Move to bottom
                }
                if border_top < 0.0 {
                    border_top = 0.0; // Move to top
                }                    border_node.left = Val::Px(border_left);
                    border_node.top = Val::Px(border_top);
                }
            }
            
                        // Update rectangle overlay position
            for (_, mut rect_node, _) in param_set.p2().iter_mut() {
                let rect_left = mouse_pos.x - 50.0; // Fixed 100x100 centered at mouse
                let rect_top = mouse_pos.y - 50.0;
                
                rect_node.left = Val::Px(rect_left);
                rect_node.top = Val::Px(rect_top);
                rect_node.width = Val::Px(100.0); // Fixed size
                rect_node.height = Val::Px(100.0);
            }

            // Update secondary image display with magnified content
            let relative_x = ((mouse_pos.x - image_left) / image_width).clamp(0.0, 1.0);
            let relative_y = ((mouse_pos.y - image_top) / image_height).clamp(0.0, 1.0);

            let magnify_area_px = 100.0;
            let uv_width = magnify_area_px / image_width;
            let uv_height = magnify_area_px / image_height;

            let uv_center_x = relative_x;
            let uv_center_y = relative_y;
            let uv_min_x = (uv_center_x - uv_width / 2.0).max(0.0);
            let uv_min_y = (uv_center_y - uv_height / 2.0).max(0.0);
            let uv_max_x = (uv_center_x + uv_width / 2.0).min(1.0);
            let uv_max_y = (uv_center_y + uv_height / 2.0).min(1.0);

            // Update secondary image display
            for mut secondary_image_node in param_set.p3().iter_mut() {
                secondary_image_node.rect = Some(Rect {
                    min: Vec2::new(uv_min_x, uv_min_y),
                    max: Vec2::new(uv_max_x, uv_max_y),
                });
            }
        }
    } else {
        // Move magnifier and rectangles off-screen when not hovering (instead of despawning to avoid artifacts)
        for (_entity, mut node, _) in param_set.p0().iter_mut() {
            node.left = Val::Px(-200.0); // Move off-screen
            node.top = Val::Px(-200.0);
        }
        for (_entity, mut rect_node, _) in param_set.p2().iter_mut() {
            rect_node.left = Val::Px(-200.0); // Move off-screen
            rect_node.top = Val::Px(-200.0);
        }
        for (_entity, mut border_node, _) in param_set.p4().iter_mut() {
            border_node.left = Val::Px(-600.0); // Move off-screen (400px width + margin)
            border_node.top = Val::Px(-600.0);
        }

        // Reset secondary image display to show full image
        for mut secondary_image_node in param_set.p3().iter_mut() {
            secondary_image_node.rect = None; // None means show full image
        }
    }
}