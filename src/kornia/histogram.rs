use kornia::image::{Image, allocator::CpuAllocator};
use kornia::io::functional as F;

#[allow(dead_code)]
pub fn show_histogram() -> Result<(), Box<dyn std::error::Error>> {
    // read the image
    let image: Image<u8, 3, CpuAllocator> = F::read_image_any_rgb8("src/kornia/assets/dots.png")?;

    println!("Hello, world! 🦀");
    println!("Loaded Image size: {:?}", image.size());
    println!("\nGoodbyte!");

    Ok(())
}
