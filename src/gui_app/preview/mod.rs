use bevy::prelude::*;

use bevy::ui::widget::ImageNode;

use crate::gui_app::{
    layout::{ArrowGridText, DownloadArrowsButton, MainImage, ZoomPreview},
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
                handle_download_button,
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
    mut arrow_text_query: Query<&mut Text, With<ArrowGridText>>,
    mut preview_state: ResMut<ZoomPreviewState>,
) {
    let Some(event) = events.read().last().cloned() else {
        return;
    };

    if let Ok((mut ui_image, mut scale)) = preview_nodes.single_mut() {
        ui_image.image = event.handle.clone();
        preview_state.handle = Some(event.handle.clone());
        preview_state.aspect_ratio = Some(event.aspect_ratio);
        preview_state.arrow_grid = Some(event.arrow_grid.clone());
        scale.aspect_override = Some(event.aspect_ratio);
    }

    if let Ok(mut text) = arrow_text_query.single_mut() {
        // Replace unicode arrows with ASCII for display since the default font might not support them
        text.0 = event
            .arrow_grid
            .replace("←", "<")
            .replace("↑", "^")
            .replace("→", ">")
            .replace("↓", "v");
    }
}

fn handle_download_button(
    interaction_query: Query<&Interaction, (Changed<Interaction>, With<DownloadArrowsButton>)>,
    preview_state: Res<ZoomPreviewState>,
) {
    for interaction in interaction_query.iter() {
        if *interaction == Interaction::Pressed {
            if let Some(grid) = &preview_state.arrow_grid {
                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name("arrows.txt")
                    .save_file()
                {
                    if let Err(e) = std::fs::write(path, grid) {
                        error!("Failed to save arrows: {e}");
                    }
                }
            }
        }
    }
}
