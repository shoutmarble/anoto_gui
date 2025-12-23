use iced::mouse::Cursor;
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Program, Stroke};
use iced::widget::{button, checkbox, column, container, pane_grid, row, stack, text, scrollable, text_editor, text_input};
use iced::{
    keyboard, mouse, window, Color, Element, Font, Length, Point, Rectangle, Size, Subscription,
    Task, Theme, Vector,
};
use iced_core::Bytes;
use image::{DynamicImage, RgbaImage};
use reqwest::Client;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anoto_dot_reader::anoto_decode::{
    decode_all_windows_from_minified_arrows, extract_best_decodable_window_from_minified_arrows,
};
use anoto_dot_reader::kornia::anoto::{detect_components_from_image, detect_grid, AnotoConfig};
use anoto_dot_reader::minify_arrow_grid::{minify_from_full_grid, write_grid_json_string};
use anoto_dot_reader::plot_grid::{build_intersection_grid, build_intersection_grid_observed, render_plot_rgba};

const JETBRAINS_FONT_BYTES: &[u8] =
    include_bytes!("../../assets/JetBrainsMono/fonts/ttf/JetBrainsMono-Medium.ttf");
const JETBRAINS_FONT: Font = Font::with_name("JetBrains Mono");

pub fn run_iced_app() -> iced::Result {
    iced::application(AnotoApp::new, AnotoApp::update, AnotoApp::view)
        .title(AnotoApp::title)
        .subscription(AnotoApp::subscription)
        .theme(AnotoApp::theme)
        .antialiasing(false)
        .window(window::Settings {
            size: Size::new(1240.0, 640.0),
            ..Default::default()
        })
        .font(JETBRAINS_FONT_BYTES)
        .default_font(JETBRAINS_FONT)
        .run()
}

#[derive(Debug, Clone, Copy)]
enum ExportKind {
    Plot,
    Anoto,
    Verify,
}

fn infer_section_from_path(path: &Path) -> Option<(usize, usize, i32, i32)> {
    let name = path.file_name()?.to_string_lossy();
    let parts: Vec<&str> = name.split("__").collect();
    if parts.len() < 5 {
        return None;
    }
    let prefix = parts[0];
    if prefix != "GUI_G" && prefix != "R" {
        return None;
    }
    let w: usize = parts.get(1)?.parse().ok()?;
    let h: usize = parts.get(2)?.parse().ok()?;
    let u: i32 = parts.get(3)?.parse().ok()?;
    let v: i32 = parts.get(4)?.parse().ok()?;
    Some((w, h, u, v))
}

fn grid_dims(grid: &[Vec<String>]) -> Option<(usize, usize)> {
    let h = grid.len();
    let w = grid.first().map(|r| r.len())?;
    if w == 0 || !grid.iter().all(|r| r.len() == w) {
        return None;
    }
    Some((h, w))
}

fn output_base_name(input_path: &Path, minified_json: &str) -> String {
    if let Some((w, h, u, v)) = infer_section_from_path(input_path) {
        return format!("R__{w}__{h}__{u}__{v}");
    }

    let grid: Vec<Vec<String>> = serde_json::from_str(minified_json).unwrap_or_default();
    if let Some((rows, cols)) = grid_dims(&grid) {
        return format!("R__{rows}__{cols}__10__10");
    }

    "R__0__0__10__10".to_string()
}

fn decode_xy_rows_from_minified_json(minified_json: &str) -> Vec<Vec<[i32; 2]>> {
    let grid: Vec<Vec<String>> = serde_json::from_str(minified_json).unwrap_or_default();
    use std::collections::{BTreeMap, BTreeSet};
    let mut rows = BTreeMap::<i32, BTreeSet<i32>>::new();
    for d in decode_all_windows_from_minified_arrows(&grid) {
        rows.entry(d.y).or_default().insert(d.x);
    }
    rows.into_iter()
        .map(|(y, xs)| xs.into_iter().map(|x| [x, y]).collect::<Vec<[i32; 2]>>())
        .collect()
}

