//! File selection dialog system for the Anoto GUI

use bevy::prelude::*;
use crate::bevy::utils::*;

/// System that handles file selection dialog
pub fn dialog_system(
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