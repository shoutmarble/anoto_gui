use iced::mouse::Cursor;
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Program, Stroke, event};
use iced::widget::scrollable::{
    self as scrollables, AbsoluteOffset, Direction, RelativeOffset, Scrollbar,
};
use iced::widget::{button, column, container, pane_grid, scrollable, text};
use iced::{
    Color, Element, Length, Point, Rectangle, Size, Subscription, Task, Theme, Vector, mouse,
    window,
};
use std::path::PathBuf;

pub fn run_iced_app() -> iced::Result {
    iced::application("Anoto Dot Reader", AnotoApp::update, AnotoApp::view)
        .subscription(AnotoApp::subscription)
        .theme(AnotoApp::theme)
        .antialiasing(true)
        .window(window::Settings {
            size: Size::new(1280.0, 720.0),
            ..Default::default()
        })
        .run_with(AnotoApp::new)
}

struct AnotoApp {
    viewer: ImageViewer,
    status_text: String,
    last_loaded: Option<PathBuf>,
    is_loading: bool,
    panes: pane_grid::State<Pane>,
}

#[derive(Debug, Clone)]
enum Message {
    LoadImagePressed,
    FilePicked(Option<PathBuf>),
    ImageLoaded(Result<LoadedImage, String>),
    Viewer(ViewerEvent),
    PaneResized(pane_grid::ResizeEvent),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pane {
    Viewer,
    Controls,
}

#[derive(Debug, Clone)]
enum ViewerEvent {
    Dragged {
        displacement: Vector,
        bounds: Size,
        start_offset: AbsoluteOffset,
        cursor: Option<Point>,
        offset: AbsoluteOffset,
    },
    Zoom {
        factor: f32,
        cursor: Option<Point>,
        bounds: Size,
    },
    Reset,
    ViewportChanged {
        size: Size,
        offset: AbsoluteOffset,
    },
    Hover {
        cursor: Option<Point>,
        bounds: Size,
        offset: AbsoluteOffset,
    },
}

#[derive(Debug, Clone)]
struct LoadedImage {
    handle: iced::widget::image::Handle,
    size: Size,
    pixels: Vec<u8>,
    path: PathBuf,
}

impl AnotoApp {
    fn new() -> (Self, Task<Message>) {
        let default_path = PathBuf::from("assets/GUI_G__81__56__10__10__X.png");
        let initial_task = if default_path.exists() {
            Task::perform(load_image_task(default_path), Message::ImageLoaded)
        } else {
            Task::none()
        };

        let (mut panes, viewer_pane) = pane_grid::State::new(Pane::Viewer);
        panes
            .split(pane_grid::Axis::Vertical, viewer_pane, Pane::Controls)
            .expect("failed to create controls pane");

        (
            AnotoApp {
                viewer: ImageViewer::default(),
                status_text: "Load an image to begin".to_string(),
                last_loaded: None,
                is_loading: false,
                panes,
            },
            initial_task,
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::LoadImagePressed => {
                if self.is_loading {
                    return Task::none();
                }

                let dialog = rfd::AsyncFileDialog::new()
                    .add_filter("Images", &["png", "jpg", "jpeg", "bmp", "tiff"])
                    .pick_file();

                Task::perform(dialog, |result| {
                    Message::FilePicked(result.map(|file| file.path().to_path_buf()))
                })
            }
            Message::FilePicked(Some(path)) => {
                self.is_loading = true;
                Task::perform(load_image_task(path), Message::ImageLoaded)
            }
            Message::FilePicked(None) => Task::none(),
            Message::ImageLoaded(Ok(image)) => {
                let LoadedImage {
                    handle,
                    size,
                    pixels,
                    path,
                } = image;

                self.viewer.set_image(handle, size, pixels);
                self.status_text = format!("Loaded {}", path.display());
                self.last_loaded = Some(path);
                self.is_loading = false;
                Task::none()
            }
            Message::ImageLoaded(Err(error)) => {
                self.status_text = format!("Failed to load image: {error}");
                self.is_loading = false;
                Task::none()
            }
            Message::Viewer(event) => self.viewer.handle_event(event),
            Message::PaneResized(event) => {
                self.panes.resize(event.split, event.ratio);
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        pane_grid::PaneGrid::new(&self.panes, |_, pane, _| match pane {
            Pane::Viewer => pane_grid::Content::new(self.viewer_section()),
            Pane::Controls => pane_grid::Content::new(self.controls_section()),
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .on_resize(10, Message::PaneResized)
        .into()
    }

    fn viewer_section(&self) -> Element<'_, Message> {
        let content_size = self.viewer.content_size();
        let viewer_canvas = Canvas::new(&self.viewer)
            .width(Length::Fixed(content_size.width.max(1.0)))
            .height(Length::Fixed(content_size.height.max(1.0)));

        let viewer_scrollable = scrollable(viewer_canvas)
            .id(self.viewer.scroll_id())
            .direction(Direction::Both {
                vertical: Scrollbar::default(),
                horizontal: Scrollbar::default(),
            })
            .on_scroll(|viewport| {
                let bounds = viewport.bounds();
                let offset = viewport.absolute_offset();
                Message::Viewer(ViewerEvent::ViewportChanged {
                    size: bounds.size(),
                    offset,
                })
            })
            .width(Length::Fill)
            .height(Length::Fill);

        container(viewer_scrollable)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(1)
            .clip(true)
            .style(|_| container::Style {
                background: Some(Color::from_rgb8(24, 24, 24).into()),
                border: iced::border::Border {
                    color: Color::from_rgb8(60, 60, 60),
                    width: 1.0,
                    ..Default::default()
                },
                ..Default::default()
            })
            .into()
    }

    fn controls_section(&self) -> Element<'_, Message> {
        let open_button: Element<'_, Message> = if self.is_loading {
            button("Loading...").width(Length::Fill).into()
        } else {
            button("Open Image")
                .on_press(Message::LoadImagePressed)
                .width(Length::Fill)
                .into()
        };

        let fit_button: Element<'_, Message> = button("Fit to Screen")
            .on_press(Message::Viewer(ViewerEvent::Reset))
            .width(Length::Fill)
            .into();

        let zoom_label: Element<'_, Message> = text(self.viewer.zoom_label()).size(16).into();

        let status_label: Element<'_, Message> = text(&self.status_text).size(14).into();

        let last_loaded: Option<Element<'_, Message>> = self.last_loaded.as_ref().map(|path| {
            text(format!("Last file: {}", path.display()))
                .size(14)
                .into()
        });

