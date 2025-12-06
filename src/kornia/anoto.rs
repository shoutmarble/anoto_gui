#![allow(dead_code)]

use std::{collections::VecDeque, f32::consts::PI};

use image::{DynamicImage, Rgba, RgbaImage};
use kornia::{
    image::{Image, ImageError, ImageSize, allocator::CpuAllocator},
    imgproc,
};

type CpuImage<T, const C: usize> = Image<T, C, CpuAllocator>;

const MIN_COMPONENT_PIXELS: usize = 3;
const MAX_COMPONENT_PIXELS: usize = 250;
const CENTRALITY_THRESHOLD: f32 = 0.55;
const COLOR_BLACK: Rgba<u8> = Rgba([25, 25, 25, 255]);
const COLOR_RED: Rgba<u8> = Rgba([220, 60, 60, 255]);
const COLOR_BLUE: Rgba<u8> = Rgba([60, 110, 220, 255]);
const COLOR_MAGENTA: Rgba<u8> = Rgba([210, 70, 210, 255]);

/// Configuration parameters for Anoto dot detection.
///
/// Controls detection thresholds, component size filters, and color classification
/// for identifying and categorizing dots in Anoto dot paper images.
#[derive(Debug, Clone)]
pub struct AnotoConfig {
    pub min_component_pixels: usize,
    pub max_component_pixels: usize,
    pub centrality_threshold: f32,
    pub color_black: Rgba<u8>,
    pub color_red: Rgba<u8>,
    pub color_blue: Rgba<u8>,
    pub color_magenta: Rgba<u8>,
}

impl Default for AnotoConfig {
    fn default() -> Self {
        Self {
            min_component_pixels: MIN_COMPONENT_PIXELS,
            max_component_pixels: MAX_COMPONENT_PIXELS,
            centrality_threshold: CENTRALITY_THRESHOLD,
            color_black: COLOR_BLACK,
            color_red: COLOR_RED,
            color_blue: COLOR_BLUE,
            color_magenta: COLOR_MAGENTA,
        }
    }
}

/// Represents a detected dot in an Anoto pattern.
///
/// Contains the dot's center coordinates, radius, and classified color.
#[derive(Debug)]
pub struct DotDetection {
    pub center: (f32, f32),
    pub radius: f32,
    pub color: Rgba<u8>,
}

