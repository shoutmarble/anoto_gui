use bevy::prelude::*;

use bevy::ui::widget::ImageNode;

use crate::gui_app::{
    layout::{MainImage, ZoomPreview},
    scaling::ScaleToFit,
    state::{GuiImageState, ImageLoadedEvent, ZoomCapturedEvent, ZoomPreviewState},
};

pub struct PreviewPlugin;

impl Plugin for PreviewPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                apply_loaded_image,
                apply_zoom_preview,
            ),
        );
    }
}

fn apply_loaded_image(
    mut events: MessageReader<ImageLoadedEvent>,
    mut image_nodes: Query<&mut ImageNode, With<MainImage>>,
    mut gui_state: ResMut<GuiImageState>,
) {
    let Some(event) = events.read().last().cloned() else {
        return;
    };

    if let Ok(mut ui_image) = image_nodes.single_mut() {
        ui_image.image = event.handle.clone();
    }

    gui_state.clear_dirty();
}

fn apply_zoom_preview(
    mut events: MessageReader<ZoomCapturedEvent>,
    mut preview_nodes: Query<(&mut ImageNode, &mut ScaleToFit), With<ZoomPreview>>,
    mut preview_state: ResMut<ZoomPreviewState>,
) {
    let Some(event) = events.read().last().cloned() else {
        return;
    };

    if let Ok((mut ui_image, mut scale)) = preview_nodes.single_mut() {
        ui_image.image = event.handle.clone();
        preview_state.handle = Some(event.handle.clone());
        preview_state.aspect_ratio = Some(event.aspect_ratio);
        scale.aspect_override = Some(event.aspect_ratio);
    }
}
