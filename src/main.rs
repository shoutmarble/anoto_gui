#![windows_subsystem = "windows"]

mod gui_app;

fn main() -> iced::Result {
    println!("Anoto Dot Reader - Starting GUI...");

    gui_app::iced_ui::run_iced_app()
}