fn write_verify_rows_compact(path: &Path, rows: &[Vec<[i32; 2]>]) -> Result<(), Box<dyn Error>> {
    let mut out = String::new();
    out.push_str("[\n");
    for (ri, row) in rows.iter().enumerate() {
        out.push_str("  [");
        for (ci, [x, y]) in row.iter().copied().enumerate() {
            if ci > 0 {
                out.push(',');
            }
            out.push_str(&format!("[{x},{y}]"));
        }
        out.push(']');
        if ri + 1 < rows.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("]\n");
    fs::write(path, out)?;
    Ok(())
}

async fn export_gui_task(
    kind: ExportKind,
    input_path: PathBuf,
    minified_json: String,
    annotated: Option<ImageData>,
) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        fs::create_dir_all("READER").map_err(|e| e.to_string())?;

        let base = output_base_name(&input_path, &minified_json);
        let out_anoto = PathBuf::from("READER").join(format!("{base}__ANOTO.json"));
        fs::write(&out_anoto, &minified_json).map_err(|e| e.to_string())?;

        match kind {
            ExportKind::Anoto => Ok(format!("Wrote {}", out_anoto.display())),
            ExportKind::Plot => {
                let Some(img) = annotated else {
                    return Err("No preview plot available".to_string());
                };
                let out_plot = PathBuf::from("READER").join(format!("{base}__PLOT.png"));
                let rgba = RgbaImage::from_raw(img.width, img.height, img.pixels)
                    .ok_or_else(|| "Invalid preview image buffer".to_string())?;
                rgba.save(&out_plot).map_err(|e| e.to_string())?;
                Ok(format!("Wrote {} and {}", out_plot.display(), out_anoto.display()))
            }
            ExportKind::Verify => {
                let out_verify = PathBuf::from("READER").join(format!("{base}__VERIFY.json"));
                let rows = decode_xy_rows_from_minified_json(&minified_json);
                write_verify_rows_compact(&out_verify, &rows).map_err(|e| e.to_string())?;
                Ok(format!("Wrote {} and {}", out_anoto.display(), out_verify.display()))
            }
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

struct AnotoApp {
    viewer: ImageViewer,
    status_text: String,
    last_loaded: Option<PathBuf>,
    is_loading: bool,
    panes: pane_grid::State<Pane>,
    decoded_text: String,
    decoded_editor: text_editor::Content,
    preview_minified_json: String,
    preview_minified_editor: text_editor::Content,
    last_annotated: Option<ImageData>,
    pattern_font_size: u32,
    shift_down: bool,
    caps_lock: bool,
    auto_post_enabled: bool,
    server_port: u16,
    server_port_input: String,
    last_posted_minified_json: Option<String>,
    last_post_attempt_at: Option<Instant>,
    post_in_flight: bool,
    http_client: Client,
}

#[derive(Debug, Clone)]
enum Message {
    LoadImagePressed,
    FilePicked(Option<PathBuf>),
    ImageLoaded(Result<LoadedImage, String>),
    Viewer(ViewerEvent),
    PaneResized(pane_grid::ResizeEvent),
    RegionSizeChanged(u32),
    PatternFontSizeChanged(u32),
    DetectionFinishedPreview(Result<DetectionPayload, String>),
    DetectionFinishedDecode(Result<DetectionPayload, String>),
    ExportPlotPressed,
    ExportAnotoPressed,
    ExportVerifyPressed,
    ExportFinished(Result<String, String>),
    PreviewMinifiedEditorAction(text_editor::Action),
    DecodedEditorAction(text_editor::Action),
    ShiftChanged(bool),
    CapsLockTapped,
    AutoPostToggled(bool),
    ServerPortChanged(String),
    PostFinished {
        result: Result<String, String>,
        posted_json: String,
    },
}

#[derive(Debug, Clone)]
struct DetectionPayload {
    decoded_text: String,
    origin: Option<(f32, f32)>,
    annotated: Option<ImageData>,
    minified_json: String,
}

#[derive(Debug, Clone)]
struct ImageData {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pane {
    Viewer,
    Data,
    Controls,
    Preview,
}

#[derive(Debug, Clone)]
enum ViewerEvent {
    Pan { offset: Vector, bounds: Size },
    Zoom {
        factor: f32,
        cursor: Point,
        bounds: Size,
    },
    Reset,
    Hover { cursor: Point, bounds: Size },
    Leave,
}

#[derive(Debug, Clone)]
struct LoadedImage {
    handle: iced::widget::image::Handle,
    size: Size,
    pixels: Bytes,
    path: PathBuf,
}

impl AnotoApp {
    const AUTO_POST_MIN_INTERVAL: Duration = Duration::from_millis(750);

    fn decoded_has_positions(decoded_text: &str) -> bool {
        let trimmed = decoded_text.trim();
        if trimmed.is_empty() {
            return false;
        }

        let rows: Vec<Vec<[i32; 2]>> = serde_json::from_str(trimmed).unwrap_or_default();
        rows.iter().any(|r| !r.is_empty())
    }

    fn can_attempt_post_now(&self) -> bool {
        self.last_post_attempt_at
            .is_none_or(|t| t.elapsed() >= Self::AUTO_POST_MIN_INTERVAL)
    }

    fn title(&self) -> String {
        "Anoto Dot Reader".to_string()
    }

    fn new() -> (Self, Task<Message>) {
        let (mut panes, viewer_pane) = pane_grid::State::new(Pane::Viewer);
        // Layout order (left -> right): Viewer | Preview | Data | Controls
        let (preview_pane, split_viewer_preview) = panes
            .split(pane_grid::Axis::Vertical, viewer_pane, Pane::Preview)
            .expect("failed to create preview pane");
        let (data_pane, split_preview_data) = panes
            .split(pane_grid::Axis::Vertical, preview_pane, Pane::Data)
            .expect("failed to create data pane");
        let (_controls_pane, split_data_controls) = panes
            .split(pane_grid::Axis::Vertical, data_pane, Pane::Controls)
            .expect("failed to create controls pane");

        // Initial widths (approx): Viewer 45% | Preview 23% | Data 23% | Controls 9%
        // Ratio is the fraction given to the *existing* pane in each split.
        panes.resize(split_viewer_preview, 0.45);
        panes.resize(split_preview_data, 0.42);
        panes.resize(split_data_controls, 0.72);

        (
            AnotoApp {
                viewer: ImageViewer::default(),
                status_text: "Load an image to begin".to_string(),
                last_loaded: None,
                is_loading: false,
                panes,
                decoded_text: "Hover to see annotated preview. Hold Shift or toggle Caps Lock to decode.".to_string(),
                decoded_editor: text_editor::Content::new(),
                preview_minified_json: String::new(),
                preview_minified_editor: text_editor::Content::new(),
                last_annotated: None,
                pattern_font_size: 9,
                shift_down: false,
                caps_lock: false,
                auto_post_enabled: false,
                server_port: 8080,
                server_port_input: "8080".to_string(),
                last_posted_minified_json: None,
                last_post_attempt_at: None,
                post_in_flight: false,
                http_client: Client::new(),
            },
            Task::none(),
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
                self.decoded_text =
                    "Hover to see annotated preview. Hold Shift or toggle Caps Lock to decode.".to_string();
                self.decoded_editor = text_editor::Content::new();
                self.preview_minified_json = String::new();
                self.preview_minified_editor = text_editor::Content::new();
                self.last_annotated = None;
                Task::none()
            }
            Message::ImageLoaded(Err(error)) => {
                self.status_text = format!("Failed to load image: {error}");
                self.is_loading = false;
                Task::none()
            }
            Message::DetectionFinishedDecode(Ok(payload)) => {
                self.viewer.set_detected_origin(payload.origin);
                if let Some(img) = payload.annotated {
                    let ImageData {
                        width,
                        height,
                        pixels,
                    } = img;
                    self.viewer
                        .set_preview_image(width, height, pixels.clone());
                    self.last_annotated = Some(ImageData {
                        width,
                        height,
                        pixels,
                    });
                } else {
                    self.last_annotated = None;
                }
                self.decoded_text = payload.decoded_text;
                self.decoded_editor = text_editor::Content::with_text(&self.decoded_text);
                self.preview_minified_json = payload.minified_json;
                self.preview_minified_editor =
                    text_editor::Content::with_text(&self.preview_minified_json);

                let should_post = self.auto_post_enabled
                    && !self.preview_minified_json.trim().is_empty()
                    && self.preview_minified_json.trim() != "[]"
                    && Self::decoded_has_positions(&self.decoded_text)
                    && self.can_attempt_post_now()
                    && !self.post_in_flight
                    && self
                        .last_posted_minified_json
                        .as_deref()
                        .is_none_or(|prev| prev != self.preview_minified_json);

                if should_post {
                    self.last_post_attempt_at = Some(Instant::now());
                    self.post_in_flight = true;
                    let client = self.http_client.clone();
                    let minified_json = self.preview_minified_json.clone();
                    let port = self.server_port;
                    let json_for_future = minified_json.clone();
                    return Task::perform(post_to_server(client, json_for_future, port), move |result| {
                        Message::PostFinished {
                            result,
                            posted_json: minified_json,
                        }
                    });
                }

                Task::none()
            }
            Message::DetectionFinishedDecode(Err(err)) => {
                self.decoded_text = format!("Detection failed: {err}");
                self.decoded_editor = text_editor::Content::with_text(&self.decoded_text);
                Task::none()
            }
            Message::DetectionFinishedPreview(Ok(payload)) => {
                if let Some(img) = payload.annotated {
                    let ImageData {
                        width,
                        height,
                        pixels,
                    } = img;
                    self.viewer
                        .set_preview_image(width, height, pixels.clone());
                    self.last_annotated = Some(ImageData {
                        width,
                        height,
                        pixels,
                    });
                } else {
                    self.last_annotated = None;
                }
                self.preview_minified_json = payload.minified_json;
                self.preview_minified_editor =
                    text_editor::Content::with_text(&self.preview_minified_json);

                // Keep the decoded positions panel updated even on preview detection.
                self.decoded_text = payload.decoded_text;
                self.decoded_editor = text_editor::Content::with_text(&self.decoded_text);

                let should_post = self.auto_post_enabled
                    && !self.preview_minified_json.trim().is_empty()
                    && self.preview_minified_json.trim() != "[]"
                    && Self::decoded_has_positions(&self.decoded_text)
                    && self.can_attempt_post_now()
                    && !self.post_in_flight
                    && self
                        .last_posted_minified_json
                        .as_deref()
                        .is_none_or(|prev| prev != self.preview_minified_json);

                if should_post {
                    self.last_post_attempt_at = Some(Instant::now());
                    self.post_in_flight = true;
                    let client = self.http_client.clone();
                    let minified_json = self.preview_minified_json.clone();
                    let port = self.server_port;
                    let json_for_future = minified_json.clone();
                    return Task::perform(post_to_server(client, json_for_future, port), move |result| {
                        Message::PostFinished {
                            result,
                            posted_json: minified_json,
                        }
                    });
                }

                Task::none()
            }
            Message::DetectionFinishedPreview(Err(_)) => Task::none(),

            Message::ExportPlotPressed => {
                let Some(path) = self.last_loaded.clone() else {
                    self.status_text = "Load an image first".to_string();
                    return Task::none();
                };
                let Some(img) = self.last_annotated.clone() else {
                    self.status_text = "Hover over the image to generate a preview plot first".to_string();
                    return Task::none();
                };
                if self.preview_minified_json.is_empty() {
                    self.status_text = "No anoto pattern available to export".to_string();
                    return Task::none();
                }

                self.status_text = "Exporting plot...".to_string();
                let minified_json = self.preview_minified_json.clone();
                Task::perform(export_gui_task(ExportKind::Plot, path, minified_json, Some(img)), Message::ExportFinished)
            }
            Message::ExportAnotoPressed => {
                let Some(path) = self.last_loaded.clone() else {
                    self.status_text = "Load an image first".to_string();
                    return Task::none();
                };
                if self.preview_minified_json.is_empty() {
                    self.status_text = "No anoto pattern available to export".to_string();
                    return Task::none();
                }

                self.status_text = "Exporting anoto pattern...".to_string();
                let minified_json = self.preview_minified_json.clone();
                Task::perform(export_gui_task(ExportKind::Anoto, path, minified_json, None), Message::ExportFinished)
            }
            Message::ExportVerifyPressed => {
                let Some(path) = self.last_loaded.clone() else {
                    self.status_text = "Load an image first".to_string();
                    return Task::none();
                };
                if self.preview_minified_json.is_empty() {
                    self.status_text = "No anoto pattern available to export".to_string();
                    return Task::none();
                }

                self.status_text = "Exporting decoded (x,y)...".to_string();
                let minified_json = self.preview_minified_json.clone();
                Task::perform(export_gui_task(ExportKind::Verify, path, minified_json, None), Message::ExportFinished)
            }
            Message::ExportFinished(Ok(msg)) => {
                self.status_text = msg;
                Task::none()
            }
            Message::ExportFinished(Err(err)) => {
                self.status_text = format!("Export failed: {err}");
                Task::none()
            }
            Message::PreviewMinifiedEditorAction(action) => {
                // Keep selection state updated, but prevent editing.
                if !action.is_edit() {
                    self.preview_minified_editor.perform(action.clone());
                }

                // Clicking/focusing the minified JSON should copy it to the clipboard.
                // (Selection + Ctrl+C will also work naturally.)
                if !self.preview_minified_json.is_empty()
                    && let text_editor::Action::Click(_) = action
                {
                    return iced::clipboard::write(self.preview_minified_json.clone());
                }

                Task::none()
            }
            Message::DecodedEditorAction(action) => {
                // Read-only textarea: keep selection/cursor/scroll, but prevent edits.
                if !action.is_edit() {
                    self.decoded_editor.perform(action);
                }

                Task::none()
            }
            Message::ShiftChanged(down) => {
                self.shift_down = down;
                self.apply_lock_state(down)
            }
            Message::CapsLockTapped => {
                self.caps_lock = !self.caps_lock;
                self.apply_lock_state(self.caps_lock)
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
            Message::PatternFontSizeChanged(size) => {
                self.pattern_font_size = size;
                Task::none()
            }
            Message::AutoPostToggled(enabled) => {
                self.auto_post_enabled = enabled;
                if enabled {
                    self.status_text = format!("Auto-POST enabled to localhost:{}", self.server_port);
                } else {
                    self.status_text = "Auto-POST disabled".to_string();
                }
                Task::none()
            }
            Message::ServerPortChanged(port_str) => {
                self.server_port_input = port_str.clone();
                if let Ok(port) = port_str.trim().parse::<u16>() {
                    self.server_port = port;
                    if self.auto_post_enabled {
                        self.status_text = format!("Server port updated to {}", port);
                    }
                } else if self.auto_post_enabled && !port_str.trim().is_empty() {
                    self.status_text = "Invalid port (1-65535)".to_string();
                }
                Task::none()
            }
            Message::PostFinished { result, posted_json } => {
                self.post_in_flight = false;
                match result {
                    Ok(msg) => {
                        self.status_text = msg;
                        if !posted_json.trim().is_empty() && posted_json.trim() != "[]" {
                            self.last_posted_minified_json = Some(posted_json);
                        }
                    }
                    Err(err) => {
                        self.status_text = format!("POST failed: {err}");
                    }
                }
                Task::none()
            }
        }
    }

    fn apply_lock_state(&mut self, trigger_detection: bool) -> Task<Message> {
        let should_lock = self.shift_down || self.caps_lock;
        let changed = self.viewer.preview_locked != should_lock;
        self.viewer.preview_locked = should_lock;

        if should_lock && (trigger_detection || changed) {
            if let Some((aoi_pixels, region_size)) = self.viewer.current_aoi() {
                let size = Size::new(region_size as f32, region_size as f32);
                return Task::perform(
                    run_detection_task(aoi_pixels, size),
                    Message::DetectionFinishedDecode,
                );
            }
            self.decoded_text = "Hover over the image before locking to decode.".to_string();
        }

        if !should_lock && changed {
            self.viewer.refresh_hover();
        }

        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        pane_grid::PaneGrid::new(&self.panes, |_, pane, _| match pane {
            Pane::Viewer => pane_grid::Content::new(self.viewer_section()),
            Pane::Controls => pane_grid::Content::new(self.controls_section()),
            Pane::Preview => pane_grid::Content::new(self.preview_section()),
            Pane::Data => pane_grid::Content::new(self.data_section()),
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .on_resize(10, Message::PaneResized)
        .into()
    }

    fn viewer_section(&self) -> Element<'_, Message> {
        let image_canvas = Canvas::new(ImageLayer(&self.viewer))
            .width(Length::Fill)
            .height(Length::Fill);

        let overlay_canvas = Canvas::new(OverlayLayer(&self.viewer))
            .width(Length::Fill)
            .height(Length::Fill);

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
            button(text("Loading...").font(JETBRAINS_FONT))
                .width(Length::Fill)
                .into()
        } else {
            button(text("Open Image").font(JETBRAINS_FONT))
                .on_press(Message::LoadImagePressed)
                .width(Length::Fill)
                .into()
        };

        let zoom_label: Element<'_, Message> = text(self.viewer.zoom_label())
            .size(16)
            .font(JETBRAINS_FONT)
            .into();

        let status_label: Element<'_, Message> = text(&self.status_text)
            .size(12)
            .font(JETBRAINS_FONT)
            .into();

        let region_size_label: Element<'_, Message> =
            text(format!("AOI Size: {}px", self.viewer.region_size()))
                .size(14)
                .font(JETBRAINS_FONT)
                .into();

        let region_size_slider: Element<'_, Message> = iced::widget::slider(
            ImageViewer::MIN_REGION_SIZE..=self.viewer.max_region_size().max(ImageViewer::MIN_REGION_SIZE),
            self.viewer.region_size(),
            Message::RegionSizeChanged,
        )
        .width(Length::Fill)
        .into();

        let legend_style = |_: &_| container::Style {
            background: None,
            border: iced::border::Border {
                color: Color::from_rgb8(100, 100, 100),
                width: 1.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        };

        let zoom_box = container(zoom_label)
            .padding(8)
            .style(legend_style)
            .width(Length::Fill);

        let loaded_box = container(status_label)
            .padding(8)
            .style(legend_style)
            .width(Length::Fill);

        let region_box = container(column![region_size_label, region_size_slider].spacing(8))
            .padding(8)
            .style(legend_style)
            .width(Length::Fill);

        let pattern_font_label: Element<'_, Message> =
            text(format!("Pattern Font: {}", self.pattern_font_size))
                .size(14)
                .font(JETBRAINS_FONT)
                .into();

        let pattern_font_slider: Element<'_, Message> = iced::widget::slider(
            6u32..=14u32,
            self.pattern_font_size,
            Message::PatternFontSizeChanged,
        )
        .width(Length::Fill)
        .into();

        let pattern_font_box =
            container(column![pattern_font_label, pattern_font_slider].spacing(8))
                .padding(8)
                .style(legend_style)
                .width(Length::Fill);

        let export_plot = button(text("Export Plot").font(JETBRAINS_FONT))
            .on_press(Message::ExportPlotPressed)
            .width(Length::Fill);
        let export_anoto = button(text("Export Anoto Pattern").font(JETBRAINS_FONT))
            .on_press(Message::ExportAnotoPressed)
            .width(Length::Fill);
        let export_verify = button(text("Export Decoded (x,y)").font(JETBRAINS_FONT))
            .on_press(Message::ExportVerifyPressed)
            .width(Length::Fill);

        let layout = column![
            open_button,
            zoom_box,
            loaded_box,
            region_box,
            pattern_font_box,
            export_plot,
            export_anoto,
            export_verify
        ]
            .spacing(16)
            .width(Length::Fill)
            .padding(20);

        container(layout)
            .width(Length::Fill)
            .style(|_| container::Style {
                background: Some(Color::from_rgb8(32, 32, 32).into()),
                ..Default::default()
            })
            .into()
    }

    fn preview_section(&self) -> Element<'_, Message> {
        let legend_style = |_: &_| container::Style {
            background: None,
            border: iced::border::Border {
                color: Color::from_rgb8(100, 100, 100),
                width: 1.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        };

        let lock_text = if self.viewer.preview_locked {
            "LOCKED (Shift/Caps Lock)"
        } else {
            "LIVE (hover)"
        };

        let lock_indicator: Element<'_, Message> = container(text(lock_text).size(12).font(JETBRAINS_FONT))
            .padding(8)
            .width(Length::Fill)
            .style(legend_style)
            .into();

        let preview_image: Element<'_, Message> = if let Some(handle) = self.viewer.preview_handle() {
            container(iced::widget::image(handle.clone()))
                .width(Length::Fill)
                .padding(8)
                .style(legend_style)
                .into()
        } else {
            container(text("Hover over the image to see a preview").size(12).font(JETBRAINS_FONT))
                .width(Length::Fill)
                .padding(8)
                .style(legend_style)
                .into()
        };

        let preview_content: Element<'_, Message> = column![preview_image]
            .spacing(10)
            .width(Length::Fill)
            .into();

        let preview_legend: Element<'_, Message> = column![
            container(text(" Preview ").size(12).font(JETBRAINS_FONT)).style(|_| container::Style {
                background: Some(Color::from_rgb8(32, 32, 32).into()),
                ..Default::default()
            }),
            container(preview_content)
                .padding(0)
                .width(Length::Fill)
                .style(legend_style)
        ]
        .spacing(0)
        .into();

        let layout = column![lock_indicator, preview_legend]
            .spacing(16)
            .width(Length::Fill)
            .padding(20);

        container(layout)
            .width(Length::Fill)
            .style(|_| container::Style {
                background: Some(Color::from_rgb8(32, 32, 32).into()),
                ..Default::default()
            })
            .into()
    }

    fn data_section(&self) -> Element<'_, Message> {
        let legend_style = |_: &_| container::Style {
            background: None,
            border: iced::border::Border {
                color: Color::from_rgb8(100, 100, 100),
                width: 1.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        };

        // Anoto pattern (minified JSON) text view
        let auto_post_row: Element<'_, Message> = {
            let url = format!("http://localhost:{}/decode", self.server_port);

            let toggle = checkbox(self.auto_post_enabled)
                .label("Auto POST")
                .on_toggle(Message::AutoPostToggled)
                .size(14)
                .font(JETBRAINS_FONT);

            let port_input = text_input("8080", &self.server_port_input)
                .on_input(Message::ServerPortChanged)
                .size(14)
                .font(JETBRAINS_FONT)
                .width(Length::Fixed(80.0));

            let port_label = text("Port:").size(12).font(JETBRAINS_FONT);
            let url_label = text(url).size(12).font(JETBRAINS_FONT);

            row![toggle, port_label, port_input, url_label]
                .spacing(10)
                .align_y(iced::Alignment::Center)
                .into()
        };

        let pattern_body: Element<'_, Message> = if self.preview_minified_json.is_empty() {
            container(text("No anoto pattern available").size(12).font(JETBRAINS_FONT))
                .padding(8)
                .width(Length::Fill)
                .into()
        } else {
            // Read-only textarea so the JSON can be selected and copied.
            let editor = text_editor(&self.preview_minified_editor)
                .on_action(Message::PreviewMinifiedEditorAction)
                .font(JETBRAINS_FONT)
                .size(self.pattern_font_size)
                .height(Length::Fixed(180.0));

            container(editor).padding(8).width(Length::Fill).into()
        };

        let pattern_view: Element<'_, Message> = column![
            container(text(" Anoto Pattern ").size(12).font(JETBRAINS_FONT)).style(|_| container::Style {
                background: Some(Color::from_rgb8(32, 32, 32).into()),
                ..Default::default()
            }),
            container(auto_post_row)
                .padding(8)
                .width(Length::Fill)
                .style(legend_style),
            container(pattern_body)
                .padding(0)
                .width(Length::Fill)
                .style(legend_style)
        ]
        .spacing(0)
        .into();

        // Decoded (X,Y) positions textarea
        let decoded_positions_body: Element<'_, Message> = {
            let editor = text_editor(&self.decoded_editor)
                .on_action(Message::DecodedEditorAction)
                .font(JETBRAINS_FONT)
                .size(10)
                .height(Length::Fixed(200.0));

            container(editor).padding(8).width(Length::Fill).into()
        };

        let decoded_positions_view: Element<'_, Message> = column![
            container(text(" decoded (X,Y) positions ").size(12).font(JETBRAINS_FONT)).style(|_| container::Style {
                background: Some(Color::from_rgb8(32, 32, 32).into()),
                ..Default::default()
            }),
            container(decoded_positions_body)
                .padding(0)
                .width(Length::Fill)
                .style(legend_style)
        ]
        .spacing(0)
        .into();

        let layout = column![pattern_view, decoded_positions_view]
            .spacing(16)
            .width(Length::Fill)
            .padding(20);

        container(scrollable(layout).height(Length::Fill))
            .width(Length::Fill)
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
    // Caches
    image_cache: canvas::Cache,

    pixels: Option<Bytes>,
    hover_viewport_pos: Option<Point>,
    hover_image_pos: Option<Point>,
    hover_overlay_center: Option<Point>,
    preview_handle: Option<iced::widget::image::Handle>,
    last_preview_region: Option<(u32, u32, u32)>,
    last_preview_detection_region: Option<(u32, u32, u32)>,
    last_preview_detection_at: Option<Instant>,
    // AOI size in source pixels (drives both the overlay box and preview extraction)
    region_size: u32,
    detected_origin: Option<Point>,
    preview_locked: bool,
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
            image_cache: canvas::Cache::default(),
            pixels: None,
            hover_viewport_pos: None,
            hover_image_pos: None,
            hover_overlay_center: None,
            preview_handle: None,
            last_preview_region: None,
            last_preview_detection_region: None,
            last_preview_detection_at: None,
            region_size: 40,
            detected_origin: None,
            preview_locked: false,
        }
    }
}

