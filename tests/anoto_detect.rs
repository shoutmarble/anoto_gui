use anoto_dot_reader::kornia::anoto::{annotate_anoto_dots, AnotoConfig};
// image::DynamicImage is not required in this test; open returns DynamicImage.

#[test]
fn smoke_detect_assets() {
    let path = std::path::Path::new("src/kornia/assets/anoto_dots2.png");
    let img = image::open(path).expect("failed to load sample image");
    let det = annotate_anoto_dots(&img, &AnotoConfig::default()).expect("detection failed");
    let y = det.annotated.to_rgba8().height();
    assert!(y > 0, "Annotated returns an image with zero height");
}
