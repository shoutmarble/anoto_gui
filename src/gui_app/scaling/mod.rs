use bevy::{math::Rect, prelude::*, window::PrimaryWindow};

use crate::gui_app::state::{GuiImageState, LayoutConfig, LayoutMetrics};

pub struct ScalingPlugin;

impl Plugin for ScalingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (store_window_metrics, apply_scale_constraints).chain(),
        );
    }
}

#[derive(Component, Clone, Copy)]
pub struct ScaleToFit {
    pub region: ScaleRegion,
    pub padding: Vec2,
    pub aspect_override: Option<f32>,
}

impl ScaleToFit {
    pub fn new(region: ScaleRegion) -> Self {
        Self {
            region,
            padding: Vec2::splat(16.0),
            aspect_override: None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ScaleRegion {
    LeftPanelImage,
    PreviewPanel,
}

fn store_window_metrics(
    window_query: Query<&Window, With<PrimaryWindow>>,
    config: Res<LayoutConfig>,
    mut metrics: ResMut<LayoutMetrics>,
) {
    let Some(window) = window_query.iter().next() else {
        return;
    };
    let size = window.size();
    if metrics.window_size == size {
        return;
    }

    metrics.window_size = size;
    metrics.left_panel =
        Rect::from_corners(Vec2::ZERO, Vec2::new(size.x * config.left_fraction, size.y));
}

fn apply_scale_constraints(
    config: Res<LayoutConfig>,
    gui_image: Res<GuiImageState>,
    mut metrics: ResMut<LayoutMetrics>,
    mut targets: Query<(&mut Node, &mut ScaleToFit)>,
) {
    let mut latest_image_rect = metrics.image_rect;

    for (mut node, mut scale) in &mut targets {
        if matches!(scale.region, ScaleRegion::LeftPanelImage)
            && let Some(aspect) = gui_image.aspect_ratio
        {
            scale.aspect_override = Some(aspect);
        }

        let target_rect = match scale.region {
            ScaleRegion::LeftPanelImage => metrics.left_panel,
            ScaleRegion::PreviewPanel => preview_rect(&metrics, &config),
        };

        let available = Vec2::new(
            (target_rect.width() - scale.padding.x * 2.0).max(0.0),
            (target_rect.height() - scale.padding.y * 2.0).max(0.0),
        );

        let mut width = available.x;
        let mut height = available.y;

        if let Some(aspect) = scale.aspect_override {
            if width / height > aspect {
                width = height * aspect;
            } else {
                height = width / aspect;
            }
        }

        node.width = Val::Px(width.max(0.0));
        node.height = Val::Px(height.max(0.0));

        if scale.region == ScaleRegion::LeftPanelImage {
            let center = Vec2::new(
                target_rect.min.x + target_rect.width() * 0.5,
                target_rect.min.y + target_rect.height() * 0.5,
            );
            latest_image_rect = Some(Rect::from_center_size(center, Vec2::new(width, height)));
            node.align_self = AlignSelf::Center;
            node.justify_self = JustifySelf::Center;
        }
    }

    metrics.image_rect = latest_image_rect;
}

fn preview_rect(metrics: &LayoutMetrics, config: &LayoutConfig) -> Rect {
    let left_edge = metrics.left_panel.max.x;
    let min = Vec2::new(left_edge + config.padding, config.padding);
    let max = Vec2::new(
        metrics.window_size.x - config.padding,
        metrics.window_size.y * 0.45,
    );
    Rect::from_corners(min, max)
}
