
// Quantize a sample Anoto image into 5 colors and save as assets/test_pallete.png.
#[cfg(test)]
pub fn quantize_colors_in_image() -> bool {
    use color_counter::color_processing::Color;
    use color_counter::space::Space;
    use image::{DynamicImage, Rgba, RgbaImage};

    // Load source image.
    let img: DynamicImage = match image::open("assets/test_output/test_images/anoto_0.png") {
        Ok(img) => img,
        Err(e) => {
            eprintln!("Failed to load image: {}", e);
            return false;
        }
    };

    // Build LAB space grouped by ~10% region size.
    let space = Space::from_file("assets/test_output/test_images/anoto_0.png", 0.10);

    // Gather regions with counts and average color.
    let mut regions: Vec<(usize, Color)> = Vec::new();
    for region in space.regions_iter() {
        if region.is_empty() {
            continue;
        }

        let count: usize = region.iter().map(|(_, c)| *c).sum();
        let avg = region.average_color();
        regions.push((count, avg));
    }

    // Sort by frequency desc.
    regions.sort_by(|a, b| b.0.cmp(&a.0));

    if regions.len() < 5 {
        eprintln!("Not enough color regions to determine top 5.");
        return false;
    }

    // Palette: top 5 average colors.
    let palette: Vec<Color> = regions.iter().take(5).map(|(_, avg)| avg.clone()).collect();

    // Quantize the image to the 5-color palette and save to assets/test_pallete.png.
    let mut quantized: RgbaImage = RgbaImage::new(img.width(), img.height());
    for (x, y, px) in img.to_rgba8().enumerate_pixels() {
        let src = Color::new_rgba(px[0], px[1], px[2], px[3]);
        let src_lab = src.get_laba();
        let mut best_idx = 0;
        let mut best_dist = f64::MAX;
        for (i, pal) in palette.iter().enumerate() {
            let plab = pal.get_laba();
            let dist = (src_lab.0 - plab.0).powi(2) + (src_lab.1 - plab.1).powi(2) + (src_lab.2 - plab.2).powi(2);
            if dist < best_dist {
                best_dist = dist;
                best_idx = i;
            }
        }
        let pal = &palette[best_idx];
        quantized.put_pixel(x, y, Rgba([pal.red, pal.green, pal.blue, pal.alpha]));
    }

        if let Err(e) = quantized.save("assets/test_pallete.png") {
        eprintln!("Failed to save quantized image: {}", e);
        return false;
    }

        println!("Quantized 5-color image written to assets/test_pallete.png");

    true
}

    // Quantize using libimagequant to 5 colors and save to assets/anoto_quant.png.
    #[allow(dead_code)]
    pub fn quantize_with_imagequant() -> bool {
        use imagequant::{new, Attributes, RGBA};
        use image::{DynamicImage, Rgba, RgbaImage};

        let img: DynamicImage = match image::open("assets/test_output/test_images/anoto_0.png") {
            Ok(img) => img,
            Err(e) => {
                eprintln!("Failed to load image: {e}");
                return false;
            }
        };

        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();

        // Prepare bitmap in libimagequant's RGBA format.
        let bitmap: Vec<RGBA> = rgba
            .pixels()
            .map(|p| RGBA::new(p[0], p[1], p[2], p[3]))
            .collect();

        // Configure libimagequant for a 5-color palette.
        let mut attr: Attributes = new();
        if let Err(e) = attr.set_max_colors(5) {
            eprintln!("Failed to set max colors: {e}");
            return false;
        }
        // Slightly slower but better quality than the fastest setting.
        let _ = attr.set_speed(3);

        let mut liq_image = match attr.new_image(&bitmap[..], width as usize, height as usize, 0.0) {
            Ok(img) => img,
            Err(e) => {
                eprintln!("Failed to create liq image: {e}");
                return false;
            }
        };

        let mut result = match attr.quantize(&mut liq_image) {
            Ok(res) => res,
            Err(e) => {
                eprintln!("Quantization failed: {e}");
                return false;
            }
        };

        // Keep default dithering; could be tuned via set_dithering_level.
        let (palette, indexed) = match result.remapped(&mut liq_image) {
            Ok(pair) => pair,
            Err(e) => {
                eprintln!("Remap failed: {e}");
                return false;
            }
        };

        // Enforce exactly 5 colors in the output by collapsing to the top-5 most frequent palette entries.
        let mut counts: Vec<(usize, u32)> = vec![(0, 0); palette.len()];
        for &idx in indexed.iter() {
            if let Some(entry) = counts.get_mut(idx as usize) {
                entry.0 = idx as usize;
                entry.1 = entry.1.saturating_add(1);
            }
        }
        counts.sort_by(|a, b| b.1.cmp(&a.1));
        counts.truncate(5);
        let keep_indices: Vec<usize> = counts.iter().map(|(i, _)| *i).collect();

        // Build a remap for all palette indices to the nearest kept color (fallback if there were more than 5).
        let mut remap_table: Vec<usize> = vec![0; palette.len()];
        for (i, color) in palette.iter().enumerate() {
            if keep_indices.contains(&i) {
                remap_table[i] = i;
                continue;
            }
            let mut best = keep_indices[0];
            let mut best_d = u32::MAX;
            for &k in keep_indices.iter() {
                let kc = &palette[k];
                let dr = color.r as i32 - kc.r as i32;
                let dg = color.g as i32 - kc.g as i32;
                let db = color.b as i32 - kc.b as i32;
                let da = color.a as i32 - kc.a as i32;
                let dist = (dr * dr + dg * dg + db * db + da * da) as u32;
                if dist < best_d {
                    best_d = dist;
                    best = k;
                }
            }
            remap_table[i] = best;
        }

        let mut out: RgbaImage = RgbaImage::new(width, height);
        for (i, &idx) in indexed.iter().enumerate() {
            let mapped_idx = *remap_table.get(idx as usize).unwrap_or(&0);
            let pal_px = match palette.get(mapped_idx) {
                Some(p) => p,
                None => {
                    eprintln!("Palette index out of range after remap: {mapped_idx}");
                    return false;
                }
            };
            let x = (i as u32) % width;
            let y = (i as u32) / width;
            out.put_pixel(x, y, Rgba([pal_px.r, pal_px.g, pal_px.b, pal_px.a]));
        }

        if let Err(e) = out.save("assets/anoto_quant.png") {
            eprintln!("Failed to save quantized image: {e}");
            return false;
        }

        println!("Quantized (libimagequant) 5-color image written to assets/anoto_quant.png");
        true
    }

