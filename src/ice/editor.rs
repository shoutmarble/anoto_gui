use iced::task::Task;
use iced::widget::image::Handle;
use iced::widget::{button, column, image as iced_image, mouse_area, row, text, text_input};
use iced::{Element, Length, Point};
use std::io::Cursor;
use std::path::{Path, PathBuf};
use image;
use image::GenericImageView;

#[derive(Default)]
#[allow(dead_code)]
struct ImageViewer {
    loaded_image: Option<Handle>,
    loaded_path: Option<PathBuf>,
    input_path: String,
    status_message: Option<String>,
    image_data: Option<image::DynamicImage>,
    image_width: u32,
    image_height: u32,
    is_hovering: bool,
    magnified_image: Option<Handle>,
    last_mouse_x: f32,
    last_mouse_y: f32,
}

impl ImageViewer {
    #[allow(dead_code)]
    fn load_image_from_path(&mut self, path: &Path) -> Result<(), String> {
        let img = image::open(path).map_err(|e| e.to_string())?;
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        self.image_data = Some(img.clone());
        self.image_width = img.width();
        self.image_height = img.height();
        self.loaded_image = Some(Handle::from_bytes(bytes));
        self.loaded_path = Some(path.to_path_buf());
        self.status_message = Some(format!("Loaded {}", path.display()));
        Ok(())
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Message {
    OpenFileDialog,
    FileSelected(Option<PathBuf>),
    PathChanged(String),
    SubmitPath,
    MouseEntered,
    MouseLeft,
    MouseMoved(Point),
}

#[allow(dead_code)]
fn update(state: &mut ImageViewer, message: Message) -> Task<Message> {
    match message {
        Message::OpenFileDialog => Task::perform(
            async {
                rfd::FileDialog::new()
                    .add_filter("Images", &["png", "jpg", "jpeg", "gif", "bmp"])
                    .pick_file()
            },
            Message::FileSelected,
        ),
        Message::FileSelected(Some(path)) => {
            state.input_path = path.to_string_lossy().to_string();
            let _ = state.load_image_from_path(&path);
            Task::none()
        }
        Message::FileSelected(None) => Task::none(),
        Message::PathChanged(value) => {
            state.input_path = value;
            state.status_message = None;
            Task::none()
        }
        Message::SubmitPath => {
            let trimmed = state.input_path.trim();
            if trimmed.is_empty() {
                state.status_message = Some(String::from("Enter a path to load."));
                return Task::none();
            }

            let owned_path = PathBuf::from(trimmed);
            // Keep the trimmed version in the text box to avoid leading/trailing spaces.
            state.input_path = trimmed.to_string();

            let _ = state.load_image_from_path(&owned_path);
            Task::none()
        }
        Message::MouseEntered => {
            state.is_hovering = true;
            Task::none()
        }
        Message::MouseLeft => {
            state.is_hovering = false;
            Task::none()
        }
        Message::MouseMoved(point) => {
            let dx = point.x - state.last_mouse_x;
            let dy = point.y - state.last_mouse_y;
            let distance = (dx * dx + dy * dy).sqrt();
            if distance > 10.0 {
                state.last_mouse_x = point.x;
                state.last_mouse_y = point.y;
                if let Some(img) = &state.image_data {
                    let x = point.x.max(0.0) as u32;
                    let y = point.y.max(0.0) as u32;
                    let half = 25;
                    let crop_x = x.saturating_sub(half).min(img.width().saturating_sub(50));
                    let crop_y = y.saturating_sub(half).min(img.height().saturating_sub(50));
                    let cropped = img.view(crop_x, crop_y, 50.min(img.width() - crop_x), 50.min(img.height() - crop_y)).to_image();
                    let resized = image::imageops::resize(&cropped, 100, 100, image::imageops::FilterType::Nearest);
                    let dynamic = image::DynamicImage::ImageRgba8(resized);
                    let rgb_img = dynamic.to_rgb8();
                    let final_dynamic = image::DynamicImage::ImageRgb8(rgb_img);
                    let mut bytes = Vec::new();
                    final_dynamic.write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png).unwrap();
                    state.magnified_image = Some(Handle::from_bytes(bytes));
                }
            }
            Task::none()
        }
    }
}

#[allow(dead_code)]
fn view(state: &ImageViewer) -> Element<'_, Message> {
    let browse_button = button("Load image…").on_press(Message::OpenFileDialog);

    let path_input = text_input("Enter an image path", &state.input_path)
        .on_input(Message::PathChanged)
        .on_submit(Message::SubmitPath)
        .padding(10)
        .width(Length::Fill);

    let load_from_path_button = button("Display").on_press(Message::SubmitPath);

    let mut content = column![
        browse_button,
        row![path_input, load_from_path_button].spacing(10),
    ]
    .spacing(16)
    .padding(20);

    if let Some(path) = &state.loaded_path {
        content = content.push(text(format!("Current image: {}", path.display())));
    }

    if let Some(status) = &state.status_message {
        content = content.push(text(status));
    }

    if let Some(handle) = &state.loaded_image {
        let image_element = iced_image(handle.clone())
            .width(Length::Shrink)
            .height(Length::Shrink);
        let image_container = mouse_area(image_element)
            .on_enter(Message::MouseEntered)
            .on_exit(Message::MouseLeft)
            .on_move(Message::MouseMoved);

        if state.is_hovering {
            if let Some(mag_handle) = &state.magnified_image {
                let mag_image = iced_image(mag_handle.clone())
                    .width(Length::Fixed(100.0))
                    .height(Length::Fixed(100.0));
                let row_content = row![image_container, mag_image].spacing(10);
                content = content.push(row_content);
            } else {
                content = content.push(image_container);
            }
        } else {
            content = content.push(image_container);
        }
    }

    content.into()
}

#[allow(dead_code)]
pub fn my_counter() -> iced::Result {
    iced::application("Image Loader", update, view).run()
}