impl ImageViewer {
    const MIN_REGION_SIZE: u32 = 10;
    const AOI_STROKE_WIDTH: f32 = 2.67;
    const PREVIEW_DETECTION_MIN_INTERVAL: Duration = Duration::from_millis(100);

    fn set_image(&mut self, handle: iced::widget::image::Handle, size: Size, pixels: Bytes) {
        self.image = Some(handle);
        self.image_size = size;
        self.image_dimensions = (size.width.round() as u32, size.height.round() as u32);
        let min_dim = self.image_dimensions.0.min(self.image_dimensions.1);
        let auto_region = (min_dim / 8).max(Self::MIN_REGION_SIZE);
        self.region_size = auto_region;
        self.zoom_mode = ZoomMode::Fit;
        self.custom_scale = 1.0;
        self.offset = Vector::new(0.0, 0.0);
        self.pixels = Some(pixels);
        self.hover_viewport_pos = None;
        self.hover_image_pos = None;
        self.hover_overlay_center = None;
        self.preview_handle = None;
        self.last_preview_region = None;
        self.last_preview_detection_region = None;
        self.last_preview_detection_at = None;
        self.detected_origin = None;
        self.preview_locked = false;
        self.invalidate_image_layer();
    }

    fn invalidate_image_layer(&mut self) {
        self.image_cache.clear();
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
                if self.preview_locked {
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
                if self.preview_locked {
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
            ViewerEvent::Hover { cursor, bounds } => {
                let _ = self.apply_viewport_resize(bounds);
                if !self.preview_locked {
                    self.hover_viewport_pos = Some(cursor);
                    self.refresh_hover();

                    // Only kick off preview detection when the AOI region actually changes.
                    if let Some(key) = self.current_aoi_region_key() {
                        if self.last_preview_detection_region != Some(key) {
                            if let Some(last) = self.last_preview_detection_at
                                && Instant::now().duration_since(last)
                                    < Self::PREVIEW_DETECTION_MIN_INTERVAL
                            {
                                return Task::none();
                            }

                            self.last_preview_detection_region = Some(key);
                            self.last_preview_detection_at = Some(Instant::now());
                            if let Some((aoi_pixels, region_size)) = self.current_aoi() {
                                let size = Size::new(region_size as f32, region_size as f32);
                                return Task::perform(
                                    run_detection_task(aoi_pixels, size),
                                    Message::DetectionFinishedPreview,
                                );
                            }
                        }
                    } else {
                        self.last_preview_detection_region = None;
                    }
                }
                Task::none()
            }
            ViewerEvent::Leave => {
                if !self.preview_locked {
                    self.hover_viewport_pos = None;
                    self.hover_image_pos = None;
                    self.hover_overlay_center = None;
                    self.preview_handle = None;
                    self.last_preview_region = None;
                    self.last_preview_detection_region = None;
                    self.last_preview_detection_at = None;
                }
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

    fn set_preview_image(&mut self, width: u32, height: u32, pixels: Vec<u8>) {
        self.preview_handle = Some(iced::widget::image::Handle::from_rgba(width, height, pixels));
    }

    fn region_size(&self) -> u32 {
        self.region_size
    }

    fn set_region_size(&mut self, size: u32) {
        let max = self.max_region_size();
        self.region_size = size.clamp(Self::MIN_REGION_SIZE, max);
        self.last_preview_detection_region = None;
        self.last_preview_detection_at = None;
        // Refresh preview with new region size
        if let Some(image_point) = self.hover_image_pos {
            self.update_preview(image_point);
        }
    }

    fn set_detected_origin(&mut self, origin: Option<(f32, f32)>) {
        self.detected_origin = origin.map(|(x, y)| Point::new(x, y));
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

        if self.preview_locked {
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
                            .with_width(Self::AOI_STROKE_WIDTH)
                            .with_color(Color::from_rgb(1.0, 0.0, 1.0)),
                    );
                    frame.fill_rectangle(
                        Point::new(center.x - 2.0, center.y - 2.0),
                        Size::new(4.0, 4.0),
                        Color::WHITE,
                    );
                }

                if let Some(origin) = self.detected_origin {
                    let scale = self.current_scale(bounds.size());
                    let origin_screen = Point::new(
                        origin.x * scale + self.offset.x,
                        origin.y * scale + self.offset.y,
                    );

                    let marker_size = 8.0;
                    let marker_half = marker_size / 2.0;
                    frame.fill_rectangle(
                        Point::new(origin_screen.x - marker_half, origin_screen.y - 1.5),
                        Size::new(marker_size, 3.0),
                        Color::from_rgb(0.95, 0.8, 0.1),
                    );
                    frame.fill_rectangle(
                        Point::new(origin_screen.x - 1.5, origin_screen.y - marker_half),
                        Size::new(3.0, marker_size),
                        Color::from_rgb(0.95, 0.8, 0.1),
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
        let Some((start_x, start_y, region_size)) = self.preview_region(image_point) else {
            self.preview_handle = None;
            self.last_preview_region = None;
            return;
        };

        let region_key = (start_x, start_y, region_size);
        if self.last_preview_region == Some(region_key) {
            return;
        }
        self.last_preview_region = Some(region_key);

        let Some(region_bytes) = self.extract_region_pixels(start_x, start_y, region_size) else {
            self.preview_handle = None;
            return;
        };

        // Use raw pixels at 1:1 for pixel-perfect display
        self.preview_handle = Some(iced::widget::image::Handle::from_rgba(
            region_size,
            region_size,
            region_bytes,
        ));
    }

    fn preview_region(&self, image_point: Point) -> Option<(u32, u32, u32)> {
        let (width_px, height_px) = self.image_dimensions;
        if width_px == 0 || height_px == 0 {
            return None;
        }

        let region_size = self.region_size.min(width_px).min(height_px);
        if region_size == 0 {
            return None;
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

        Some((start_x as u32, start_y as u32, region_size))
    }

    fn extract_region_pixels(&self, start_x: u32, start_y: u32, region_size: u32) -> Option<Vec<u8>> {
        let pixels = self.pixels.as_ref()?;
        let pixels: &[u8] = pixels.as_ref();
        let (width_px, height_px) = self.image_dimensions;
        if width_px == 0 || height_px == 0 || region_size == 0 {
            return None;
        }

        let bytes_per_pixel = 4usize;
        let row_len = (region_size as usize) * bytes_per_pixel;
        let mut region = vec![0u8; (region_size as usize) * (region_size as usize) * bytes_per_pixel];

        for row in 0..region_size {
            let src_y = start_y + row;
            let src_index = ((src_y * width_px) + start_x) as usize * bytes_per_pixel;
            let dst_index = (row as usize) * row_len;

            if src_index + row_len <= pixels.len() {
                region[dst_index..dst_index + row_len]
                    .copy_from_slice(&pixels[src_index..src_index + row_len]);
            }
        }

        Some(region)
    }

    fn current_aoi(&self) -> Option<(Vec<u8>, u32)> {
        let image_point = self.hover_image_pos?;
        let (start_x, start_y, region_size) = self.preview_region(image_point)?;
        let region = self.extract_region_pixels(start_x, start_y, region_size)?;
        Some((region, region_size))
    }

    fn current_aoi_region_key(&self) -> Option<(u32, u32, u32)> {
        let image_point = self.hover_image_pos?;
        self.preview_region(image_point)
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
                // Let the renderer handle clipping/scissoring; avoid CPU-side cropping on pan/zoom.
                let clip = Rectangle::new(Point::ORIGIN, bounds.size());
                frame.with_clip(clip, |frame| {
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
                });
            } else {
                frame.fill_text(canvas::Text {
                    content: "No image loaded".to_string(),
                    position: Point::new(bounds.width / 2.0 - 70.0, bounds.height / 2.0),
                    color: Color::from_rgb8(200, 200, 200),
                    font: JETBRAINS_FONT,
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
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> Option<iced::widget::Action<Message>> {
        let viewer = self.0;
        let in_bounds = cursor.position_in(bounds).is_some();

        match event {
            canvas::Event::Mouse(mouse_event) => match mouse_event {
                mouse::Event::ButtonPressed(mouse::Button::Left) => {
                    if in_bounds {
                        if let Some(global) = cursor.position() {
                            state.dragging = true;
                            state.drag_origin = Some(global);
                            state.drag_start_offset = viewer.offset;
                        } else {
                            state.dragging = false;
                            state.drag_origin = None;
                        }
                        Some(iced::widget::Action::capture())
                    } else {
                        None
                    }
                }
                mouse::Event::ButtonReleased(mouse::Button::Left) => {
                    if state.dragging || in_bounds {
                        state.dragging = false;
                        state.drag_origin = None;
                        Some(iced::widget::Action::capture())
                    } else {
                        None
                    }
                }
                mouse::Event::ButtonPressed(mouse::Button::Right) => {
                    if in_bounds {
                        Some(
                            iced::widget::Action::publish(Message::Viewer(ViewerEvent::Reset))
                                .and_capture(),
                        )
                    } else {
                        None
                    }
                }
                mouse::Event::CursorMoved { .. } => {
                    let viewport_cursor = cursor.position_in(bounds);

                    if state.dragging
                        && let Some(origin) = state.drag_origin
                        && let Some(current) = cursor.position()
                    {
                        let displacement =
                            Vector::new(current.x - origin.x, current.y - origin.y);

                        let new_offset = state.drag_start_offset + displacement;

                        return Some(
                            iced::widget::Action::publish(Message::Viewer(ViewerEvent::Pan {
                                offset: new_offset,
                                bounds: bounds.size(),
                            }))
                            .and_capture(),
                        );
                    }

                    if let Some(cursor_pos) = viewport_cursor {
                        Some(
                            iced::widget::Action::publish(Message::Viewer(ViewerEvent::Hover {
                                cursor: cursor_pos,
                                bounds: bounds.size(),
                            }))
                            .and_capture(),
                        )
                    } else {
                        // Do not capture out-of-bounds movement; otherwise this Canvas can
                        // starve other widgets (e.g., buttons) from receiving mouse events.
                        Some(iced::widget::Action::publish(Message::Viewer(ViewerEvent::Leave)))
                    }
                }
                mouse::Event::WheelScrolled { delta } => {
                    if !in_bounds {
                        return None;
                    }

                    let steps = match delta {
                        mouse::ScrollDelta::Lines { y, .. } => *y,
                        mouse::ScrollDelta::Pixels { y, .. } => *y / 120.0,
                    };

                    if steps.abs() > f32::EPSILON {
                        let factor = if steps > 0.0 { 1.1 } else { 0.9 };
                        let cursor_position = cursor.position_in(bounds).unwrap_or(Point::ORIGIN);
                        Some(
                            iced::widget::Action::publish(Message::Viewer(ViewerEvent::Zoom {
                                factor,
                                cursor: cursor_position,
                                bounds: bounds.size(),
                            }))
                            .and_capture(),
                        )
                    } else {
                        None
                    }
                }
                mouse::Event::CursorLeft => {
                    Some(iced::widget::Action::publish(Message::Viewer(ViewerEvent::Leave)))
                }
                _ => None,
            },
            canvas::Event::Keyboard(key_event) => match key_event {
                keyboard::Event::KeyPressed { key, .. } => {
                    if matches!(key, keyboard::Key::Named(keyboard::key::Named::Shift)) {
                        return Some(
                            iced::widget::Action::publish(Message::ShiftChanged(true)),
                        );
                    }
                    if matches!(key, keyboard::Key::Named(keyboard::key::Named::CapsLock)) {
                        return Some(
                            iced::widget::Action::publish(Message::CapsLockTapped),
                        );
                    }
                    None
                }
                keyboard::Event::KeyReleased { key, .. } => {
                    if matches!(key, keyboard::Key::Named(keyboard::key::Named::Shift)) {
                        return Some(
                            iced::widget::Action::publish(Message::ShiftChanged(false)),
                        );
                    }
                    None
                }
                _ => None,
            },
            _ => None,
        }
    }
}

async fn load_image_task(path: PathBuf) -> Result<LoadedImage, String> {
    let original_path = path.clone();
    tokio::task::spawn_blocking(move || {
        let image = image::open(&original_path).map_err(|err| err.to_string())?;
        let rgba = image.to_rgba8();
        let (width, height) = rgba.dimensions();
        let pixels: Bytes = rgba.into_raw().into();
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

async fn post_to_server(client: Client, minified_json: String, port: u16) -> Result<String, String> {
    fn body_snippet(s: &str) -> String {
        const MAX: usize = 2048;
        if s.len() <= MAX {
            return s.to_string();
        }
        format!("{}...", &s[..MAX])
    }

    let url = format!("http://localhost:{}/decode", port);

    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .timeout(Duration::from_secs(2))
        .body(minified_json)
        .send()
        .await
        .map_err(|e| format!("Failed to POST to {}: {}", url, e))?;

    let status = response.status();
    if status.is_success() {
        Ok(format!("Posted to {} (status: {})", url, status))
    } else {
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "<failed to read response body>".to_string());
        Err(format!(
            "Server returned error: {} ({})",
            status,
            body_snippet(body.trim())
        ))
    }
}

async fn run_detection_task(pixels: Vec<u8>, size: Size) -> Result<DetectionPayload, String> {
    tokio::task::spawn_blocking(move || {
        let width = size.width.round() as u32;
        let height = size.height.round() as u32;
        if width == 0 || height == 0 {
            return Ok(DetectionPayload {
                decoded_text: "Empty image".to_string(),
                origin: None,
                annotated: None,
                minified_json: String::new(),
            });
        }

        let rgba: RgbaImage = RgbaImage::from_raw(width, height, pixels)
            .ok_or_else(|| "Invalid pixel buffer dimensions".to_string())?;
        let dyn_img = DynamicImage::ImageRgba8(rgba);

        let config = AnotoConfig::default();
        let components =
            detect_components_from_image(&dyn_img, &config).map_err(|e| e.to_string())?;

        let origin = match detect_grid(&components, &config) {
            Some((_rows, _cols, _grid, origin)) => origin,
            None => None,
        };

        // Build preview JSON: minified arrows derived from the intersection-grid structure.
        // Also decode Anoto page coordinates (x,y) from the minified arrows only.
        // We build both variants and pick the one with best arrow coverage.
        let full_grid_observed = build_intersection_grid_observed(&components, &config);
        let full_grid_phase = build_intersection_grid(&components, &config);

        let minified_observed = minify_from_full_grid(&full_grid_observed);
        let minified_phase = minify_from_full_grid(&full_grid_phase);

        let arrow_count = |g: &[Vec<String>]| {
            g.iter()
                .flat_map(|r| r.iter())
                .filter(|v| matches!(v.as_str(), "↑" | "↓" | "←" | "→"))
                .count()
        };

        let mut minified_full = if arrow_count(&minified_phase) >= arrow_count(&minified_observed) {
            minified_phase
        } else {
            minified_observed
        };

        // GUI uses the same default as CLI: crop to a best 8x8 window.
        if let Some(window) = extract_best_decodable_window_from_minified_arrows(&minified_full, 8, 8) {
            minified_full = window;
        }

        let minified_json = if minified_full.is_empty() {
            "[]".to_string()
        } else {
            write_grid_json_string(&minified_full)
        };

        // Decoded (x,y) panel: show compact JSON rows, same shape as CLI VERIFY.json.
        let decoded_text = if minified_full.is_empty() {
            "[]\n".to_string()
        } else {
            use std::collections::{BTreeMap, BTreeSet};
            let mut rows = BTreeMap::<i32, BTreeSet<i32>>::new();
            for d in decode_all_windows_from_minified_arrows(&minified_full) {
                rows.entry(d.y).or_default().insert(d.x);
            }

            let mut out = String::new();
            out.push_str("[\n");
            let total_rows = rows.len();
            for (ri, (y, xs)) in rows.into_iter().enumerate() {
                out.push_str("  [");
                for (ci, x) in xs.into_iter().enumerate() {
                    if ci > 0 {
                        out.push(',');
                    }
                    out.push_str(&format!("[{x},{y}]"));
                }
                out.push(']');
                if ri + 1 < total_rows {
                    out.push(',');
                }
                out.push('\n');
            }
            out.push_str("]\n");
            out
        };

        // Preview image: plotters rendering (same style as CLI --plot).
        let annotated = match render_plot_rgba(width, height, &components, &config) {
            Ok(buf) if !buf.is_empty() => Some(ImageData {
                width,
                height,
                pixels: buf,
            }),
            Ok(_) => None,
            Err(e) => return Err(e),
        };

        Ok(DetectionPayload {
            decoded_text,
            origin,
            annotated,
            minified_json,
        })
    })
    .await
    .map_err(|err| err.to_string())?
}
