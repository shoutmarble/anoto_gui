//! Image loading, scaling, and display logic for the Anoto GUI

use bevy::prelude::*;
use bevy::picking::hover::Hovered;
use image::{DynamicImage, GenericImageView, ImageReader};
use std::path::PathBuf;
use crate::bevy::utils::*;

/// Updates the image display with scaling and border indicators
pub fn update_image_display(
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