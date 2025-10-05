//! This experimental example illustrates how to create widgets using the `bevy_ui_widgets` widget set.
//!
//! These widgets have no inherent styling, so this example also shows how to implement custom styles.
//!
//! The patterns shown here are likely to change substantially as the `bevy_ui_widgets` crate
//! matures, so please exercise caution if you are using this as a reference for your own code,
//! and note that there are still "user experience" issues with this API.

use bevy::{
    input_focus::tab_navigation::{TabGroup, TabIndex},
    picking::hover::Hovered,
    prelude::*,
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
            DefaultPlugins,
            UiWidgetsPlugins,
        ))
        .init_resource::<SelectedImage>()
        .init_resource::<DialogState>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                image_select_system,
                dialog_system,
                update_image_display,
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
                font: asset_server.load("fonts/icons.ttf"),
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