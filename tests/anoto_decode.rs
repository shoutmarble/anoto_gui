use anoto_dot_reader::kornia::anoto::{detect_components_from_image, detect_grid, AnotoConfig, annotate_anoto_dots};
use image::DynamicImage;
use std::fs;

#[test]
fn smoke_decode_grid() {
    let path = std::path::Path::new("src/kornia/assets/anoto_dots2.png");
    let img = image::open(path).expect("failed to load sample image");
    let config = AnotoConfig::default();
    let comps = detect_components_from_image(&img, &config).expect("detect failed");
    assert!(!comps.is_empty(), "no components detected");
    let grid = detect_grid(&comps, &config).expect("grid detection failed");
    eprintln!("grid rows {} cols {}\n{}", grid.0, grid.1, grid.2);

    // Save annotated image for debugging
    let det = annotate_anoto_dots(&img, &config).expect("annotation failed");
    if let image::DynamicImage::ImageRgba8(img) = det.annotated {
        fs::create_dir_all("output").ok();
        img.save("output/anoto_dots2_annotated.png").expect("failed to write annotated image");
    }

    assert!(grid.0 > 0 && grid.1 > 0, "grid dims invalid");
}
