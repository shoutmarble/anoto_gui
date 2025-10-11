//! Shared utilities and common components for the Anoto GUI

use bevy::prelude::*;
use std::path::PathBuf;
use image::DynamicImage;

/// Marker for the image selection button
#[derive(Component)]
pub struct ImageSelectButton;

/// Marker for the image display area
#[derive(Component)]
pub struct ImageDisplay;

/// Marker for placeholder text when no image is selected
#[derive(Component)]
pub struct PlaceholderText;

/// Marker for the secondary image display area
#[derive(Component)]
pub struct SecondaryImageDisplay;

/// Marker for placeholder text in secondary area when no image is selected
#[derive(Component)]
pub struct SecondaryPlaceholderText;

/// Marker for displayed images
#[derive(Component)]
pub struct DisplayedImage;

/// Marker for secondary displayed images
#[derive(Component)]
pub struct SecondaryDisplayedImage;

/// Marker for the magnifier window
#[derive(Component)]
pub struct Magnifier;

/// Marker for the magnifier rectangle overlay
#[derive(Component)]
pub struct MagnifierRectangle;

/// Marker for the blue border around magnified area
#[derive(Component)]
pub struct MagnifierBlueBorder;

/// Resource to track the selected image
#[derive(Resource, Default)]
pub struct SelectedImage {
    pub path: Option<PathBuf>,
    pub handle: Option<Handle<Image>>,
    pub dimensions: Option<(u32, u32)>,
    pub is_loading: bool,
}

/// Resource to store the original image buffer
#[derive(Resource, Default)]
pub struct OriginalImage(pub Option<DynamicImage>);

/// Resource to track dialog state
#[derive(Resource, Default)]
pub struct DialogState {
    pub should_open: bool,
    pub frame_delay: u32,
}

/// Constants for UI styling
pub const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
pub const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);