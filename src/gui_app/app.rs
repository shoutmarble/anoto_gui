use bevy::{prelude::*, window::{Window, WindowMode, WindowPlugin, WindowResolution}};

use crate::gui_app::{
    layout::LayoutPlugin,
    loader::LoaderPlugin,
    preview::PreviewPlugin,
    scaling::ScalingPlugin,
    state::{CursorState, GuiImageState, ImageLoadedEvent, LayoutConfig, LayoutMetrics, ZoomCapturedEvent, ZoomPreviewState, ZoomSettings},
    zoom::ZoomPlugin,
};

pub struct GuiAppPlugin;

impl Plugin for GuiAppPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Anoto GUI - Image Lab".to_string(),
                resolution: default_window_resolution(),
                resizable: true,
                mode: WindowMode::Windowed,
                ..default()
            }),
            ..default()
        }))
        .init_resource::<GuiImageState>()
        .insert_resource(LayoutConfig::default())
        .init_resource::<LayoutMetrics>()
        .init_resource::<ZoomSettings>()
        .init_resource::<ZoomPreviewState>()
        .init_resource::<CursorState>()
        .add_message::<ImageLoadedEvent>()
        .add_message::<ZoomCapturedEvent>()
        .add_plugins((
            LayoutPlugin,
            LoaderPlugin,
            ScalingPlugin,
            ZoomPlugin,
            PreviewPlugin,
        ));
    }
}

pub fn run_gui_app() {
    App::new().add_plugins(GuiAppPlugin).run();
}

fn default_window_resolution() -> WindowResolution {
    WindowResolution::new(1280, 720)
}
