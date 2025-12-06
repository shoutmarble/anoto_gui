use iced::mouse::Cursor;
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Program, Stroke, event};
use iced::widget::{button, column, container, pane_grid, stack, text};
use iced::{
    Color, Element, Length, Point, Rectangle, Size, Subscription, Task, Theme, Vector, mouse,
    window,
};
use std::path::PathBuf;

pub fn run_iced_app() -> iced::Result {
    iced::application("Anoto Dot Reader", AnotoApp::update, AnotoApp::view)
        .subscription(AnotoApp::subscription)
        .theme(AnotoApp::theme)
        .antialiasing(false)
        .window(window::Settings {
            size: Size::new(640.0, 380.0),
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
    RegionSizeChanged(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pane {
    Viewer,
    Controls,
}

#[derive(Debug, Clone)]
enum ViewerEvent {
    Pan {
        offset: Vector,
        bounds: Size,
    },
    Zoom {
        factor: f32,
        cursor: Point,
        bounds: Size,
    },
    Reset,
    ToggleDebug,
    Hover {
        cursor: Point,
        bounds: Size,
    },
    Leave,
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
            Message::RegionSizeChanged(size) => {
                self.viewer.set_region_size(size);
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
        // Use Stack to layer: image canvas on bottom, overlay canvas on top
        // Both canvases share the same viewer state for coordinates
        let image_canvas = Canvas::new(ImageLayer(&self.viewer))
            .width(Length::Fill)
            .height(Length::Fill);

        let overlay_canvas = Canvas::new(OverlayLayer(&self.viewer))
            .width(Length::Fill)
            .height(Length::Fill);

        // Stack them: image first (bottom), then overlay (top)
        let stacked = stack![
            container(image_canvas)
                .width(Length::Fill)
                .height(Length::Fill)
                .clip(true),
            overlay_canvas
        ];

        container(stacked)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(0)
            .clip(true)
            .style(|_| container::Style {
                background: Some(Color::from_rgb8(24, 24, 24).into()),
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

        let debug_button: Element<'_, Message> = button("Toggle Debug")
            .on_press(Message::Viewer(ViewerEvent::ToggleDebug))
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

        let region_size_label: Element<'_, Message> = text(format!(
            "AOI Size: {}px",
            self.viewer.region_size()
        ))
        .size(14)
        .into();

        let region_size_slider: Element<'_, Message> = iced::widget::slider(
            ImageViewer::MIN_REGION_SIZE..=self.viewer.max_region_size().max(ImageViewer::MIN_REGION_SIZE),
            self.viewer.region_size(),
            Message::RegionSizeChanged,
        )
        .width(Length::Fill)
        .into();

        let mut controls = column![
            open_button,
            fit_button,
            debug_button,
            zoom_label,
            status_label,
            region_size_label,
            region_size_slider
        ]
        .spacing(16)
        .width(Length::Fill);

        if let Some(label) = last_loaded {
            controls = controls.push(label);
        }

        // Helper function to create a legend-style frame
        let legend_style = |_: &_| container::Style {
            background: None,
            border: iced::border::Border {
                color: Color::from_rgb8(100, 100, 100),
                width: 1.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        };

        // Wrap controls in a "Controls" legend
        let controls_legend: Element<'_, Message> = column![
            container(
                text(" Controls ").size(12)
            )
            .style(|_| container::Style {
                background: Some(Color::from_rgb8(32, 32, 32).into()),
                ..Default::default()
            }),
            container(controls.push(instructions))
                .padding(10)
                .style(legend_style)
        ]
        .spacing(0)
        .into();

        let preview_content: Element<'_, Message> =
            if let Some(handle) = self.viewer.preview_handle() {
                // Use aspect_ratio container to maintain square shape
                container(
                    iced::widget::image(handle.clone())
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .content_fit(iced::ContentFit::Fill), // Fill the square container
                )
                .width(Length::Fill)
                .style(|_| container::Style {
                    background: Some(Color::WHITE.into()),
                    border: iced::border::Border {
                        color: Color::from_rgb(1.0, 0.0, 1.0), // Magenta to match AOI
                        width: 3.0,
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

        // Wrap preview in a "Preview" legend
        let preview_legend: Element<'_, Message> = column![
            container(
                text(" Preview ").size(12)
            )
            .style(|_| container::Style {
                background: Some(Color::from_rgb8(32, 32, 32).into()),
                ..Default::default()
            }),
            container(preview_content)
                .padding(10)
                .width(Length::Fill)
                .style(legend_style)
        ]
        .spacing(0)
        .into();

        let all_controls = column![
            controls_legend,
            preview_legend
        ]
        .spacing(16)
        .width(Length::Fill);

        container(all_controls)
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

    // View state
    offset: Vector, // Pan offset
    viewport_size: Size,
    show_debug: bool,

    // Caches
    image_cache: canvas::Cache,
    overlay_cache: canvas::Cache,

    // Cropped image cache for clipping
    cropped_handle: Option<iced::widget::image::Handle>,
    cropped_dest: Rectangle,

    pixels: Option<Vec<u8>>,
    hover_viewport_pos: Option<Point>,
    hover_image_pos: Option<Point>,
    hover_overlay_center: Option<Point>,
    preview_handle: Option<iced::widget::image::Handle>,
    region_size: u32, // Source pixels for AOI overlay and preview
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
            offset: Vector::new(0.0, 0.0),
            viewport_size: Size::new(0.0, 0.0),
            show_debug: true,
            image_cache: canvas::Cache::default(),
            overlay_cache: canvas::Cache::default(),
            cropped_handle: None,
            cropped_dest: Rectangle::new(Point::ORIGIN, Size::ZERO),
            pixels: None,
            hover_viewport_pos: None,
            hover_image_pos: None,
            hover_overlay_center: None,
            preview_handle: None,
            region_size: 40,
        }
    }
}

impl ImageViewer {
    const MIN_REGION_SIZE: u32 = 10;

    fn set_image(&mut self, handle: iced::widget::image::Handle, size: Size, pixels: Vec<u8>) {
        self.image = Some(handle);
        self.image_size = size;
        self.image_dimensions = (size.width.round() as u32, size.height.round() as u32);
        self.zoom_mode = ZoomMode::Fit;
        self.custom_scale = 1.0;
        self.offset = Vector::new(0.0, 0.0);
        self.pixels = Some(pixels);
        self.hover_viewport_pos = None;
        self.hover_image_pos = None;
        self.hover_overlay_center = None;
        self.preview_handle = None;
        self.cropped_handle = None;
        self.cropped_dest = Rectangle::new(Point::ORIGIN, Size::ZERO);
        self.invalidate_image_layer();
    }

    fn invalidate_image_layer(&mut self) {
        self.image_cache.clear();
        self.update_cropped_cache();
    }

    fn update_cropped_cache(&mut self) {
        // Clear existing cache
        self.cropped_handle = None;
        self.cropped_dest = Rectangle::new(Point::ORIGIN, Size::ZERO);

        let Some(pixels) = &self.pixels else { return };
        if self.image.is_none() || self.viewport_size.width <= 0.0 {
            return;
        }

        let scale = self.current_scale(self.viewport_size);
        let bounds = self.viewport_size;

        // Calculate full image bounds in screen space
        let img_left = self.offset.x;
        let img_top = self.offset.y;
        let img_width = self.image_size.width * scale;
        let img_height = self.image_size.height * scale;

        // Check if clipping is needed
        let needs_clip = img_left < 0.0
            || img_top < 0.0
            || img_left + img_width > bounds.width
            || img_top + img_height > bounds.height;

        if !needs_clip {
            return; // Will use full image directly
        }

        let (img_w, img_h) = self.image_dimensions;

        // Calculate visible region
        let vis_left = img_left.max(0.0);
        let vis_top = img_top.max(0.0);
        let vis_right = (img_left + img_width).min(bounds.width);
        let vis_bottom = (img_top + img_height).min(bounds.height);

        if vis_right <= vis_left || vis_bottom <= vis_top {
            return;
        }

        // Calculate source pixel coordinates
        let src_left = ((vis_left - img_left) / scale).floor() as u32;
        let src_top = ((vis_top - img_top) / scale).floor() as u32;
        let src_right = (((vis_right - img_left) / scale).ceil() as u32).min(img_w);
        let src_bottom = (((vis_bottom - img_top) / scale).ceil() as u32).min(img_h);

        let crop_w = src_right.saturating_sub(src_left);
        let crop_h = src_bottom.saturating_sub(src_top);

        if crop_w == 0 || crop_h == 0 {
            return;
        }

        // Extract visible pixels
        let mut cropped = Vec::with_capacity((crop_w * crop_h * 4) as usize);
        for y in src_top..src_bottom {
            let start = ((y * img_w + src_left) * 4) as usize;
            let end = ((y * img_w + src_right) * 4) as usize;
            if end <= pixels.len() {
                cropped.extend_from_slice(&pixels[start..end]);
            }
        }

        self.cropped_handle = Some(iced::widget::image::Handle::from_rgba(crop_w, crop_h, cropped));

        // Calculate destination rectangle
        let dest_x = img_left + (src_left as f32 * scale);
        let dest_y = img_top + (src_top as f32 * scale);
        self.cropped_dest = Rectangle::new(
            Point::new(dest_x, dest_y),
            Size::new(crop_w as f32 * scale, crop_h as f32 * scale),
        );
    }

    fn clamp_offset(&self, offset: Vector, viewport: Size, scale: f32) -> Vector {
        let image_width = self.image_size.width * scale;
        let image_height = self.image_size.height * scale;
        let viewport_width = viewport.width;
        let viewport_height = viewport.height;

        let (min_x, max_x) = if image_width <= viewport_width {
            let center = (viewport_width - image_width) / 2.0;
            (center, center)
        } else {
            (viewport_width - image_width, 0.0)
        };

        let (min_y, max_y) = if image_height <= viewport_height {
            let center = (viewport_height - image_height) / 2.0;
            (center, center)
        } else {
            (viewport_height - image_height, 0.0)
        };

        Vector::new(offset.x.clamp(min_x, max_x), offset.y.clamp(min_y, max_y))
    }

    fn handle_event(&mut self, event: ViewerEvent) -> Task<Message> {
        match event {
            ViewerEvent::Pan { offset, bounds } => {
                let _ = self.apply_viewport_resize(bounds);
                if self.image.is_none() {
                    return Task::none();
                }
                let scale = self.current_scale(bounds);
                self.offset = self.clamp_offset(offset, bounds, scale);
                self.invalidate_image_layer();
                Task::none()
            }
            ViewerEvent::Zoom {
                factor,
                cursor,
                bounds,
            } => {
                let _ = self.apply_viewport_resize(bounds);
                if self.image.is_none() {
                    return Task::none();
                }

                let current_scale = self.current_scale(bounds);
                let new_scale = current_scale * factor;

                let fit_scale = self.compute_fit_scale(bounds);
                let max_scale = fit_scale * self.max_zoom_factor;
                let clamped_scale = new_scale.clamp(fit_scale, max_scale);

                if (clamped_scale - fit_scale).abs() < 0.001 {
                    self.zoom_mode = ZoomMode::Fit;
                    self.custom_scale = 1.0;
                    self.offset = self.center_offset(bounds, fit_scale);
                } else {
                    self.zoom_mode = ZoomMode::Custom;
                    self.custom_scale = clamped_scale;

                    let scale_ratio = clamped_scale / current_scale;

                    let raw_offset = Vector::new(
                        cursor.x - (cursor.x - self.offset.x) * scale_ratio,
                        cursor.y - (cursor.y - self.offset.y) * scale_ratio,
                    );

                    self.offset = self.clamp_offset(raw_offset, bounds, clamped_scale);
                }

                self.invalidate_image_layer();
                self.refresh_hover();
                Task::none()
            }
            ViewerEvent::Reset => {
                self.zoom_mode = ZoomMode::Fit;
                self.custom_scale = 1.0;
                if self.viewport_size.width > 0.0 && self.viewport_size.height > 0.0 {
                    let scale = self.compute_fit_scale(self.viewport_size);
                    self.offset = self.center_offset(self.viewport_size, scale);
                } else {
                    self.offset = Vector::new(0.0, 0.0);
                }
                self.invalidate_image_layer();
                self.refresh_hover();
                Task::none()
            }
            ViewerEvent::ToggleDebug => {
                self.show_debug = !self.show_debug;
                Task::none()
            }
            ViewerEvent::Hover { cursor, bounds } => {
                let _ = self.apply_viewport_resize(bounds);
                self.hover_viewport_pos = Some(cursor);
                self.refresh_hover();
                Task::none()
            }
            ViewerEvent::Leave => {
                self.hover_viewport_pos = None;
                self.hover_image_pos = None;
                self.hover_overlay_center = None;
                self.preview_handle = None;
                Task::none()
            }
        }
    }

    fn apply_viewport_resize(&mut self, new_size: Size) -> bool {
        let size_changed = (self.viewport_size.width - new_size.width).abs() > f32::EPSILON
            || (self.viewport_size.height - new_size.height).abs() > f32::EPSILON;

        if !size_changed {
            return false;
        }

        self.viewport_size = new_size;

        if self.image.is_some() {
            match self.zoom_mode {
                ZoomMode::Fit => {
                    let fit_scale = self.compute_fit_scale(new_size);
                    self.offset = self.center_offset(new_size, fit_scale);
                }
                ZoomMode::Custom => {
                    let scale = self.current_scale(new_size);
                    self.offset = self.clamp_offset(self.offset, new_size, scale);
                }
            }

            self.invalidate_image_layer();
        }

        true
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

    fn center_offset(&self, viewport: Size, scale: f32) -> Vector {
        let scaled_width = self.image_size.width * scale;
        let scaled_height = self.image_size.height * scale;

        let x = (viewport.width - scaled_width) / 2.0;
        let y = (viewport.height - scaled_height) / 2.0;

        Vector::new(x, y)
    }

    fn zoom_label(&self) -> String {
        if self.viewport_size.width > 0.0 {
            let percent = self.current_scale(self.viewport_size) * 100.0;
            if matches!(self.zoom_mode, ZoomMode::Fit) {
                format!("Zoom: {:.0}% (Fit)", percent)
            } else {
                format!("Zoom: {:.0}%", percent)
            }
        } else {
            "Zoom: --".to_string()
        }
    }

    fn preview_handle(&self) -> Option<&iced::widget::image::Handle> {
        self.preview_handle.as_ref()
    }

    fn region_size(&self) -> u32 {
        self.region_size
    }

    fn set_region_size(&mut self, size: u32) {
        let max = self.max_region_size();
        self.region_size = size.clamp(Self::MIN_REGION_SIZE, max);
        // Refresh preview with new region size
        if let Some(image_point) = self.hover_image_pos {
            self.update_preview(image_point);
        }
    }

    fn max_region_size(&self) -> u32 {
        let (w, h) = self.image_dimensions;
        if w == 0 || h == 0 {
            Self::MIN_REGION_SIZE
        } else {
            w.min(h)
        }
    }

    fn refresh_hover(&mut self) {
        if self.image.is_none() {
            self.hover_image_pos = None;
            self.hover_overlay_center = None;
            self.preview_handle = None;
            return;
        }

        let Some(cursor) = self.hover_viewport_pos else {
            self.hover_image_pos = None;
            self.hover_overlay_center = None;
            self.preview_handle = None;
            return;
        };

        let viewport = self.viewport_size;
        if viewport.width <= 0.0 || viewport.height <= 0.0 {
            return;
        }

        let scale = self.current_scale(viewport);
        if scale <= 0.0 {
            return;
        }

        let image_x = (cursor.x - self.offset.x) / scale;
        let image_y = (cursor.y - self.offset.y) / scale;

        let image_point = Point::new(image_x, image_y);

        if image_x < 0.0
            || image_y < 0.0
            || image_x > self.image_size.width
            || image_y > self.image_size.height
        {
            self.hover_image_pos = None;
            self.hover_overlay_center = None;
            self.preview_handle = None;
            return;
        }

        self.hover_image_pos = Some(image_point);
        self.hover_overlay_center = Some(cursor);

        self.update_preview(image_point);
    }

    fn draw_overlay(&self, frame: &mut Frame, bounds: Rectangle) {
        let clip_region = Rectangle::new(Point::ORIGIN, bounds.size());
        frame.with_clip(clip_region, |frame| {
            if self.image.is_some() {
                let corner_size = Size::new(20.0, 20.0);
                let right = (bounds.width - corner_size.width).max(0.0);
                let bottom = (bounds.height - corner_size.height).max(0.0);

                frame.fill_rectangle(Point::ORIGIN, corner_size, Color::from_rgb(1.0, 0.0, 0.0));
                frame.fill_rectangle(
                    Point::new(right, 0.0),
                    corner_size,
                    Color::from_rgb(0.0, 1.0, 0.0),
                );
                frame.fill_rectangle(
                    Point::new(0.0, bottom),
                    corner_size,
                    Color::from_rgb(0.0, 0.0, 1.0),
                );
                frame.fill_rectangle(
                    Point::new(right, bottom),
                    corner_size,
                    Color::from_rgb(1.0, 1.0, 0.0),
                );

                let overlay_center = match (self.hover_overlay_center, self.hover_viewport_pos) {
                    (Some(center), _) => Some(center),
                    (None, other) => other,
                };

                if let Some(center) = overlay_center {
                    // Compute scale to size the overlay box correctly
                    let scale = self.current_scale(bounds.size());
                    let overlay_screen_size = (self.region_size as f32) * scale;
                    let half = overlay_screen_size / 2.0;
                    let top_left = Point::new(center.x - half, center.y - half);
                    let overlay_size =
                        Size::new(overlay_screen_size, overlay_screen_size);

                    // Only stroke, no fill
                    frame.stroke_rectangle(
                        top_left,
                        overlay_size,
                        Stroke::default()
                            .with_width(2.67)
                            .with_color(Color::from_rgb(1.0, 0.0, 1.0)),
                    );
                    frame.fill_rectangle(
                        Point::new(center.x - 2.0, center.y - 2.0),
                        Size::new(4.0, 4.0),
                        Color::WHITE,
                    );
                }
            }
        });

        let border = Stroke::default()
            .with_width(1.0)
            .with_color(Color::from_rgb8(70, 70, 70));
        frame.stroke_rectangle(Point::ORIGIN, bounds.size(), border);
    }

    fn build_overlay_layer(&self, renderer: &iced::Renderer, bounds: Rectangle) -> Geometry {
        let mut frame = Frame::new(renderer, bounds.size());
        self.draw_overlay(&mut frame, bounds);
        frame.into_geometry()
    }

    fn update_preview(&mut self, image_point: Point) {
        let Some(pixels) = self.pixels.as_ref() else {
            self.preview_handle = None;
            return;
        };

        let (width_px, height_px) = self.image_dimensions;
        if width_px == 0 || height_px == 0 {
            self.preview_handle = None;
            return;
        }

        // Extract a fixed-size region from the source image
        // This will be scaled UP to the preview display size for pixel-perfect viewing
        let region_size = self.region_size.min(width_px).min(height_px);

        if region_size == 0 {
            self.preview_handle = None;
            return;
        }

        let mut center_x = image_point.x.round() as i32;
        let mut center_y = image_point.y.round() as i32;

        center_x = center_x.clamp(0, width_px.saturating_sub(1) as i32);
        center_y = center_y.clamp(0, height_px.saturating_sub(1) as i32);

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
            // Use raw pixels at 1:1 for pixel-perfect display
            self.preview_handle = Some(iced::widget::image::Handle::from_rgba(
                buffer.width(),
                buffer.height(),
                buffer.into_raw(),
            ));
        } else {
            self.preview_handle = None;
        }
    }
}

struct InteractionState {
    dragging: bool,
    drag_origin: Option<Point>,
    drag_start_offset: Vector,
}

impl Default for InteractionState {
    fn default() -> Self {
        Self {
            dragging: false,
            drag_origin: None,
            drag_start_offset: Vector::new(0.0, 0.0),
        }
    }
}

// Wrapper for the image layer - draws the image with pan/zoom, no event handling
struct ImageLayer<'a>(&'a ImageViewer);

impl<'a> Program<Message> for ImageLayer<'a> {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        let viewer = self.0;
        let image_layer = viewer.image_cache.draw(renderer, bounds.size(), |frame| {
            frame.fill_rectangle(Point::ORIGIN, bounds.size(), Color::from_rgb8(18, 18, 18));

            if let Some(handle) = &viewer.image {
                // Check if we have a pre-computed cropped handle
                if let Some(cropped) = &viewer.cropped_handle {
                    // Use cached cropped image
                    frame.draw_image(
                        viewer.cropped_dest,
                        canvas::Image::new(cropped.clone())
                            .filter_method(iced::widget::image::FilterMethod::Nearest),
                    );
                } else {
                    // No clipping needed - draw full image
                    let scale = viewer.current_scale(bounds.size());
                    let dest_rect = Rectangle::new(
                        Point::new(viewer.offset.x, viewer.offset.y),
                        Size::new(
                            viewer.image_size.width * scale,
                            viewer.image_size.height * scale,
                        ),
                    );
                    frame.draw_image(
                        dest_rect,
                        canvas::Image::new(handle.clone())
                            .filter_method(iced::widget::image::FilterMethod::Nearest),
                    );
                }
            } else {
                frame.fill_text(canvas::Text {
                    content: "No image loaded".to_string(),
                    position: Point::new(bounds.width / 2.0 - 70.0, bounds.height / 2.0),
                    color: Color::from_rgb8(200, 200, 200),
                    ..Default::default()
                });
            }
        });

        vec![image_layer]
    }
}

// Wrapper for the overlay layer - draws overlays and handles all events
struct OverlayLayer<'a>(&'a ImageViewer);

impl<'a> Program<Message> for OverlayLayer<'a> {
    type State = InteractionState;

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        let viewer = self.0;
        let overlay = viewer.build_overlay_layer(renderer, bounds);
        vec![overlay]
    }

    fn update(
        &self,
        state: &mut Self::State,
        event: canvas::Event,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> (event::Status, Option<Message>) {
        let viewer = self.0;
        match event {
            canvas::Event::Mouse(mouse_event) => match mouse_event {
                mouse::Event::ButtonPressed(mouse::Button::Left) => {
                    if cursor.position_in(bounds).is_some() {
                        if let Some(global) = cursor.position() {
                            state.dragging = true;
                            state.drag_origin = Some(global);
                            state.drag_start_offset = viewer.offset;
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
                mouse::Event::ButtonPressed(mouse::Button::Middle) => (
                    event::Status::Captured,
                    Some(Message::Viewer(ViewerEvent::ToggleDebug)),
                ),
                mouse::Event::CursorMoved { .. } => {
                    let viewport_cursor = cursor.position_in(bounds);

                    if state.dragging {
                        if let Some(origin) = state.drag_origin {
                            if let Some(current) = cursor.position() {
                                let displacement =
                                    Vector::new(current.x - origin.x, current.y - origin.y);

                                let new_offset = state.drag_start_offset + displacement;

                                return (
                                    event::Status::Captured,
                                    Some(Message::Viewer(ViewerEvent::Pan {
                                        offset: new_offset,
                                        bounds: bounds.size(),
                                    })),
                                );
                            }
                        }
                    }

                    if let Some(cursor_pos) = viewport_cursor {
                        (
                            event::Status::Captured,
                            Some(Message::Viewer(ViewerEvent::Hover {
                                cursor: cursor_pos,
                                bounds: bounds.size(),
                            })),
                        )
                    } else {
                        (
                            event::Status::Captured,
                            Some(Message::Viewer(ViewerEvent::Leave)),
                        )
                    }
                }
                mouse::Event::WheelScrolled { delta } => {
                    let steps = match delta {
                        mouse::ScrollDelta::Lines { y, .. } => y,
                        mouse::ScrollDelta::Pixels { y, .. } => y / 120.0,
                    };

                    if steps.abs() > f32::EPSILON {
                        let factor = if steps > 0.0 { 1.1 } else { 0.9 };
                        let cursor_position = cursor.position_in(bounds).unwrap_or(Point::ORIGIN);
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
                    Some(Message::Viewer(ViewerEvent::Leave)),
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
