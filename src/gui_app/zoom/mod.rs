use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
    window::{PrimaryWindow, Window},
};
use image::GenericImageView;

use crate::gui_app::{
    layout::{ZoomSizer, ZoomSizerLabel, ZoomSquare},
    loader::push_dynamic_image,
    state::{CursorState, GuiImageState, LayoutMetrics, ZoomCapturedEvent, ZoomSettings},
};

pub struct ZoomPlugin;

impl Plugin for ZoomPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                track_cursor_position,
                update_zoom_square,
                handle_zoom_sizer_interaction,
                respond_to_zoom_scroll,
                refresh_zoom_label,
                capture_zoom_preview,
            ),
        );
    }
}

fn track_cursor_position(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut cursor: ResMut<CursorState>,
) {
    if let Some(window) = windows.iter().next() {
        cursor.window_position = window.cursor_position().map(|mut position| {
            // Convert from window (origin at top-left) to UI space (origin at bottom-left)
            let window_height = window.size().y;
            position.y = window_height - position.y;
            position
        });
    }
}

fn update_zoom_square(
    metrics: Res<LayoutMetrics>,
    zoom: Res<ZoomSettings>,
    cursor: Res<CursorState>,
    mut square_query: Query<(&mut Node, &mut Visibility), With<ZoomSquare>>,
) {
    if metrics.is_changed() || zoom.is_changed() || cursor.is_changed() {
        if let Ok((mut node, mut visibility)) = square_query.single_mut() {
            if let (Some(rect), Some(cursor_pos)) = (metrics.image_rect, cursor.window_position) {
                if rect.contains(cursor_pos) {
                    let size = zoom
                        .square_size
                        .min(rect.width())
                        .min(rect.height());
                    let clamped_left = (cursor_pos.x - size * 0.5).clamp(rect.min.x, rect.max.x - size);
                    let clamped_bottom = (cursor_pos.y - size * 0.5).clamp(rect.min.y, rect.max.y - size);
                    let local_left = clamped_left - metrics.left_panel.min.x;
                    let local_bottom = clamped_bottom - metrics.left_panel.min.y;

                    node.width = Val::Px(size);
                    node.height = Val::Px(size);
                    node.left = Val::Px(local_left);
                    node.bottom = Val::Px(local_bottom);
                    *visibility = Visibility::Visible;
                } else {
                    *visibility = Visibility::Hidden;
                }
            } else {
                *visibility = Visibility::Hidden;
            }
        }
    }
}

fn handle_zoom_sizer_interaction(
    mut sizer: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<ZoomSizer>)>,
) {
    for (interaction, mut color) in &mut sizer {
        match *interaction {
            Interaction::Pressed => *color = BackgroundColor(Color::srgb(0.45, 0.45, 0.65)),
            Interaction::Hovered => *color = BackgroundColor(Color::srgb(0.4, 0.4, 0.6)),
            Interaction::None => *color = BackgroundColor(Color::srgb(0.3, 0.3, 0.5)),
        }
    }
}

fn respond_to_zoom_scroll(
    mut scroll_events: MessageReader<MouseWheel>,
    sizer: Query<&Interaction, With<ZoomSizer>>,
    mut zoom: ResMut<ZoomSettings>,
) {
    let hovered = sizer.iter().any(|interaction| match interaction {
        Interaction::Hovered | Interaction::Pressed => true,
        Interaction::None => false,
    });

    if !hovered {
        for _ in scroll_events.read() {}
        return;
    }

    let mut delta = 0.0_f32;
    for event in scroll_events.read() {
        let step = match event.unit {
            MouseScrollUnit::Line => 12.0,
            MouseScrollUnit::Pixel => 1.0,
        };
        delta += event.y * step as f32;
    }

    if delta.abs() > f32::EPSILON {
        zoom.square_size = (zoom.square_size + delta).clamp(zoom.min_square, zoom.max_square);
    }
}

fn refresh_zoom_label(
    zoom: Res<ZoomSettings>,
    mut labels: Query<&mut Text, With<ZoomSizerLabel>>,
) {
    if !zoom.is_changed() {
        return;
    }

    for mut text in &mut labels {
        text.0 = format!("{:.0}px", zoom.square_size);
    }
}

fn capture_zoom_preview(
    buttons: Res<ButtonInput<MouseButton>>,
    cursor: Res<CursorState>,
    metrics: Res<LayoutMetrics>,
    zoom: Res<ZoomSettings>,
    gui_image: Res<GuiImageState>,
    mut images: ResMut<Assets<Image>>,
    mut writer: MessageWriter<ZoomCapturedEvent>,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }

    let Some(cursor_pos) = cursor.window_position else {
        return;
    };
    let Some(display_rect) = metrics.image_rect else {
        return;
    };
    if !display_rect.contains(cursor_pos) {
        return;
    }
    let Some(original) = gui_image.original.as_ref() else {
        return;
    };

    let display_size = Vec2::new(display_rect.width(), display_rect.height());
    let square_display = zoom.square_size.min(display_size.x).min(display_size.y);
    let clamped_left = (cursor_pos.x - square_display * 0.5).clamp(display_rect.min.x, display_rect.max.x - square_display);
    let clamped_bottom = (cursor_pos.y - square_display * 0.5).clamp(display_rect.min.y, display_rect.max.y - square_display);

    let rel_left = (clamped_left - display_rect.min.x) / display_size.x;
    let rel_bottom = (clamped_bottom - display_rect.min.y) / display_size.y;

    let (orig_w, orig_h) = original.dimensions();
    let crop_w = ((square_display / display_size.x) * orig_w as f32).round().clamp(4.0, orig_w as f32) as u32;
    let crop_h = ((square_display / display_size.y) * orig_h as f32).round().clamp(4.0, orig_h as f32) as u32;

    let px_left = (rel_left * orig_w as f32).round().clamp(0.0, (orig_w - crop_w) as f32) as u32;
    let px_bottom = (rel_bottom * orig_h as f32).round().clamp(0.0, (orig_h - crop_h) as f32) as u32;
    let px_top = orig_h.saturating_sub(px_bottom + crop_h);

    let sub_image = original.crop_imm(px_left, px_top, crop_w, crop_h);
    if let Some(handle) = push_dynamic_image(&sub_image, &mut images) {
        let aspect_ratio = crop_w.max(1) as f32 / crop_h.max(1) as f32;
        writer.write(ZoomCapturedEvent {
            handle,
            aspect_ratio,
        });
    }
}
