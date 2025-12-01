use std::{collections::VecDeque, f32::consts::PI};

use image::{DynamicImage, Rgba, RgbaImage};
use kornia::{
    image::{Image, ImageError, ImageSize},
    imgproc,
};

const MIN_COMPONENT_PIXELS: usize = 3;
const MAX_COMPONENT_PIXELS: usize = 250;
const CENTRALITY_THRESHOLD: f32 = 0.55;
const COLOR_BLACK: Rgba<u8> = Rgba([25, 25, 25, 255]);
const COLOR_RED: Rgba<u8> = Rgba([220, 60, 60, 255]);
const COLOR_BLUE: Rgba<u8> = Rgba([60, 110, 220, 255]);
const COLOR_MAGENTA: Rgba<u8> = Rgba([210, 70, 210, 255]);

#[derive(Debug)]
pub struct DotDetection {
    pub center: (f32, f32),
    pub radius: f32,
    pub color: Rgba<u8>,
}

#[derive(Debug, thiserror::Error)]
pub enum DetectionError {
    #[error("kornia image error: {0}")]
    Kornia(#[from] ImageError),
}

pub fn annotate_anoto_dots(source: &DynamicImage) -> Result<DynamicImage, DetectionError> {
    let rgb = source.to_rgb8();
    let (width, height) = rgb.dimensions();
    if width == 0 || height == 0 {
        return Ok(source.clone());
    }

    let raw_pixels = rgb.into_raw();
    let image = Image::<u8, 3>::new(
        ImageSize {
            width: width as usize,
            height: height as usize,
        },
        raw_pixels.clone(),
    )?;

    let mut gray = Image::<u8, 1>::from_size_val(image.size(), 0u8)?;
    imgproc::color::gray_from_rgb_u8(&image, &mut gray)?;

    let threshold = otsu_threshold(gray.as_slice());
    let mut binary = Image::<u8, 1>::from_size_val(gray.size(), 0u8)?;
    imgproc::threshold::threshold_binary(&gray, &mut binary, threshold, 255)?;

    let mut mask = binary.as_slice().to_vec();
    ensure_foreground_convention(&mut mask);

    let components = extract_components(
        &mask,
        gray.as_slice(),
        &raw_pixels,
        width as usize,
        height as usize,
    );
    let mut canvas: RgbaImage = source.to_rgba8();
    for dot in components.iter() {
        draw_ring(&mut canvas, dot.center, dot.radius, dot.color);
    }

    Ok(DynamicImage::ImageRgba8(canvas))
}

fn ensure_foreground_convention(mask: &mut [u8]) {
    let foreground = mask.iter().filter(|&&px| px != 0).count();
    if foreground * 2 < mask.len() {
        return;
    }
    for px in mask.iter_mut() {
        *px = if *px == 0 { 255 } else { 0 };
    }
}

fn extract_components(
    mask: &[u8],
    grayscale: &[u8],
    rgb: &[u8],
    width: usize,
    height: usize,
) -> Vec<DotDetection> {
    let mut visited = vec![false; mask.len()];
    let mut out = Vec::new();

    for start in 0..mask.len() {
        if mask[start] == 0 || visited[start] {
            continue;
        }

        let mut queue = VecDeque::new();
        queue.push_back(start);
        visited[start] = true;

        let mut sum_x = 0f32;
        let mut sum_y = 0f32;
        let mut weighted_x = 0f64;
        let mut weighted_y = 0f64;
        let mut weight_sum = 0f64;
        let mut count = 0usize;
        let mut sum_r = 0u32;
        let mut sum_g = 0u32;
        let mut sum_b = 0u32;

        while let Some(idx) = queue.pop_front() {
            let y = idx / width;
            let x = idx % width;
            sum_x += x as f32;
            sum_y += y as f32;
            count += 1;
            let rgb_idx = idx * 3;
            if rgb_idx + 2 < rgb.len() {
                sum_r += rgb[rgb_idx] as u32;
                sum_g += rgb[rgb_idx + 1] as u32;
                sum_b += rgb[rgb_idx + 2] as u32;
            }

            let intensity = if let Some(&value) = grayscale.get(idx) {
                // Darker pixels carry more weight so the centroid aligns with the inked dot.
                (255.0 - value as f64).max(1.0)
            } else {
                1.0
            };
            weighted_x += (x as f64) * intensity;
            weighted_y += (y as f64) * intensity;
            weight_sum += intensity;

            for (dx, dy) in [(-1isize, 0isize), (1, 0), (0, -1), (0, 1)] {
                let nx = x as isize + dx;
                let ny = y as isize + dy;
                if nx < 0 || ny < 0 {
                    continue;
                }
                let nxu = nx as usize;
                let nyu = ny as usize;
                if nxu >= width || nyu >= height {
                    continue;
                }
                let next_idx = nyu * width + nxu;
                if mask[next_idx] == 0 || visited[next_idx] {
                    continue;
                }
                visited[next_idx] = true;
                queue.push_back(next_idx);
            }
        }

        if !(MIN_COMPONENT_PIXELS..=MAX_COMPONENT_PIXELS).contains(&count) {
            continue;
        }

        let uniform_center = (sum_x / count as f32, sum_y / count as f32);
        let weighted_center = if weight_sum > 0.0 {
            (
                weighted_x as f32 / weight_sum as f32,
                weighted_y as f32 / weight_sum as f32,
            )
        } else {
            uniform_center
        };
        let center = blend_centers(uniform_center, weighted_center);
        let radius = ((count as f32) / PI).sqrt().max(1.5) * 1.35 + 1.5;
        let mean_r = sum_r as f32 / count as f32;
        let mean_g = sum_g as f32 / count as f32;
        let mean_b = sum_b as f32 / count as f32;
        let color = classify_color(mean_r, mean_g, mean_b);
        out.push(DotDetection {
            center,
            radius,
            color,
        });
    }

    out
}

fn classify_color(r: f32, g: f32, b: f32) -> Rgba<u8> {
    if r < 70.0 && g < 70.0 && b < 70.0 {
        COLOR_BLACK
    } else if r > 190.0 && b < 120.0 && r - g > 40.0 {
        COLOR_RED
    } else if b > 190.0 && r < 150.0 && b - g > 30.0 {
        COLOR_BLUE
    } else {
        COLOR_MAGENTA
    }
}

fn blend_centers(uniform: (f32, f32), weighted: (f32, f32)) -> (f32, f32) {
    let shift = ((weighted.0 - uniform.0).abs() + (weighted.1 - uniform.1).abs()) * 0.5;
    if shift > CENTRALITY_THRESHOLD {
        weighted
    } else {
        (
            (uniform.0 + weighted.0) * 0.5,
            (uniform.1 + weighted.1) * 0.5,
        )
    }
}

fn draw_ring(canvas: &mut RgbaImage, center: (f32, f32), radius: f32, color: Rgba<u8>) {
    let (cx, cy) = center;
    let cx_i = cx.round() as i32;
    let cy_i = cy.round() as i32;
    let radius = radius.max(2.0);
    let r_outer = radius.ceil() as i32;
    let r_inner = (radius - 1.0).max(0.0);
    let r_inner_sq = (r_inner * r_inner) as i32;
    let r_outer_sq = (radius * radius) as i32;
    let width = canvas.width() as i32;
    let height = canvas.height() as i32;

    for y in (cy_i - r_outer)..=(cy_i + r_outer) {
        if y < 0 || y >= height {
            continue;
        }
        for x in (cx_i - r_outer)..=(cx_i + r_outer) {
            if x < 0 || x >= width {
                continue;
            }
            let dx = x - cx_i;
            let dy = y - cy_i;
            let dist_sq = dx * dx + dy * dy;
            if dist_sq <= r_outer_sq && dist_sq >= r_inner_sq {
                canvas.put_pixel(x as u32, y as u32, color);
            }
        }
    }
}

fn otsu_threshold(pixels: &[u8]) -> u8 {
    let mut histogram = [0u32; 256];
    for &value in pixels {
        histogram[value as usize] += 1;
    }

    let total_pixels = pixels.len() as f64;
    let mut sum_total = 0f64;
    for (value, &count) in histogram.iter().enumerate() {
        sum_total += value as f64 * count as f64;
    }

    let mut sum_background = 0f64;
    let mut weight_background = 0f64;
    let mut max_variance = f64::MIN;
    let mut threshold = 0u8;

    for (value, &count) in histogram.iter().enumerate() {
        weight_background += count as f64;
        if weight_background == 0.0 {
            continue;
        }

        let weight_foreground = total_pixels - weight_background;
        if weight_foreground == 0.0 {
            break;
        }

        sum_background += value as f64 * count as f64;

        let mean_background = sum_background / weight_background;
        let mean_foreground = (sum_total - sum_background) / weight_foreground;
        let variance =
            weight_background * weight_foreground * (mean_background - mean_foreground).powi(2);

        if variance > max_variance {
            max_variance = variance;
            threshold = value as u8;
        }
    }

    threshold
}
