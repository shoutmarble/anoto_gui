use image::{Rgba, RgbaImage};

/// Generates a test pattern image for testing the Anoto reader.
/// 
/// Creates an A4-sized image at 72 DPI with:
/// - White background
/// - Orange border around the edges
/// - Large red circle in the center
/// 
/// # Arguments
/// 
/// * `path` - The file path where the image should be saved
/// 
/// # Returns
/// 
/// Returns `Ok(())` if successful, or an error if the save operation fails.
pub fn generate_test_image(path: &str) -> Result<(), String> {
    // A4 at 72 DPI
    // Width = 8.27 * 72 = 595.44 -> 595
    // Height = 11.69 * 72 = 841.68 -> 842
    let width = 595;
    let height = 842;

    let mut img = RgbaImage::new(width, height);

    let orange = Rgba([255, 165, 0, 255]);
    let red = Rgba([255, 0, 0, 255]);
    let white = Rgba([255, 255, 255, 255]);

    // Fill white
    for pixel in img.pixels_mut() {
        *pixel = white;
    }

    // Orange border (5 pixels)
    let border = 5;
    for x in 0..width {
        for y in 0..height {
            if x < border || x >= width - border || y < border || y >= height - border {
                img.put_pixel(x, y, orange);
            }
        }
    }

    // Large Red Circle touching both sides
    let cx = width as f32 / 2.0;
    let cy = height as f32 / 2.0;
    let radius = width as f32 / 2.0;

    for x in 0..width {
        for y in 0..height {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            if dx * dx + dy * dy <= radius * radius {
                let is_border = x < border || x >= width - border || y < border || y >= height - border;
                if !is_border {
                    img.put_pixel(x, y, red);
                }
            }
        }
    }

    img.save(path).map_err(|e| format!("Failed to save image: {}", e))?;
    Ok(())
}
