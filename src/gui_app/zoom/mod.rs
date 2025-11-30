use bevy::{
    ecs::observer::On,
    prelude::*,
    window::{PrimaryWindow, Window},
};
use bevy_ui_widgets::{SliderRange, SliderValue, ValueChange, slider_self_update};
use image::GenericImageView;

use crate::gui_app::{
    layout::{ZoomSlider, ZoomSliderThumb, ZoomSliderTrack, ZoomSliderValue, ZoomSquare},
    loader::push_dynamic_image,
    state::{
        CursorState, GuiImageState, ImageLoadedEvent, LayoutMetrics, ZoomCapturedEvent,
        ZoomSettings,
    },
};

type SliderValueQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Children,
        &'static SliderValue,
        &'static SliderRange,
    ),
    (Changed<SliderValue>, With<ZoomSlider>),
>;

type SliderTrackQuery<'w, 's> = Query<
    'w,
    's,
    (&'static Children, &'static Node),
    (With<ZoomSliderTrack>, Without<ZoomSliderThumb>),
>;

type SliderThumbQuery<'w, 's> =
    Query<'w, 's, &'static mut Node, (With<ZoomSliderThumb>, Without<ZoomSliderTrack>)>;

pub struct ZoomPlugin;

impl Plugin for ZoomPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(slider_self_update)
            .add_observer(handle_zoom_slider_change)
            .add_systems(
                Update,
                (
                    track_cursor_position,
                    update_zoom_square,
                    update_zoom_slider_value_text,
                    capture_zoom_preview,
                    reset_zoom_slider_on_image_load,
                ),
            )
            .add_systems(PostUpdate, update_zoom_slider_thumb);
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
    if (metrics.is_changed() || zoom.is_changed() || cursor.is_changed())
        && let Ok((mut node, mut visibility)) = square_query.single_mut()
    {
        if let (Some(rect), Some(cursor_pos)) = (metrics.image_rect, cursor.window_position) {
            let size = square_size_for_rect(rect, &zoom);
            if rect.contains(cursor_pos) && size > 0.0 {
                let clamped_left = (cursor_pos.x - size * 0.5).clamp(rect.min.x, rect.max.x - size);
                let clamped_bottom =
                    (cursor_pos.y - size * 0.5).clamp(rect.min.y, rect.max.y - size);
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

fn update_zoom_slider_thumb(
    slider_query: SliderValueQuery<'_, '_>,
    tracks: SliderTrackQuery<'_, '_>,
    mut thumbs: SliderThumbQuery<'_, '_>,
) {
    for (children, value, range) in &slider_query {
        let normalized = range.thumb_position(value.0).clamp(0.0, 1.0);

        for child in children.iter() {
            if let Ok((track_children, _)) = tracks.get(child) {
                // Approximate the left position to account for thumb width
                // Assuming track_width ≈ 500px, thumb_width = 18px, so scale by (1 - 18/500) ≈ 0.964
                let scaled_normalized = normalized * 0.964;
                let left_percent = scaled_normalized * 100.0;

                for thumb_child in track_children.iter() {
                    if let Ok(mut thumb_node) = thumbs.get_mut(thumb_child) {
                        thumb_node.left = Val::Percent(left_percent);
                    }
                }
                break; // Assume only one track per slider
            }
        }
    }
}

fn update_zoom_slider_value_text(
    zoom: Res<ZoomSettings>,
    mut labels: Query<&mut Text, With<ZoomSliderValue>>,
) {
    if !zoom.is_changed() {
        return;
    }

    let percent = zoom.normalized_percent() * 100.0;
    for mut text in &mut labels {
        text.0 = format!("{percent:.0}%");
    }
}

fn handle_zoom_slider_change(
    value_change: On<ValueChange<f32>>,
    slider_query: Query<(), With<ZoomSlider>>,
    mut zoom: ResMut<ZoomSettings>,
) {
    if slider_query.get(value_change.source).is_ok() {
        zoom.apply_slider_value(value_change.value);
    }
}

fn reset_zoom_slider_on_image_load(
    mut events: MessageReader<ImageLoadedEvent>,
    mut zoom: ResMut<ZoomSettings>,
    slider_query: Query<Entity, With<ZoomSlider>>,
    mut commands: Commands,
) {
    let mut reset = false;
    for _ in events.read() {
        zoom.reset_to_default();
        reset = true;
    }

    if reset && let Some(slider_entity) = slider_query.iter().next() {
        commands
            .entity(slider_entity)
            .insert(SliderValue(zoom.slider_value()));
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
    let square_display = square_size_for_rect(display_rect, &zoom);
    if square_display <= 0.0 {
        return;
    }
    let clamped_left = (cursor_pos.x - square_display * 0.5)
        .clamp(display_rect.min.x, display_rect.max.x - square_display);
    let clamped_bottom = (cursor_pos.y - square_display * 0.5)
        .clamp(display_rect.min.y, display_rect.max.y - square_display);

    let rel_left = (clamped_left - display_rect.min.x) / display_size.x;
    let rel_bottom = (clamped_bottom - display_rect.min.y) / display_size.y;

    let (orig_w, orig_h) = original.dimensions();
    let crop_w = ((square_display / display_size.x) * orig_w as f32)
        .round()
        .clamp(4.0, orig_w as f32) as u32;
    let crop_h = ((square_display / display_size.y) * orig_h as f32)
        .round()
        .clamp(4.0, orig_h as f32) as u32;

    let px_left = (rel_left * orig_w as f32)
        .round()
        .clamp(0.0, (orig_w - crop_w) as f32) as u32;
    let px_bottom = (rel_bottom * orig_h as f32)
        .round()
        .clamp(0.0, (orig_h - crop_h) as f32) as u32;
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

fn square_size_for_rect(rect: Rect, zoom: &ZoomSettings) -> f32 {
    let min_dimension = rect.width().min(rect.height());
    if min_dimension <= 0.0 {
        0.0
    } else {
        zoom.normalized_percent() * min_dimension
    }
}
