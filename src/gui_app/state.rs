use bevy::math::Rect;
use bevy::prelude::*;
use image::{DynamicImage, GenericImageView};

#[derive(Resource, Default)]
pub struct GuiImageState {
    pub original: Option<DynamicImage>,
    pub gpu_handle: Option<Handle<Image>>,
    pub aspect_ratio: Option<f32>,
    pub dirty: bool,
}

impl GuiImageState {
    pub fn set_image(&mut self, dynamic: DynamicImage, handle: Handle<Image>) {
        let (width, height) = dynamic.dimensions();
        self.aspect_ratio = Some(width as f32 / height as f32);
        self.gpu_handle = Some(handle);
        self.original = Some(dynamic);
        self.dirty = true;
    }

    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }
}

#[derive(Resource)]
pub struct LayoutConfig {
    pub left_fraction: f32,
    pub right_fraction: f32,
    pub padding: f32,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            left_fraction: 0.65,
            right_fraction: 0.35,
            padding: 24.0,
        }
    }
}

#[derive(Resource, Default)]
pub struct LayoutMetrics {
    pub window_size: Vec2,
    pub left_panel: Rect,
    pub image_rect: Option<Rect>,
}

#[derive(Resource)]
pub struct ZoomSettings {
    pub square_size: f32,
    pub min_square: f32,
    pub max_square: f32,
}

impl Default for ZoomSettings {
    fn default() -> Self {
        Self {
            square_size: 160.0,
            min_square: 60.0,
            max_square: 320.0,
        }
    }
}

#[derive(Resource, Default)]
pub struct ZoomPreviewState {
    pub handle: Option<Handle<Image>>,
    pub aspect_ratio: Option<f32>,
}

#[derive(Resource, Default)]
pub struct CursorState {
    pub window_position: Option<Vec2>,
}

#[derive(Event, Clone, Message)]
pub struct ImageLoadedEvent {
    pub handle: Handle<Image>,
}

#[derive(Event, Clone, Message)]
pub struct ZoomCapturedEvent {
    pub handle: Handle<Image>,
    pub aspect_ratio: f32,
}
