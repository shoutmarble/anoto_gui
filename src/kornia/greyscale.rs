use kornia::{image::{Image, ImageSize}, imgproc};
use kornia::io::functional as F;

pub fn show_greyscale() -> Result<(), Box<dyn std::error::Error>> {
    // read the image
    // let image: Image<u8, 3, _> = F::read_image_any_rgb8("tests/data/dog.jpeg")?;
    let image: Image<u8, 3> = F::read_image_any_rgb8("src/kornia/assets/dots.png")?;

    let image_viz = image.clone();

    let image_f32  = image.cast_and_scale::<f32>(1.0 / 255.0)?;

    // convert the image to grayscale
    let mut gray = Image::from_size_val(image_f32.size(), 0.0)?;
    imgproc::color::gray_from_rgb(&image_f32, &mut gray)?;

    // resize the image
    let new_size = ImageSize {
        width: 128,
        height: 128,
    };

    let mut gray_resized = Image::from_size_val(new_size, 0.0)?;
    imgproc::resize::resize_native(
        &gray, &mut gray_resized,
        imgproc::interpolation::InterpolationMode::Bilinear,
    )?;

    println!("gray_resize: {:?}", gray_resized.size());

    // create a Rerun recording stream
    let rec = rerun::RecordingStreamBuilder::new("Kornia App").spawn()?;

    rec.log(
        "image",
        &rerun::Image::from_elements(
            image_viz.as_slice(),
            image_viz.size().into(),
            rerun::ColorModel::RGB,
        ),
    )?;

    rec.log(
        "gray",
        &rerun::Image::from_elements(gray.as_slice(), gray.size().into(), rerun::ColorModel::L),
    )?;

    rec.log(
        "gray_resize",
        &rerun::Image::from_elements(
            gray_resized.as_slice(),
            gray_resized.size().into(),
            rerun::ColorModel::L,
        ),
    )?;

    Ok(())
}