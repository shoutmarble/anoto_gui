mod gui_app;
mod kornia;
mod test_image_gen;

fn main() -> iced::Result {
    println!("Anoto Dot Reader - Starting GUI...");

    // Generate test pattern if it doesn't exist
    let test_pattern_path = "test_pattern.png";
    if !std::path::Path::new(test_pattern_path).exists() {
        println!("Generating test pattern image...");
        if let Err(e) = test_image_gen::generate_test_image(test_pattern_path) {
            eprintln!("Warning: Failed to generate test pattern: {}", e);
        } else {
            println!("Test pattern generated successfully.");
        }
    }

    gui_app::iced_ui::run_iced_app()
}