        let instructions: Element<'_, Message> =
            text("Mouse wheel to zoom. Drag with left click to pan. Right click to reset.")
                .size(14)
                .into();

        let mut controls = column![open_button, fit_button, zoom_label, status_label]
            .spacing(16)
            .width(Length::Fill);

        if let Some(label) = last_loaded {
            controls = controls.push(label);
        }

        let preview_title: Element<'_, Message> = text("Preview").size(14).into();

        let preview_content: Element<'_, Message> =
            if let Some(handle) = self.viewer.preview_handle() {
                container(
                    iced::widget::image(handle.clone())
                        .width(Length::Fixed(self.viewer.preview_display_size()))
                        .height(Length::Fixed(self.viewer.preview_display_size())),
                )
                .width(Length::Fill)
                .padding(8)
                .style(|_| container::Style {
                    background: Some(Color::from_rgb8(20, 20, 20).into()),
                    border: iced::border::Border {
                        color: Color::from_rgb8(70, 70, 70),
                        width: 1.0,
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .into()
            } else {
                container(text("Hover over the image to see a preview").size(12))
                    .width(Length::Fill)
                    .padding(8)
                    .style(|_| container::Style {
                        background: Some(Color::from_rgb8(20, 20, 20).into()),
                        border: iced::border::Border {
                            color: Color::from_rgb8(70, 70, 70),
                            width: 1.0,
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .into()
            };

        let controls = controls
            .push(instructions)
            .push(preview_title)
            .push(preview_content);

        container(controls)
            .width(Length::Fixed(260.0))
            .padding(20)
            .style(|_| container::Style {
                background: Some(Color::from_rgb8(32, 32, 32).into()),
                ..Default::default()
            })
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::none()
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }
}

struct ImageViewer {
    image: Option<iced::widget::image::Handle>,
    image_size: Size,
    image_dimensions: (u32, u32),
    zoom_mode: ZoomMode,
    custom_scale: f32,
    max_zoom_factor: f32,
    viewport_size: Option<Size>,
    scroll_id: scrollables::Id,
    scroll_offset: AbsoluteOffset,
    pixels: Option<Vec<u8>>,
    hover_viewport_pos: Option<Point>,
    hover_image_pos: Option<Point>,
    hover_overlay_center: Option<Point>,
    preview_handle: Option<iced::widget::image::Handle>,
    pending_scroll: Option<AbsoluteOffset>,
}

#[derive(Debug, Clone, Copy)]
enum ZoomMode {
    Fit,
    Custom,
}

impl Default for ImageViewer {
    fn default() -> Self {
        Self {
            image: None,
            image_size: Size::new(0.0, 0.0),
            image_dimensions: (0, 0),
            zoom_mode: ZoomMode::Fit,
            custom_scale: 1.0,
            max_zoom_factor: 8.0,
            viewport_size: None,
            scroll_id: scrollables::Id::unique(),
            scroll_offset: AbsoluteOffset { x: 0.0, y: 0.0 },
            pixels: None,
            hover_viewport_pos: None,
            hover_image_pos: None,
            hover_overlay_center: None,
            preview_handle: None,
            pending_scroll: None,
        }
    }
}

impl ImageViewer {
    const OVERLAY_SCREEN_SIZE: f32 = 120.0;
    const PREVIEW_DIMENSION: u32 = 160;
    const PREVIEW_DISPLAY_SIZE: f32 = 200.0;

    fn set_image(&mut self, handle: iced::widget::image::Handle, size: Size, pixels: Vec<u8>) {
        self.image = Some(handle);
        self.image_size = size;
        self.image_dimensions = (size.width.round() as u32, size.height.round() as u32);
        self.zoom_mode = ZoomMode::Fit;
        self.custom_scale = 1.0;
        self.viewport_size = None;
        self.scroll_id = scrollables::Id::unique();
        self.scroll_offset = AbsoluteOffset { x: 0.0, y: 0.0 };
        self.pixels = Some(pixels);
        self.hover_viewport_pos = None;
        self.hover_image_pos = None;
        self.hover_overlay_center = None;
        self.preview_handle = None;
        self.pending_scroll = None;
    }

    fn handle_event(&mut self, event: ViewerEvent) -> Task<Message> {
        match event {
            ViewerEvent::Dragged {
                displacement,
                bounds,
                start_offset,
                cursor,
                offset,
            } => {
                self.pending_scroll = None;
                let viewport = self.viewport_size.unwrap_or(bounds);
                let mut task = Task::none();
                self.scroll_offset = offset;
                let effective_offset = if self.image.is_some() {
                    let scale = self.current_scale(viewport);
                    let content_width = self.image_size.width * scale;
                    let content_height = self.image_size.height * scale;

                    if content_width > viewport.width + f32::EPSILON
                        || content_height > viewport.height + f32::EPSILON
                    {
                        let max_offset_x = (content_width - viewport.width).max(0.0);
                        let max_offset_y = (content_height - viewport.height).max(0.0);

                        let target_offset = AbsoluteOffset {
                            x: (start_offset.x - displacement.x).clamp(0.0, max_offset_x),
                            y: (start_offset.y - displacement.y).clamp(0.0, max_offset_y),
                        };

                        let delta = AbsoluteOffset {
                            x: target_offset.x - self.scroll_offset.x,
                            y: target_offset.y - self.scroll_offset.y,
                        };

                        if delta.x.abs() > f32::EPSILON || delta.y.abs() > f32::EPSILON {
                            self.scroll_offset = target_offset;
                            task = scrollables::scroll_by::<Message>(self.scroll_id.clone(), delta);
                        }

                        self.scroll_offset
                    } else {
                        self.scroll_offset
                    }
                } else {
                    self.scroll_offset
                };

                self.set_hover(cursor, viewport, effective_offset);
                task
            }
            ViewerEvent::Zoom {
                factor,
                cursor,
                bounds,
            } => {
                let viewport = self.viewport_size.unwrap_or(bounds);
                if let Some(cursor) = cursor {
                    self.hover_viewport_pos = Some(cursor);
                }
                let task = self.apply_zoom(factor, cursor, viewport);
                self.refresh_hover(viewport, self.scroll_offset);
                task
            }
            ViewerEvent::Reset => {
                self.clear_hover();
                self.reset_view()
            }
            ViewerEvent::ViewportChanged { size, offset } => {
                self.viewport_size = Some(size);
                let mut task = Task::none();

                if self.image.is_some() {
                    let scale = self.current_scale(size);
                    let content_width = self.image_size.width * scale;
                    let content_height = self.image_size.height * scale;
                    let max_offset_x = (content_width - size.width).max(0.0);
                    let max_offset_y = (content_height - size.height).max(0.0);

                    self.scroll_offset = AbsoluteOffset {
                        x: offset.x.clamp(0.0, max_offset_x),
                        y: offset.y.clamp(0.0, max_offset_y),
                    };

                    if let Some(target) = self.pending_scroll {
                        let dx = (self.scroll_offset.x - target.x).abs();
                        let dy = (self.scroll_offset.y - target.y).abs();

                        if dx < 1.0 && dy < 1.0 {
                            self.pending_scroll = None;
                        } else {
                            let clamped_target_x = target.x.clamp(0.0, max_offset_x);
                            let clamped_target_y = target.y.clamp(0.0, max_offset_y);

                            let delta = AbsoluteOffset {
                                x: clamped_target_x - self.scroll_offset.x,
                                y: clamped_target_y - self.scroll_offset.y,
                            };

                            if delta.x.abs() > 1.0 || delta.y.abs() > 1.0 {
                                task = scrollables::scroll_by::<Message>(self.scroll_id.clone(), delta);
                            } else {
                                self.pending_scroll = None;
                            }
                        }
                    }
                } else {
                    self.scroll_offset = AbsoluteOffset { x: 0.0, y: 0.0 };
                    self.pending_scroll = None;
                }

                self.refresh_hover(size, self.scroll_offset);
                task
            }
            ViewerEvent::Hover {
                cursor,
                bounds,
                offset,
            } => {
                self.scroll_offset = offset;
                self.set_hover(cursor, bounds, offset);
                Task::none()
            }
        }
    }

    fn reset_view(&mut self) -> Task<Message> {
        self.clear_hover();
        self.zoom_mode = ZoomMode::Fit;
        self.custom_scale = 1.0;
        self.scroll_offset = AbsoluteOffset { x: 0.0, y: 0.0 };
        self.pending_scroll = None;
        scrollables::snap_to::<Message>(self.scroll_id.clone(), RelativeOffset { x: 0.0, y: 0.0 })
    }

    fn apply_zoom(&mut self, factor: f32, cursor: Option<Point>, viewport: Size) -> Task<Message> {
        if self.image.is_none() {
            return Task::none();
        }

        let fit_scale = self.compute_fit_scale(viewport);
        if fit_scale <= 0.0 {
            return Task::none();
        }

        let previous_scale = match self.zoom_mode {
            ZoomMode::Fit => fit_scale,
            ZoomMode::Custom => self.custom_scale,
        };

        let target_scale =
            (previous_scale * factor).clamp(fit_scale, fit_scale * self.max_zoom_factor);

        if (target_scale - fit_scale).abs() < 0.001 {
            self.zoom_mode = ZoomMode::Fit;
            self.custom_scale = 1.0;
            self.scroll_offset = AbsoluteOffset { x: 0.0, y: 0.0 };
            return scrollables::snap_to::<Message>(
                self.scroll_id.clone(),
                RelativeOffset { x: 0.0, y: 0.0 },
            );
        }

        self.zoom_mode = ZoomMode::Custom;
        self.custom_scale = target_scale;

        let content_size = Size::new(
            (self.image_size.width * target_scale).max(1.0),
            (self.image_size.height * target_scale).max(1.0),
        );

        let max_offset_x = (content_size.width - viewport.width).max(0.0);
        let max_offset_y = (content_size.height - viewport.height).max(0.0);

        if let Some(focus) = cursor {
            let previous_translation = self.compute_translation(viewport, previous_scale);
            let mut content_point = Point::new(
                self.scroll_offset.x + focus.x - previous_translation.x,
                self.scroll_offset.y + focus.y - previous_translation.y,
            );
            content_point.x = content_point.x.max(0.0);
            content_point.y = content_point.y.max(0.0);

            let mut image_point = if previous_scale > 0.0 {
                Point::new(
                    content_point.x / previous_scale,
                    content_point.y / previous_scale,
                )
            } else {
                Point::new(0.0, 0.0)
            };

            image_point.x = image_point.x.clamp(0.0, self.image_size.width.max(0.0));
            image_point.y = image_point.y.clamp(0.0, self.image_size.height.max(0.0));

            let new_translation = self.compute_translation(viewport, target_scale);
            let desired_content = Point::new(
                image_point.x * target_scale + new_translation.x,
                image_point.y * target_scale + new_translation.y,
            );

            let target_offset = AbsoluteOffset {
                x: (desired_content.x - focus.x).clamp(0.0, max_offset_x),
                y: (desired_content.y - focus.y).clamp(0.0, max_offset_y),
            };

            let delta = AbsoluteOffset {
                x: target_offset.x - self.scroll_offset.x,
                y: target_offset.y - self.scroll_offset.y,
            };

            if delta.x.abs() > f32::EPSILON || delta.y.abs() > f32::EPSILON {
                self.scroll_offset = target_offset;
                self.pending_scroll = Some(target_offset);
                scrollables::scroll_by::<Message>(self.scroll_id.clone(), delta)
            } else {
                self.pending_scroll = None;
                Task::none()
            }
        } else {
            let centered_offset = AbsoluteOffset {
                x: (max_offset_x / 2.0).max(0.0),
                y: (max_offset_y / 2.0).max(0.0),
            };

            self.scroll_offset = centered_offset;

            scrollables::snap_to::<Message>(
                self.scroll_id.clone(),
                RelativeOffset { x: 0.5, y: 0.5 },
            )
        }
    }

    fn compute_fit_scale(&self, viewport: Size) -> f32 {
        if self.image_size.width <= 0.0 || self.image_size.height <= 0.0 {
            return 1.0;
        }

        let scale_x = viewport.width / self.image_size.width;
        let scale_y = viewport.height / self.image_size.height;
        scale_x.min(scale_y).max(0.0001)
    }

    fn current_scale(&self, viewport: Size) -> f32 {
        match self.zoom_mode {
            ZoomMode::Fit => self.compute_fit_scale(viewport),
            ZoomMode::Custom => self.custom_scale,
        }
    }

    fn compute_translation(&self, viewport: Size, scale: f32) -> Vector {
        let scaled_width = self.image_size.width * scale;
        let scaled_height = self.image_size.height * scale;

        let translate_x = if scaled_width < viewport.width {
            (viewport.width - scaled_width) / 2.0
        } else {
            0.0
        };

        let translate_y = if scaled_height < viewport.height {
            (viewport.height - scaled_height) / 2.0
        } else {
            0.0
        };

        Vector::new(translate_x, translate_y)
    }

    fn zoom_label(&self) -> String {
        match self.viewport_size {
            Some(viewport) => {
                let percent = self.current_scale(viewport) * 100.0;
                if matches!(self.zoom_mode, ZoomMode::Fit) {
                    format!("Zoom: {:.0}% (Fit)", percent)
                } else {
                    format!("Zoom: {:.0}%", percent)
                }
            }
            None => "Zoom: --".to_string(),
        }
    }

    fn content_size(&self) -> Size {
        if self.image.is_none() {
            return Size::new(1.0, 1.0);
        }

        let scale = if let Some(viewport) = self.viewport_size {
            self.current_scale(viewport)
        } else {
            match self.zoom_mode {
                ZoomMode::Fit => 1.0,
                ZoomMode::Custom => self.custom_scale,
            }
        };

        Size::new(
            (self.image_size.width * scale).max(1.0),
            (self.image_size.height * scale).max(1.0),
        )
    }

    fn scroll_id(&self) -> scrollables::Id {
        self.scroll_id.clone()
    }

    fn preview_handle(&self) -> Option<&iced::widget::image::Handle> {
        self.preview_handle.as_ref()
    }

    fn preview_display_size(&self) -> f32 {
        Self::PREVIEW_DISPLAY_SIZE
    }

    fn clear_hover(&mut self) {
        self.hover_viewport_pos = None;
        self.hover_image_pos = None;
        self.hover_overlay_center = None;
        self.preview_handle = None;
    }

    fn set_hover(&mut self, cursor: Option<Point>, viewport: Size, offset: AbsoluteOffset) {
        if self.image.is_none() {
            self.clear_hover();
            return;
        }

        self.hover_viewport_pos = cursor;

        if cursor.is_some() {
            self.project_hover(viewport, offset);
        } else {
            self.hover_image_pos = None;
            self.hover_overlay_center = None;
            self.preview_handle = None;
        }
    }

    fn refresh_hover(&mut self, viewport: Size, offset: AbsoluteOffset) {
        if self.image.is_none() {
            self.clear_hover();
            return;
        }

        if self.hover_viewport_pos.is_some() {
            self.project_hover(viewport, offset);
        }
    }

    fn project_hover(&mut self, viewport: Size, offset: AbsoluteOffset) {
        if self.image.is_none() {
            self.clear_hover();
            return;
        }

        let Some(cursor) = self.hover_viewport_pos else {
            self.hover_image_pos = None;
            self.hover_overlay_center = None;
            self.preview_handle = None;
            return;
        };

        let scale = self.current_scale(viewport);
        if scale <= 0.0 {
            self.hover_image_pos = None;
            self.hover_overlay_center = None;
            self.preview_handle = None;
            return;
        }

        let translation = self.compute_translation(viewport, scale);
        let scaled_width = self.image_size.width * scale;
        let scaled_height = self.image_size.height * scale;

        // cursor is already in content coordinates
        let content_pos = cursor;
        let relative = Point::new(content_pos.x - translation.x, content_pos.y - translation.y);

        if relative.x < -0.5
            || relative.y < -0.5
            || relative.x > scaled_width + 0.5
            || relative.y > scaled_height + 0.5
        {
            self.hover_image_pos = None;
            self.hover_overlay_center = None;
            self.preview_handle = None;
            return;
        }

        let mut clamped_relative = relative;
        clamped_relative.x = clamped_relative.x.clamp(0.0, scaled_width);
        clamped_relative.y = clamped_relative.y.clamp(0.0, scaled_height);

        let mut image_point = Point::new(clamped_relative.x / scale, clamped_relative.y / scale);
        image_point.x = image_point.x.clamp(0.0, self.image_size.width.max(0.0));
        image_point.y = image_point.y.clamp(0.0, self.image_size.height.max(0.0));

        self.hover_image_pos = Some(image_point);
        
        // Calculate center in content coordinates
        let center = Point::new(
            clamped_relative.x + translation.x,
            clamped_relative.y + translation.y,
        );
        
        // Clamp to viewport in content coordinates
        let half = Self::OVERLAY_SCREEN_SIZE / 2.0;
        
        let min_x = offset.x + half;
        let max_x = (offset.x + viewport.width - half).max(min_x);
        
        let min_y = offset.y + half;
        let max_y = (offset.y + viewport.height - half).max(min_y);
        
        let clamped_center = Point::new(
            center.x.clamp(min_x, max_x),
            center.y.clamp(min_y, max_y)
        );
        
        self.hover_overlay_center = Some(clamped_center);
        self.update_preview(image_point, scale);
    }

    fn update_preview(&mut self, image_point: Point, scale: f32) {
        let Some(pixels) = self.pixels.as_ref() else {
            self.preview_handle = None;
            return;
        };

        let (width_px, height_px) = self.image_dimensions;
        if width_px == 0 || height_px == 0 {
            self.preview_handle = None;
            return;
        }

        let mut region_size = (Self::OVERLAY_SCREEN_SIZE / scale).round().max(1.0) as u32;
        let max_region = width_px.min(height_px).max(1);
        if region_size > max_region {
            region_size = max_region;
        }

        if region_size == 0 {
            self.preview_handle = None;
            return;
        }

        let mut center_x = image_point.x.round() as i32;
        let mut center_y = image_point.y.round() as i32;

        center_x = center_x.clamp(0, width_px.saturating_sub(1) as i32);
        center_y = center_y.clamp(0, height_px.saturating_sub(1) as i32);

        if width_px < region_size {
            region_size = width_px;
        }
        if height_px < region_size {
            region_size = height_px;
        }

        if region_size == 0 {
            self.preview_handle = None;
            return;
        }

        let half = (region_size as i32) / 2;

        let mut start_x = center_x - half;
        let mut start_y = center_y - half;

        if start_x < 0 {
            start_x = 0;
        }
        if start_y < 0 {
            start_y = 0;
        }

        if (start_x as u32 + region_size) > width_px {
            start_x = (width_px - region_size) as i32;
        }
        if (start_y as u32 + region_size) > height_px {
            start_y = (height_px - region_size) as i32;
        }

        start_x = start_x.max(0);
        start_y = start_y.max(0);

        let region_pixels_len = (region_size as usize) * (region_size as usize) * 4;
        let mut region = vec![0u8; region_pixels_len];
        let bytes_per_pixel = 4usize;

        for row in 0..region_size {
            let src_y = start_y as u32 + row;
            let src_index = ((src_y * width_px) + start_x as u32) as usize * bytes_per_pixel;
            let dst_index = (row as usize) * (region_size as usize) * bytes_per_pixel;
            let row_len = (region_size as usize) * bytes_per_pixel;

            if src_index + row_len <= pixels.len() {
                region[dst_index..dst_index + row_len]
                    .copy_from_slice(&pixels[src_index..src_index + row_len]);
            }
        }

        if let Some(buffer) = image::ImageBuffer::<image::Rgba<u8>, Vec<u8>>::from_raw(
            region_size,
            region_size,
            region,
        ) {
            let resized = image::imageops::resize(
                &buffer,
                Self::PREVIEW_DIMENSION,
                Self::PREVIEW_DIMENSION,
                image::imageops::FilterType::Nearest,
            );

            self.preview_handle = Some(iced::widget::image::Handle::from_rgba(
                resized.width(),
                resized.height(),
                resized.into_raw(),
            ));
        } else {
            self.preview_handle = None;
        }
    }
}

struct InteractionState {
    dragging: bool,
    drag_origin: Option<Point>,
    drag_start_offset: AbsoluteOffset,
}

impl Default for InteractionState {
    fn default() -> Self {
        Self {
            dragging: false,
            drag_origin: None,
            drag_start_offset: AbsoluteOffset { x: 0.0, y: 0.0 },
        }
    }
}

impl Program<Message> for ImageViewer {
    type State = InteractionState;

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let clip_region = Rectangle::new(Point::ORIGIN, frame.size());

        frame.with_clip(clip_region, |frame| {
            frame.fill_rectangle(Point::ORIGIN, frame.size(), Color::from_rgb8(18, 18, 18));

            if let Some(handle) = &self.image {
                let scale = self.current_scale(bounds.size());
                let translation = self.compute_translation(bounds.size(), scale);

                frame.with_save(|frame| {
                    frame.translate(translation);
                    frame.scale(scale);
                    frame.draw_image(
                        Rectangle::new(Point::ORIGIN, self.image_size),
                        canvas::Image::new(handle.clone()),
                    );
                });
            } else {
                frame.fill_text(canvas::Text {
                    content: "No image loaded".to_string(),
                    position: Point::new(bounds.width / 2.0 - 70.0, bounds.height / 2.0),
                    color: Color::from_rgb8(200, 200, 200),
                    ..Default::default()
                });
            }

            let overlay_center = match (self.hover_overlay_center, self.hover_viewport_pos) {
                (Some(center), _) => Some(center),
                (None, other) => other,
            };

            if let Some(center) = overlay_center {
                let half = Self::OVERLAY_SCREEN_SIZE / 2.0;
                let top_left = Point::new(center.x - half, center.y - half);
                let overlay_size = Size::new(Self::OVERLAY_SCREEN_SIZE, Self::OVERLAY_SCREEN_SIZE);

                let overlay_fill = Color::from_rgba(0.2, 0.6, 0.9, 0.15);
                let overlay_border = Stroke::default()
                    .with_width(2.0)
                    .with_color(Color::from_rgba(0.2, 0.6, 0.9, 0.7));

                frame.fill_rectangle(top_left, overlay_size, overlay_fill);
                frame.stroke_rectangle(top_left, overlay_size, overlay_border);
            }

            let border = Stroke::default()
                .with_width(1.0)
                .with_color(Color::from_rgb8(70, 70, 70));
            frame.stroke_rectangle(Point::ORIGIN, frame.size(), border);
        });

        vec![frame.into_geometry()]
    }

    fn update(
        &self,
        state: &mut Self::State,
        event: canvas::Event,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> (event::Status, Option<Message>) {
        match event {
            canvas::Event::Mouse(mouse_event) => match mouse_event {
                mouse::Event::ButtonPressed(mouse::Button::Left) => {
                    if cursor.position_in(bounds).is_some() {
                        if let Some(global) = cursor.position() {
                            state.dragging = true;
                            state.drag_origin = Some(global);
                            state.drag_start_offset = self.scroll_offset;
                        } else {
                            state.dragging = false;
                            state.drag_origin = None;
                        }
                        (event::Status::Captured, None)
                    } else {
                        (event::Status::Ignored, None)
                    }
                }
                mouse::Event::ButtonReleased(mouse::Button::Left) => {
                    state.dragging = false;
                    state.drag_origin = None;
                    (event::Status::Captured, None)
                }
                mouse::Event::ButtonPressed(mouse::Button::Right) => (
                    event::Status::Captured,
                    Some(Message::Viewer(ViewerEvent::Reset)),
                ),
                mouse::Event::CursorMoved { .. } => {
                    let viewport_cursor = cursor.position_in(bounds);

                    if state.dragging {
                        if let Some(origin) = state.drag_origin {
                            if let Some(current) = cursor.position() {
                                let displacement =
                                    Vector::new(current.x - origin.x, current.y - origin.y);
                                if displacement.x.abs() > f32::EPSILON
                                    || displacement.y.abs() > f32::EPSILON
                                {
                                    return (
                                        event::Status::Captured,
                                        Some(Message::Viewer(ViewerEvent::Dragged {
                                            displacement,
                                            bounds: bounds.size(),
                                            start_offset: state.drag_start_offset,
                                            cursor: viewport_cursor,
                                            offset: self.scroll_offset,
                                        })),
                                    );
                                }
                            }
                        }

                        (
                            event::Status::Captured,
                            Some(Message::Viewer(ViewerEvent::Hover {
                                cursor: viewport_cursor,
                                bounds: bounds.size(),
                                offset: self.scroll_offset,
                            })),
                        )
                    } else {
                        (
                            event::Status::Captured,
                            Some(Message::Viewer(ViewerEvent::Hover {
                                cursor: viewport_cursor,
                                bounds: bounds.size(),
                                offset: self.scroll_offset,
                            })),
                        )
                    }
                }
                mouse::Event::WheelScrolled { delta } => {
                    let steps = match delta {
                        mouse::ScrollDelta::Lines { y, .. } => y,
                        mouse::ScrollDelta::Pixels { y, .. } => y / 120.0,
                    };

                    if steps.abs() > f32::EPSILON {
                        let factor = 1.1_f32.powf(steps);
                        let cursor_position = cursor.position_in(bounds);
                        (
                            event::Status::Captured,
                            Some(Message::Viewer(ViewerEvent::Zoom {
                                factor,
                                cursor: cursor_position,
                                bounds: bounds.size(),
                            })),
                        )
                    } else {
                        (event::Status::Ignored, None)
                    }
                }
                mouse::Event::CursorLeft => (
                    event::Status::Captured,
                    Some(Message::Viewer(ViewerEvent::Hover {
                        cursor: None,
                        bounds: bounds.size(),
                        offset: self.scroll_offset,
                    })),
                ),
                _ => (event::Status::Ignored, None),
            },
            _ => (event::Status::Ignored, None),
        }
    }
}

async fn load_image_task(path: PathBuf) -> Result<LoadedImage, String> {
    let original_path = path.clone();
    tokio::task::spawn_blocking(move || {
        let image = image::open(&original_path).map_err(|err| err.to_string())?;
        let rgba = image.to_rgba8();
        let (width, height) = rgba.dimensions();
        let pixels = rgba.into_raw();
        let handle = iced::widget::image::Handle::from_rgba(width, height, pixels.clone());
        Ok(LoadedImage {
            handle,
            size: Size::new(width as f32, height as f32),
            pixels,
            path: original_path,
        })
    })
    .await
    .map_err(|err| err.to_string())?
}