// Public wrapper used by tests; keeps the original helper alive for manual runs too.
#[cfg(test)]
pub fn color_area() -> bool {
    quantize_colors_in_image()
}

#[cfg(test)]
pub fn display_colors() -> bool {
    use color_counter::color_processing::Color;
    use color_counter::space::Space;
    use image::DynamicImage;

    // Load quantized image.
    let _img: DynamicImage = match image::open("assets/test_pallete.png") {
        Ok(img) => img,
        Err(e) => {
            eprintln!("Failed to load image: {}", e);
            return false;
        }
    };

    // Build LAB space grouped by ~10% region size.
    let space = Space::from_file("assets/test_pallete.png", 0.10);

    // Count regions and report distinct colors.
    let mut regions: Vec<(usize, Color)> = Vec::new();
    for region in space.regions_iter() {
        if region.is_empty() {
            continue;
        }

        let count: usize = region.iter().map(|(_, c)| *c).sum();
        let avg = region.average_color();
        regions.push((count, avg));
    }

    regions.sort_by(|a, b| b.0.cmp(&a.0));

    println!("Found {} distinct colors (by region):", regions.len());
    for (idx, (cnt, col)) in regions.iter().enumerate() {
        println!(
            "{}: RGBA ({}, {}, {}, {}), pixels: {}",
            idx + 1,
            col.red,
            col.green,
            col.blue,
            col.alpha,
            cnt
        );
    }

    true
}

