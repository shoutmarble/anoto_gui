use anoto_dot_reader::kornia::anoto::{annotate_anoto_dots, detect_components_from_image, AnotoConfig};
use image::DynamicImage;
use std::fs;

#[test]
fn debug_detect_assets2() {
    let path = std::path::Path::new("src/kornia/assets/anoto_dots2.png");
    let img = image::open(path).expect("failed to load sample image");

    // programmatic detection - get components
    let comps = detect_components_from_image(&img, &AnotoConfig::default()).expect("detection failed");
    eprintln!("components: {}", comps.len());
    for (i, d) in comps.iter().enumerate().take(40) {
        eprintln!("comp {}: center=({:.1}, {:.1}), r={:.1}, color={:?}", i, d.center.0, d.center.1, d.radius, d.type_color);
    }
    assert!(comps.len() > 0, "Should detect some components");

    // annotated image saved for review
    let det = annotate_anoto_dots(&img, &AnotoConfig::default()).expect("annotate failed");
    if let image::DynamicImage::ImageRgba8(img) = det.annotated {
        fs::create_dir_all("output").ok();
        img.save("output/anoto_dots2_annotated.png").expect("failed to write annotated image");
    }

        // Note: using only anoto_dots2.png in current test set
}
