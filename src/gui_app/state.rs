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
    pub padding: f32,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            left_fraction: 0.65,
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
    pub square_percent: f32,
    pub min_percent: f32,
    pub max_percent: f32,
}

impl Default for ZoomSettings {
    fn default() -> Self {
        Self {
            square_percent: 0.10,
            min_percent: 0.0,
            max_percent: 1.0,
        }
    }
}

impl ZoomSettings {
    pub fn normalized_percent(&self) -> f32 {
        self.square_percent
            .clamp(self.min_percent, self.max_percent)
    }

    pub fn reset_to_default(&mut self) {
        self.square_percent = Self::default().square_percent;
    }

    pub fn slider_value(&self) -> f32 {
        self.normalized_percent() * 100.0
    }

    pub fn apply_slider_value(&mut self, slider_value: f32) {
        let normalized = (slider_value / 100.0).clamp(self.min_percent, self.max_percent);
        self.square_percent = normalized;
    }
}

#[derive(Resource, Default)]
pub struct ZoomPreviewState {
    pub handle: Option<Handle<Image>>,
    pub aspect_ratio: Option<f32>,
    pub arrow_grid: Option<String>,
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
    pub arrow_grid: String,
}