// draw a horizontal line through anoto dots that are the same color
fn draw_horizontal_line_on(input_path: &str, output_path: &str) -> bool {
    use super::anoto::{detect_components_from_image, AnotoConfig, DotDetection};
    use image::{DynamicImage, Rgba, RgbaImage};
    use std::collections::HashSet;

    // Load the source image we want to annotate.
    let img: DynamicImage = match image::open(input_path) {
        Ok(img) => img,
        Err(e) => {
            eprintln!("Failed to load image {input_path}: {e}");
            return false;
        }
    };

    // Detect all dots and their classified colors.
    let config = AnotoConfig::default();
    let dots: Vec<DotDetection> = match detect_components_from_image(&img, &config) {
        Ok(detections) => detections,
        Err(e) => {
            eprintln!("Dot detection failed: {e}");
            return false;
        }
    };

    if dots.is_empty() {
        eprintln!("No dots found; skipping line drawing.");
        return false;
    }

    // Identify which rows should receive a line: rows where a dot shares its color with
    // at least one other dot whose vertical span intersects this dot's center line.
    let allowed_colors: HashSet<[u8; 4]> = HashSet::from([
        config.color_green.0,
        config.color_orange.0,
    ]);

    let mut rows_to_draw: HashSet<(i32, [u8; 4])> = HashSet::new();
    for (idx, dot) in dots.iter().enumerate() {
        if !allowed_colors.contains(&dot.type_color.0) {
            continue;
        }

        let line_y = dot.center.1.round() as i32;
        let intersects_same_color = dots.iter().enumerate().any(|(other_idx, other)| {
            if idx == other_idx {
                return false;
            }
            if other.type_color != dot.type_color {
                return false;
            }
            let vert_dist = (other.center.1 - dot.center.1).abs();
            // Treat overlap as sharing the line when the centers are within the blended radius.
            let tolerance = 0.5 * (other.radius + dot.radius) + 1.0; // +1 for the 2px stroke thickness
            vert_dist <= tolerance
        });

        if intersects_same_color {
            rows_to_draw.insert((line_y, dot.type_color.0));
        }
    }

    if rows_to_draw.is_empty() {
        eprintln!("No rows met the intersection rule; nothing drawn.");
        return false;
    }

    let mut canvas: RgbaImage = img.to_rgba8();
    let height = canvas.height() as i32;
    let width = canvas.width();

    for (y, color_bytes) in rows_to_draw {
        let color = Rgba(color_bytes);
        // 2px thick: draw at y and y+1.
        for dy in 0..2 {
            let yy = y + dy;
            if yy < 0 || yy >= height {
                continue;
            }
            for x in 0..width {
                // Only draw through dots of the same color; skip over other colors.
                let mut blocked = false;
                for dot in dots.iter() {
                    let dx = x as f32 - dot.center.0;
                    let dy_pix = yy as f32 - dot.center.1;
                    let dist = (dx * dx + dy_pix * dy_pix).sqrt();
                    if dist <= dot.radius {
                        if dot.type_color != color {
                            blocked = true;
                        }
                        break;
                    }
                }
                if blocked {
                    continue;
                }
                canvas.put_pixel(x, yy as u32, color);
            }
        }
    }

    if let Err(e) = canvas.save(output_path) {
        eprintln!("Failed to save horizontal lines image: {e}");
        return false;
    }

    println!("Horizontal lines written to {output_path}");
    true
}

#[allow(dead_code)]
pub fn draw_horizontal_line() -> bool {
    draw_horizontal_line_on(
        "assets/test_output/test_images/anoto_0.png",
        "assets/horizontal_lines.png",
    )
}

#[allow(dead_code)]
pub fn draw_horizontal_line_on_anoto_quant() -> bool {
    draw_horizontal_line_on("assets/anoto_quant.png", "assets/horizontal_lines_quant.png")
}

