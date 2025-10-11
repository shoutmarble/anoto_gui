//! Overlay zoom/magnifier functionality for the Anoto GUI

use bevy::prelude::*;
use crate::bevy::utils::*;
use image::GenericImageView;

/// System that handles the magnifier overlay functionality
pub fn magnifier_system(
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