mod kornia;
mod bevy;
mod bevy_gui;
mod bevy_tut;
mod gui_app;

fn main() {
    println!("dots!");
    // Call the PDF generator
    // pdf::gen_anoto_pdf();
    // kornia::image_proc().unwrap();
    // ice::editor::my_counter().unwrap();
    // bevy::bevy_hello(); // Old GUI
    // bevy_gui::run_gui_window(); // New resizable GUI window

    gui_app::run_gui_app();
}
