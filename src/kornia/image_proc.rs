use argh::FromArgs;
use std::{fs, path::PathBuf};
// ...existing code removed for fresh implementation...
use kornia::io::functional as F;
use kornia::{
    image::{Image, allocator::CpuAllocator},
    imgproc,
};

type CpuImage<T, const C: usize> = Image<T, C, CpuAllocator>;

#[derive(FromArgs)]
#[allow(dead_code)]
/// Image process an image and log it to Rerun
struct Args {
    /// path to an input image
    #[argh(option, short = 'i')]
    image_path: PathBuf,
}

#[allow(dead_code)]
pub fn image_proc() -> Result<(), Box<dyn std::error::Error>> {
    // read the image from the assets (hard-coded path)
    let image: CpuImage<u8, 3> = F::read_image_any_rgb8("src/kornia/assets/anoto_dots2.png")?;

    // ensure output directory exists
    let out_dir = PathBuf::from("output");
    fs::create_dir_all(&out_dir)?;

    // convert the image to grayscale
    let mut gray: CpuImage<u8, 1> = Image::from_size_val(image.size(), 0, CpuAllocator)?;
    imgproc::color::gray_from_rgb_u8(&image, &mut gray)?;

    // binarize the image (simple fixed-threshold)
    let mut binary: CpuImage<u8, 1> =
        Image::from_size_val(image.size(), 0, CpuAllocator)?;
    imgproc::threshold::threshold_binary(&gray, &mut binary, 128, 255)?;

    // save the binarized image to output/binary.png
    let out_path_bin = out_dir.join("binary.png");
    let width_bin = binary.size().width as u32;
    let height_bin = binary.size().height as u32;
    image::save_buffer(
        out_path_bin.as_path(),
        binary.as_slice(),
        width_bin,
        height_bin,
        image::ColorType::L8,
    )?;
    println!("wrote {}", out_path_bin.display());

    // save the grayscale image to output/greyscale.png
    let out_path_gray = out_dir.join("greyscale.png");
    let width_gray = gray.size().width as u32;
    let height_gray = gray.size().height as u32;
    image::save_buffer(
        out_path_gray.as_path(),
        gray.as_slice(),
        width_gray,
        height_gray,
        image::ColorType::L8,
    )?;
    println!("wrote {}", out_path_gray.display());

    // compute a 1-pixel edge map around each binary group
    let width_usize = binary.size().width as usize;
    let height_usize = binary.size().height as usize;
    // copy binary into an owned buffer so we can invert if needed
    let mut bin_data = binary.as_slice().to_vec();

    // if foreground occupies >50% of pixels, assume it's inverted (background is foreground)
    let total = bin_data.len();
    let fg_count = bin_data.iter().filter(|&&v| v != 0).count();
    eprintln!("binary foreground pixels: {} / {}", fg_count, total);
    if fg_count * 2 > total {
        eprintln!("inverting binary because foreground > 50% of image");
        for v in bin_data.iter_mut() {
            *v = if *v == 0 { 255u8 } else { 0u8 };
        }
        // save the inverted binary for inspection
        let out_path_bin_inv = out_dir.join("binary_inverted.png");
        image::save_buffer(
            out_path_bin_inv.as_path(),
            &bin_data,
            width_usize as u32,
            height_usize as u32,
            image::ColorType::L8,
        )?;
        println!("wrote {}", out_path_bin_inv.display());
    }

    let bin_slice = bin_data.as_slice();

    let mut edges_buf = vec![0u8; width_usize * height_usize];
    for y in 0..height_usize {
        for x in 0..width_usize {
            let idx = y * width_usize + x;
            if bin_slice[idx] == 0 {
                continue;
            }

            // check 4-neighbors for background -> this pixel is on the boundary
            let mut is_edge = false;
            if x > 0 && bin_slice[idx - 1] == 0 {
                is_edge = true;
            }
            if x + 1 < width_usize && bin_slice[idx + 1] == 0 {
                is_edge = true;
            }
            if y > 0 && bin_slice[idx - width_usize] == 0 {
                is_edge = true;
            }
            if y + 1 < height_usize && bin_slice[idx + width_usize] == 0 {
                is_edge = true;
            }

            if is_edge {
                edges_buf[idx] = 255u8;
            }
        }
    }

    // save edge map
    let out_path_edges = out_dir.join("edges.png");
    image::save_buffer(
        out_path_edges.as_path(),
        &edges_buf,
        width_usize as u32,
        height_usize as u32,
        image::ColorType::L8,
    )?;
    println!("wrote {}", out_path_edges.display());

    // create an RGB overlay: red edges on top of greyscale background
    let gray_slice = gray.as_slice();
    let mut overlay = vec![0u8; width_usize * height_usize * 3];
    for i in 0..(width_usize * height_usize) {
        let g = gray_slice[i];
        if edges_buf[i] != 0 {
            overlay[3 * i] = 255u8; // R
            overlay[3 * i + 1] = 0u8; // G
            overlay[3 * i + 2] = 0u8; // B
        } else {
            overlay[3 * i] = g;
            overlay[3 * i + 1] = g;
            overlay[3 * i + 2] = g;
        }
    }

    // connected-component labeling (4-neighbour) to find each binary group's centroid
    let mut labels = vec![0usize; width_usize * height_usize];
    let mut comps: Vec<(
        usize, /*sum_x*/
        usize, /*sum_y*/
        usize, /*count*/
    )> = Vec::new();
    let mut current_label = 1usize;
    for y in 0..height_usize {
        for x in 0..width_usize {
            let idx = y * width_usize + x;
            if bin_slice[idx] == 0 || labels[idx] != 0 {
                continue;
            }

            // BFS flood fill
            let mut stack = vec![idx];
            labels[idx] = current_label;
            let mut sum_x: usize = 0;
            let mut sum_y: usize = 0;
            let mut count: usize = 0;
            while let Some(p) = stack.pop() {
                let py = p / width_usize;
                let px = p % width_usize;
                sum_x += px;
                sum_y += py;
                count += 1;

                // neighbors
                if px > 0 {
                    let n = p - 1;
                    if bin_slice[n] != 0 && labels[n] == 0 {
                        labels[n] = current_label;
                        stack.push(n);
                    }
                }
                if px + 1 < width_usize {
                    let n = p + 1;
                    if bin_slice[n] != 0 && labels[n] == 0 {
                        labels[n] = current_label;
                        stack.push(n);
                    }
                }
                if py > 0 {
                    let n = p - width_usize;
                    if bin_slice[n] != 0 && labels[n] == 0 {
                        labels[n] = current_label;
                        stack.push(n);
                    }
                }
                if py + 1 < height_usize {
                    let n = p + width_usize;
                    if bin_slice[n] != 0 && labels[n] == 0 {
                        labels[n] = current_label;
                        stack.push(n);
                    }
                }
            }

            comps.push((sum_x, sum_y, count));
            current_label += 1;
        }
    }

    // debug: report number of components and first few centroids
    eprintln!("found {} components", comps.len());
    for (i, (sx, sy, cnt)) in comps.iter().enumerate().take(20) {
        if *cnt == 0 {
            continue;
        }
        let cx = (*sx as f32 / *cnt as f32).round() as i32;
        let cy = (*sy as f32 / *cnt as f32).round() as i32;
        eprintln!("comp {}: count={} centroid=({}, {})", i, cnt, cx, cy);
    }

    // draw small filled red dots at each component, anchored to a pixel inside the component
    let dot_radius: i32 = 5; // increased for visibility
    use std::collections::VecDeque;

    // compute an anchor pixel for each component (nearest pixel inside the component to the centroid)
    let mut anchors: Vec<Option<(usize, usize)>> = vec![None; comps.len()];
    for (label_idx, (sum_x, sum_y, count)) in comps.iter().enumerate() {
        if *count == 0 {
            continue;
        }
        let mut cx = (*sum_x as f32 / *count as f32).round() as i32;
        let mut cy = (*sum_y as f32 / *count as f32).round() as i32;
        if cx < 0 {
            cx = 0;
        }
        if cy < 0 {
            cy = 0;
        }
        if (cx as usize) >= width_usize {
            cx = (width_usize - 1) as i32;
        }
        if (cy as usize) >= height_usize {
            cy = (height_usize - 1) as i32;
        }

        let label_id = label_idx + 1;
        let start_idx = (cy as usize) * width_usize + (cx as usize);
        if labels[start_idx] == label_id {
            anchors[label_idx] = Some((cx as usize, cy as usize));
            continue;
        }

        // BFS to find nearest pixel labeled with label_id
        let mut visited = vec![false; width_usize * height_usize];
        let mut dq: VecDeque<usize> = VecDeque::new();
        visited[start_idx] = true;
        dq.push_back(start_idx);
        let mut found = false;
        while let Some(p) = dq.pop_front() {
            if labels[p] == label_id {
                let py = p / width_usize;
                let px = p % width_usize;
                anchors[label_idx] = Some((px, py));
                found = true;
                break;
            }
            let py = p / width_usize;
            let px = p % width_usize;
            if px > 0 {
                let n = p - 1;
                if !visited[n] {
                    visited[n] = true;
                    dq.push_back(n);
                }
            }
            if px + 1 < width_usize {
                let n = p + 1;
                if !visited[n] {
                    visited[n] = true;
                    dq.push_back(n);
                }
            }
            if py > 0 {
                let n = p - width_usize;
                if !visited[n] {
                    visited[n] = true;
                    dq.push_back(n);
                }
            }
            if py + 1 < height_usize {
                let n = p + width_usize;
                if !visited[n] {
                    visited[n] = true;
                    dq.push_back(n);
                }
            }
        }
        if !found {
            anchors[label_idx] = None;
        }
    }

    // draw dots using anchors
    for anchor in anchors.iter() {
        let (cx_u, cy_u) = match anchor {
            Some(c) => *c,
            None => continue,
        };
        let cx = cx_u as i32;
        let cy = cy_u as i32;
        for dy in -dot_radius..=dot_radius {
            for dx in -dot_radius..=dot_radius {
                let xx = cx + dx;
                let yy = cy + dy;
                if xx < 0 || yy < 0 {
                    continue;
                }
                let dx2 = dx * dx;
                let dy2 = dy * dy;
                if dx2 + dy2 > dot_radius * dot_radius {
                    continue;
                }
                let xui = xx as usize;
                let yui = yy as usize;
                if xui >= width_usize || yui >= height_usize {
                    continue;
                }
                let i = yui * width_usize + xui;
                overlay[3 * i] = 255u8;
                overlay[3 * i + 1] = 0u8;
                overlay[3 * i + 2] = 0u8;
            }
        }
    }

    // create a debug image that draws each centroid in a distinct color (to verify count)
    let mut centroids_debug = vec![0u8; width_usize * height_usize * 3];
    for (label_idx, (sum_x, sum_y, count)) in comps.iter().enumerate() {
        if *count == 0 {
            continue;
        }
        let cx = (*sum_x as f32 / *count as f32).round() as i32;
        let cy = (*sum_y as f32 / *count as f32).round() as i32;
        if cx < 0 || cy < 0 {
            continue;
        }
        let xui = cx as usize;
        let yui = cy as usize;
        if xui >= width_usize || yui >= height_usize {
            continue;
        }
        let i = yui * width_usize + xui;
        // pick a color from a simple hash
        let idx = label_idx as u32;
        let r = ((idx.wrapping_mul(1664525).wrapping_add(1013904223)) & 0xFF) as u8;
        let g = ((idx.wrapping_mul(22695477).wrapping_add(1)) & 0xFF) as u8;
        let b = ((idx.wrapping_mul(1103515245).wrapping_add(12345)) & 0xFF) as u8;
        centroids_debug[3 * i] = r;
        centroids_debug[3 * i + 1] = g;
        centroids_debug[3 * i + 2] = b;
    }
    let out_path_centroids = out_dir.join("centroids_debug.png");
    image::save_buffer(
        out_path_centroids.as_path(),
        &centroids_debug,
        width_usize as u32,
        height_usize as u32,
        image::ColorType::Rgb8,
    )?;
    println!("wrote {}", out_path_centroids.display());

    let out_path_overlay = out_dir.join("overlay.png");
    image::save_buffer(
        out_path_overlay.as_path(),
        &overlay,
        width_usize as u32,
        height_usize as u32,
        image::ColorType::Rgb8,
    )?;
    println!("wrote {}", out_path_overlay.display());

    // Also save a RGB version of the binary image with the red dots drawn inside each component
    let mut bin_rgb = vec![0u8; width_usize * height_usize * 3];
    for i in 0..(width_usize * height_usize) {
        if bin_slice[i] != 0 {
            bin_rgb[3 * i] = 255u8;
            bin_rgb[3 * i + 1] = 255u8;
            bin_rgb[3 * i + 2] = 255u8;
        } else {
            bin_rgb[3 * i] = 0u8;
            bin_rgb[3 * i + 1] = 0u8;
            bin_rgb[3 * i + 2] = 0u8;
        }
    }

    // draw the same red dots into bin_rgb
    for (sum_x, sum_y, count) in comps.iter() {
        if *count == 0 {
            continue;
        }
        let cx = (*sum_x as f32 / *count as f32).round() as i32;
        let cy = (*sum_y as f32 / *count as f32).round() as i32;
        for dy in -dot_radius..=dot_radius {
            for dx in -dot_radius..=dot_radius {
                let xx = cx + dx;
                let yy = cy + dy;
                if xx < 0 || yy < 0 {
                    continue;
                }
                let dx2 = dx * dx;
                let dy2 = dy * dy;
                if dx2 + dy2 > dot_radius * dot_radius {
                    continue;
                }
                let xui = xx as usize;
                let yui = yy as usize;
                if xui >= width_usize || yui >= height_usize {
                    continue;
                }
                let i = yui * width_usize + xui;
                bin_rgb[3 * i] = 255u8;
                bin_rgb[3 * i + 1] = 0u8;
                bin_rgb[3 * i + 2] = 0u8;
            }
        }
    }

    let out_path_bin_dots = out_dir.join("binary_with_dots.png");
    image::save_buffer(
        out_path_bin_dots.as_path(),
        &bin_rgb,
        width_usize as u32,
        height_usize as u32,
        image::ColorType::Rgb8,
    )?;
    println!("wrote {}", out_path_bin_dots.display());

    Ok(())
}