/// Errors that can occur during Anoto dot detection.
#[derive(Debug, thiserror::Error)]
pub enum DetectionError {
    #[error("kornia image error: {0}")]
    Kornia(#[from] ImageError),
}

/// Detects and annotates Anoto dots in an image.
///
/// This function processes an image to identify Anoto dot patterns, classifies them
/// by color, and returns an annotated version with visual overlays plus a grid
/// representation of the detected pattern.
///
/// # Arguments
///
/// * `source` - The input image to process
/// * `config` - Detection configuration parameters
///
/// # Returns
///
/// Returns a tuple containing:
/// * The annotated image with detected dots highlighted
/// * A string representation of the arrow grid pattern
///
/// # Errors
///
/// Returns `DetectionError` if image processing fails.
pub fn annotate_anoto_dots(
    source: &DynamicImage,
    config: &AnotoConfig,
) -> Result<(DynamicImage, String), DetectionError> {
    let rgb = source.to_rgb8();
    let (width, height) = rgb.dimensions();
    if width == 0 || height == 0 {
        return Ok((source.clone(), String::new()));
    }

    let raw_pixels = rgb.into_raw();
    let image = CpuImage::<u8, 3>::new(
        ImageSize {
            width: width as usize,
            height: height as usize,
        },
        raw_pixels.clone(),
        CpuAllocator,
    )?;

    let mut gray = CpuImage::<u8, 1>::from_size_val(image.size(), 0u8, CpuAllocator)?;
    imgproc::color::gray_from_rgb_u8(&image, &mut gray)?;

    let threshold = otsu_threshold(gray.as_slice());
    let mut binary = CpuImage::<u8, 1>::from_size_val(gray.size(), 0u8, CpuAllocator)?;
    imgproc::threshold::threshold_binary(&gray, &mut binary, threshold, 255)?;

    let mut mask = binary.as_slice().to_vec();
    ensure_foreground_convention(&mut mask);

    let components = extract_components(
        &mask,
        gray.as_slice(),
        &raw_pixels,
        width as usize,
        height as usize,
        config,
    );
    let mut canvas: RgbaImage = source.to_rgba8();
    for dot in components.iter() {
        draw_ring(&mut canvas, dot.center, dot.radius, dot.color);
    }

    let mut row_ys = Vec::new();
    let mut target_dots: Vec<&DotDetection> = components
        .iter()
        .filter(|d| d.color == config.color_red || d.color == config.color_magenta)
        .collect();

    target_dots.sort_by(|a, b| {
        a.center
            .1
            .partial_cmp(&b.center.1)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    if !target_dots.is_empty() {
        let avg_radius =
            target_dots.iter().map(|d| d.radius).sum::<f32>() / target_dots.len() as f32;
        let row_threshold = avg_radius * 3.0;

        let mut rows = Vec::new();
        let mut current_row: Vec<&DotDetection> = Vec::new();

        for dot in target_dots {
            if let Some(last) = current_row.last()
                && (dot.center.1 - last.center.1) > row_threshold
            {
                rows.push(current_row);
                current_row = Vec::new();
            }
            current_row.push(dot);
        }
        if !current_row.is_empty() {
            rows.push(current_row);
        }

        for row in rows {
            let avg_y: f32 = row.iter().map(|d| d.center.1).sum::<f32>() / row.len() as f32;
            row_ys.push(avg_y);
            let y_i = avg_y.round() as i32;
            if y_i >= 0 && y_i < height as i32 {
                for x in 0..width {
                    canvas.put_pixel(x, y_i as u32, config.color_magenta);
                }
            }
        }
    }

    let mut col_xs = Vec::new();
    let mut col_dots: Vec<&DotDetection> = components
        .iter()
        .filter(|d| d.color == config.color_black || d.color == config.color_blue)
        .collect();

    col_dots.sort_by(|a, b| {
        a.center
            .0
            .partial_cmp(&b.center.0)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    if !col_dots.is_empty() {
        let avg_radius = col_dots.iter().map(|d| d.radius).sum::<f32>() / col_dots.len() as f32;
        let col_threshold = avg_radius * 3.0;

        let mut cols = Vec::new();
        let mut current_col: Vec<&DotDetection> = Vec::new();

        for dot in col_dots {
            if let Some(last) = current_col.last()
                && (dot.center.0 - last.center.0) > col_threshold
            {
                cols.push(current_col);
                current_col = Vec::new();
            }
            current_col.push(dot);
        }
        if !current_col.is_empty() {
            cols.push(current_col);
        }

        for col in cols {
            let avg_x: f32 = col.iter().map(|d| d.center.0).sum::<f32>() / col.len() as f32;
            col_xs.push(avg_x);
            let x_i = avg_x.round() as i32;
            if x_i >= 0 && x_i < width as i32 {
                for y in 0..height {
                    canvas.put_pixel(x_i as u32, y, config.color_blue);
                }
            }
        }
    }

    let mut arrow_grid = String::new();
    row_ys.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    col_xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    for &y in &row_ys {
        for &x in &col_xs {
            let mut min_dist_sq = f32::MAX;
            let mut closest_dot = None;

            for dot in &components {
                let dx = dot.center.0 - x;
                let dy = dot.center.1 - y;
                let dist_sq = dx * dx + dy * dy;
                if dist_sq < min_dist_sq {
                    min_dist_sq = dist_sq;
                    closest_dot = Some(dot);
                }
            }

            if let Some(dot) = closest_dot {
                let dx = dot.center.0 - x;
                let dy = dot.center.1 - y;
                let arrow = if dx.abs() > dy.abs() {
                    if dx > 0.0 { "→" } else { "←" }
                } else if dy > 0.0 {
                    "↓"
                } else {
                    "↑"
                };
                arrow_grid.push_str(arrow);
                arrow_grid.push(' ');
            } else {
                arrow_grid.push('?');
                arrow_grid.push(' ');
            }
        }
        arrow_grid.push('\n');
    }

    Ok((DynamicImage::ImageRgba8(canvas), arrow_grid))
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
    config: &AnotoConfig,
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

        if !(config.min_component_pixels..=config.max_component_pixels).contains(&count) {
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
        let center = blend_centers(uniform_center, weighted_center, config.centrality_threshold);
        let radius = ((count as f32) / PI).sqrt().max(1.5) * 1.35 + 1.5;
        let mean_r = sum_r as f32 / count as f32;
        let mean_g = sum_g as f32 / count as f32;
        let mean_b = sum_b as f32 / count as f32;
        let color = classify_color(mean_r, mean_g, mean_b, config);
        out.push(DotDetection {
            center,
            radius,
            color,
        });
    }

    out
}

fn classify_color(r: f32, g: f32, b: f32, config: &AnotoConfig) -> Rgba<u8> {
    if r < 70.0 && g < 70.0 && b < 70.0 {
        config.color_black
    } else if r > 190.0 && b < 120.0 && r - g > 40.0 {
        config.color_red
    } else if b > 190.0 && r < 150.0 && b - g > 30.0 {
        config.color_blue
    } else {
        config.color_magenta
    }
}

fn blend_centers(uniform: (f32, f32), weighted: (f32, f32), threshold: f32) -> (f32, f32) {
    let shift = ((weighted.0 - uniform.0).abs() + (weighted.1 - uniform.1).abs()) * 0.5;
    if shift > threshold {
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
    let radius = radius.max(2.0);
    let stroke_width = 1.2;
    let target_radius = radius - 0.5;

    let cx_i = cx.round() as i32;
    let cy_i = cy.round() as i32;
    let max_r = (radius + 1.5).ceil() as i32;

    let width = canvas.width() as i32;
    let height = canvas.height() as i32;

    let min_x = (cx_i - max_r).max(0);
    let max_x = (cx_i + max_r).min(width - 1);
    let min_y = (cy_i - max_r).max(0);
    let max_y = (cy_i + max_r).min(height - 1);

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let dist = (dx * dx + dy * dy).sqrt();

            let dist_from_ring = (dist - target_radius).abs();
            let half_stroke = stroke_width * 0.5;

            if dist_from_ring < half_stroke + 0.5 {
                let coverage = if dist_from_ring < half_stroke - 0.5 {
                    1.0
                } else {
                    1.0 - (dist_from_ring - (half_stroke - 0.5))
                };

                let pixel = canvas.get_pixel_mut(x as u32, y as u32);
                let bg = pixel.0;
                let fg = color.0;

                let alpha = (fg[3] as f32 / 255.0) * coverage;
                let inv_alpha = 1.0 - alpha;

                let r = (fg[0] as f32 * alpha + bg[0] as f32 * inv_alpha) as u8;
                let g = (fg[1] as f32 * alpha + bg[1] as f32 * inv_alpha) as u8;
                let b = (fg[2] as f32 * alpha + bg[2] as f32 * inv_alpha) as u8;

                *pixel = Rgba([r, g, b, 255]);
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
