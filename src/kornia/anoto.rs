#![allow(dead_code)]
//! Detects anoto dots in a preview image, draws circles around them, and provides grid line drawing functions.

use image::{DynamicImage, Rgba, RgbaImage};
use std::collections::{HashSet, VecDeque};
use std::f32::consts::PI;
// Note: thiserror is used via derive attribute `thiserror::Error` on DetectionError
use kornia::{
    image::{Image, ImageError, ImageSize, allocator::CpuAllocator},
    imgproc,
};

type CpuImage<T, const C: usize> = Image<T, C, CpuAllocator>;

const MIN_COMPONENT_PIXELS: usize = 3;
const MAX_COMPONENT_PIXELS: usize = 250;
const CENTRALITY_THRESHOLD: f32 = 0.55;
const COLOR_GREEN: Rgba<u8> = Rgba([110, 170, 90, 255]);
const COLOR_ORANGE: Rgba<u8> = Rgba([230, 130, 30, 255]);
const COLOR_BLUE: Rgba<u8> = Rgba([60, 110, 220, 255]);

type GridDetectionResult = (usize, usize, String, Option<(f32, f32)>);
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
    pub color_green: Rgba<u8>,
    pub color_orange: Rgba<u8>,
    pub color_blue: Rgba<u8>,
    pub color_magenta: Rgba<u8>,
}

impl Default for AnotoConfig {
    fn default() -> Self {
        Self {
            min_component_pixels: MIN_COMPONENT_PIXELS,
            max_component_pixels: MAX_COMPONENT_PIXELS,
            centrality_threshold: CENTRALITY_THRESHOLD,
            color_green: COLOR_GREEN,
            color_orange: COLOR_ORANGE,
            color_blue: COLOR_BLUE,
            color_magenta: COLOR_MAGENTA,
        }
    }
}

/// Represents a detected dot in an Anoto pattern.
///
/// Contains the dot's center coordinates, radius, actual dot color, and classified type color.
#[derive(Debug, Clone)]
pub struct DotDetection {
    pub center: (f32, f32),
    pub radius: f32,
    pub dot_color: Rgba<u8>,
    pub type_color: Rgba<u8>,
}

/// Errors that can occur during Anoto dot detection.
#[derive(Debug, thiserror::Error)]
pub enum DetectionError {
    #[error("kornia image error: {0}")]
    Kornia(#[from] ImageError),
}

/// Result of Anoto dot annotation.
#[derive(Debug, Clone)]
pub struct AnotoDetection {
    pub annotated: DynamicImage,
    pub arrow_grid: String,
    pub origin: Option<(f32, f32)>,
}

/// Detects and annotates Anoto dots in an image.
///
/// This function processes an image to identify Anoto dot patterns, classifies them
/// by color, and returns an annotated version with visual overlays plus a grid
/// representation of the detected pattern, along with the inferred origin between
/// the detected dot rows/columns.
pub fn annotate_anoto_dots(
    source: &DynamicImage,
    config: &AnotoConfig,
) -> Result<AnotoDetection, DetectionError> {
    let rgb = source.to_rgb8();
    let (width, height) = rgb.dimensions();
    if width == 0 || height == 0 {
        return Ok(AnotoDetection {
            annotated: source.clone(),
            arrow_grid: String::new(),
            origin: None,
        });
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

    // Detect rotation: if more unique x positions than y, assume 90 degree rotation
    let xs: Vec<f32> = components.iter().map(|d| d.center.0).collect();
    let ys: Vec<f32> = components.iter().map(|d| d.center.1).collect();
    let unique_x: HashSet<i32> = xs.iter().map(|&x| x.round() as i32).collect();
    let unique_y: HashSet<i32> = ys.iter().map(|&y| y.round() as i32).collect();
    let _rotated = unique_x.len() > unique_y.len();

    // Adjust components for rotation - removed to avoid coordinate mismatch in drawing
    // Use original components for drawing on original canvas

    // Check for exactly 4 unique colors
    let unique_colors: std::collections::HashSet<Rgba<u8>> = components.iter().map(|d| d.type_color).collect();
    if unique_colors.len() != 4 {
        eprintln!("Warning: Detected {} unique colors, expected 4.", unique_colors.len());
    }

    let mut canvas: RgbaImage = source.to_rgba8();
    for dot in components.iter() {
        draw_ring(&mut canvas, dot.center, dot.radius, dot.dot_color);
    }

    draw_grid_lines(&mut canvas, &components);

    let (arrow_grid, origin) = match detect_grid(&components, config) {
        Some((_rows, _cols, grid, origin)) => (grid, origin),
        None => (String::new(), None),
    };

    Ok(AnotoDetection {
        annotated: DynamicImage::ImageRgba8(canvas),
        arrow_grid,
        origin,
    })
}

/// Detects dots and returns the raw component detections for programmatic use.
pub fn detect_components_from_image(
    source: &DynamicImage,
    config: &AnotoConfig,
) -> Result<Vec<DotDetection>, DetectionError> {
    let rgb = source.to_rgb8();
    let (width, height) = rgb.dimensions();
    if width == 0 || height == 0 {
        return Ok(Vec::new());
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
    Ok(extract_components(
        &mask,
        gray.as_slice(),
        &raw_pixels,
        width as usize,
        height as usize,
        config,
    ))
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
        let mut component_pixels: Vec<usize> = Vec::new();

        while let Some(idx) = queue.pop_front() {
            let y = idx / width;
            let x = idx % width;
            sum_x += x as f32;
            sum_y += y as f32;
            count += 1;
            component_pixels.push(idx);
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
            // if too large, attempt to split into smaller dots
            if count > config.max_component_pixels {
                let splits = split_large_component(&component_pixels, mask, grayscale, rgb, width, height, config);
                for d in splits { out.push(d); }
            }
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
        let dot_color = Rgba([mean_r as u8, mean_g as u8, mean_b as u8, 255]);
        let type_color = classify_color(mean_r, mean_g, mean_b, config);
        out.push(DotDetection {
            center,
            radius,
            dot_color,
            type_color,
        });
    }

    out
}

fn color_distance(a: Rgba<u8>, b: Rgba<u8>) -> f32 {
    let dr = a.0[0] as f32 - b.0[0] as f32;
    let dg = a.0[1] as f32 - b.0[1] as f32;
    let db = a.0[2] as f32 - b.0[2] as f32;
    (dr * dr + dg * dg + db * db).sqrt()
}

fn saturation(r: f32, g: f32, b: f32) -> f32 {
    let max = r.max(g).max(b);
    if max == 0.0 {
        0.0
    } else {
        (max - r.min(g).min(b)) / max
    }
}

fn classify_color(r: f32, g: f32, b: f32, config: &AnotoConfig) -> Rgba<u8> {
    let candidate = Rgba([r as u8, g as u8, b as u8, 255]);
    let palette = [config.color_green, config.color_orange, config.color_blue, config.color_magenta];
    let mut best = palette[0];
    let mut best_d = color_distance(candidate, best);
    for &c in &palette[1..] {
        let d = color_distance(candidate, c);
        if d < best_d {
            best = c;
            best_d = d;
        }
    }
    best
}

fn draw_line(canvas: &mut RgbaImage, start: (f32, f32), end: (f32, f32), thickness: f32, color: Rgba<u8>, dots: &[DotDetection]) {
    let half_thickness = thickness / 2.0;
    if (start.0 - end.0).abs() < 1e-6 {
        // Vertical line
        let x = start.0.round() as i32;
        let y_start = start.1.min(end.1).round() as i32;
        let y_end = start.1.max(end.1).round() as i32;
        let width = canvas.width() as i32;
        let height = canvas.height() as i32;
        if x < 0 || x >= width { return; }
        for y in y_start.max(0)..=y_end.min(height - 1) {
            for dx in (-half_thickness.round() as i32)..=(half_thickness.round() as i32) {
                let px = x + dx;
                if px >= 0 && px < width {
                    let point = (px as f32, y as f32);
                    let mut inside_other_dot = false;
                    for dot in dots {
                        let dist_x = point.0 - dot.center.0;
                        let dist_y = point.1 - dot.center.1;
                        let dist = (dist_x * dist_x + dist_y * dist_y).sqrt();
                        let core_radius = dot.radius * 0.5;
                        // always preserve center core pixels (don't overwrite any center)
                        if dist < core_radius {
                            inside_other_dot = true;
                            break;
                        }
                        // if another color dot is here, do not draw through it
                        if dist < dot.radius && dot.type_color != color {
                            inside_other_dot = true;
                            break;
                        }
                    }
                    if !inside_other_dot {
                        canvas.put_pixel(px as u32, y as u32, color);
                    }
                }
            }
        }
    } else if (start.1 - end.1).abs() < 1e-6 {
        // Horizontal line
        let y = start.1.round() as i32;
        let x_start = start.0.min(end.0).round() as i32;
        let x_end = start.0.max(end.0).round() as i32;
        let width = canvas.width() as i32;
        let height = canvas.height() as i32;
        if y < 0 || y >= height { return; }
        for x in x_start.max(0)..=x_end.min(width - 1) {
            for dy in (-half_thickness.round() as i32)..=(half_thickness.round() as i32) {
                let py = y + dy;
                if py >= 0 && py < height {
                    let point = (x as f32, py as f32);
                    let mut inside_other_dot = false;
                    for dot in dots {
                        let dist_x = point.0 - dot.center.0;
                        let dist_y = point.1 - dot.center.1;
                        let dist = (dist_x * dist_x + dist_y * dist_y).sqrt();
                        let core_radius = dot.radius * 0.5;
                        if dist < core_radius {
                            inside_other_dot = true;
                            break;
                        }
                        if dist < dot.radius && dot.type_color != color {
                            inside_other_dot = true;
                            break;
                        }
                    }
                    if !inside_other_dot {
                        canvas.put_pixel(x as u32, py as u32, color);
                    }
                }
            }
        }
    }
}

fn draw_grid_lines(canvas: &mut RgbaImage, dots: &[DotDetection]) {
    // no HashMap needed here
    let global_avg_radius: f32 = dots.iter().map(|d| d.radius).sum::<f32>() / dots.len() as f32;
    let thickness = (global_avg_radius / 4.0).max(1.0);

    // compute rotation like detect_grid
    let centers: Vec<(f32,f32)> = dots.iter().map(|d| d.center).collect();
    let mean_x = centers.iter().map(|c| c.0).sum::<f32>() / centers.len() as f32;
    let mean_y = centers.iter().map(|c| c.1).sum::<f32>() / centers.len() as f32;
    let mut sxx = 0f32; let mut sxy = 0f32; let mut syy = 0f32;
    for &(x,y) in &centers { let dx = x - mean_x; let dy = y - mean_y; sxx += dx*dx; sxy += dx*dy; syy += dy*dy; }
    let n = centers.len() as f32; sxx /= n; sxy /= n; syy /= n;
    let trace = sxx + syy; let det = sxx*syy - sxy*sxy; let temp = ((trace*trace)/4.0 - det).max(0.0); let lambda = trace/2.0 + temp.sqrt();
    let vx = lambda - syy; let vy = sxy; let angle = vy.atan2(vx);
    let cos_a = angle.cos(); let sin_a = angle.sin();
    let mut rotated: Vec<(f32,f32)> = Vec::new();
    for &(x,y) in &centers { let dx = x - mean_x; let dy = y - mean_y; let rx = dx * cos_a + dy * sin_a; let ry = -dx * sin_a + dy * cos_a; rotated.push((rx, ry)); }
    let xs: Vec<f32> = rotated.iter().map(|c| c.0).collect(); let ys: Vec<f32> = rotated.iter().map(|c| c.1).collect();
    let cols = cluster_positions(&xs); let rows = cluster_positions(&ys);
    if cols.is_empty() || rows.is_empty() { return; }

    // compute rotated bounds using canvas corners
    let width = canvas.width() as f32; let height = canvas.height() as f32;
    let corners = [(0.0f32, 0.0f32), (width, 0.0f32), (0.0f32, height), (width, height)];
    let mut min_rx = f32::MAX; let mut max_rx = f32::MIN; let mut min_ry = f32::MAX; let mut max_ry = f32::MIN;
    for &(cx, cy) in corners.iter() {
        let dx = cx - mean_x; let dy = cy - mean_y; let rx = dx * cos_a + dy * sin_a; let ry = -dx * sin_a + dy * cos_a;
        if rx < min_rx { min_rx = rx; }
        if rx > max_rx { max_rx = rx; }
        if ry < min_ry { min_ry = ry; }
        if ry > max_ry { max_ry = ry; }
    }
    let cols_full = expand_positions(cols.clone(), min_rx, max_rx);
    let rows_full = expand_positions(rows.clone(), min_ry, max_ry);

    // Top-down: draw horizontal lines in ascending original Y
    let mut rows_with_y: Vec<(f32, f32)> = rows_full.iter().map(|&ry| {
        let orig_y = mean_y + ry * cos_a; (ry, orig_y)
    }).collect();
    rows_with_y.sort_by(|a,b| a.1.partial_cmp(&b.1).unwrap());
    for (ry, _orig_y) in rows_with_y.iter() {
        // choose color from nearest dot if available
        let mut best = None::<Rgba<u8>>; let mut best_d = f32::MAX;
        for d in dots.iter() { let dx = d.center.0 - mean_x; let dy = d.center.1 - mean_y; let rry = -dx * sin_a + dy * cos_a; let delta = (rry - *ry).abs(); if delta < best_d { best_d = delta; best = Some(d.type_color); } }
        let color = best.unwrap_or(Rgba([200,200,200,255]));
        let start_rot = (min_rx, *ry); let end_rot = (max_rx, *ry);
        let start = (mean_x + (start_rot.0 * cos_a - start_rot.1 * sin_a), mean_y + (start_rot.0 * sin_a + start_rot.1 * cos_a));
        let end = (mean_x + (end_rot.0 * cos_a - end_rot.1 * sin_a), mean_y + (end_rot.0 * sin_a + end_rot.1 * cos_a));
        draw_line(canvas, start, end, thickness, color, dots);
    }

    // Left-to-right: draw vertical lines in ascending original X
    let mut cols_with_x: Vec<(f32, f32)> = cols_full.iter().map(|&rx| { let orig_x = mean_x + rx * cos_a; (rx, orig_x) }).collect();
    cols_with_x.sort_by(|a,b| a.1.partial_cmp(&b.1).unwrap());
    for (rx, _orig_x) in cols_with_x.iter() {
        let mut best = None::<Rgba<u8>>; let mut best_d = f32::MAX;
        for d in dots.iter() { let dx = d.center.0 - mean_x; let dy = d.center.1 - mean_y; let rrx = dx * cos_a + dy * sin_a; let delta = (rrx - *rx).abs(); if delta < best_d { best_d = delta; best = Some(d.type_color); } }
        let color = best.unwrap_or(Rgba([200,200,200,255]));
        let start_rot = (*rx, min_ry); let end_rot = (*rx, max_ry);
        let start = (mean_x + (start_rot.0 * cos_a - start_rot.1 * sin_a), mean_y + (start_rot.0 * sin_a + start_rot.1 * cos_a));
        let end = (mean_x + (end_rot.0 * cos_a - end_rot.1 * sin_a), mean_y + (end_rot.0 * sin_a + end_rot.1 * cos_a));
        draw_line(canvas, start, end, thickness, color, dots);
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
                // Solid overwrite to keep ring color identical to dot classification
                *canvas.get_pixel_mut(x as u32, y as u32) = color;
            }
        }
    }
}

fn draw_horizontal_line(canvas: &mut RgbaImage, y: f32, color: Rgba<u8>, all_dots: &[DotDetection], should_skip: impl Fn(&DotDetection) -> bool) {
    let y_i = y.round() as i32;
    let width = canvas.width() as i32;
    let height = canvas.height() as i32;
    if y_i < 0 || y_i >= height {
        return;
    }
    for x in 0..width {
        let point = (x as f32, y);
        let mut skip = false;
        for dot in all_dots {
            if should_skip(dot) {
                let dx = point.0 - dot.center.0;
                let dy = point.1 - dot.center.1;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist < dot.radius {
                    skip = true;
                    break;
                }
            }
        }
        if !skip {
            canvas.put_pixel(x as u32, y_i as u32, color);
        }
    }
}

fn draw_vertical_line(canvas: &mut RgbaImage, x: f32, color: Rgba<u8>, all_dots: &[DotDetection], should_skip: impl Fn(&DotDetection) -> bool) {
    let x_i = x.round() as i32;
    let width = canvas.width() as i32;
    let height = canvas.height() as i32;
    if x_i < 0 || x_i >= width {
        return;
    }
    for y in 0..height {
        let point = (x, y as f32);
        let mut skip = false;
        for dot in all_dots {
            if should_skip(dot) {
                let dx = point.0 - dot.center.0;
                let dy = point.1 - dot.center.1;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist < dot.radius {
                    skip = true;
                    break;
                }
            }
        }
        if !skip {
            canvas.put_pixel(x_i as u32, y as u32, color);
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

fn split_large_component(
    pixels: &[usize],
    _mask: &[u8],
    _grayscale: &[u8],
    rgb: &[u8],
    width: usize,
    height: usize,
    config: &AnotoConfig,
) -> Vec<DotDetection> {
    if pixels.is_empty() { return Vec::new(); }
    // bounding box
    let mut min_x = width; let mut min_y = height; let mut max_x = 0usize; let mut max_y = 0usize;
    for &idx in pixels {
        let y = idx / width; let x = idx % width;
        if x < min_x { min_x = x; }
        if x > max_x { max_x = x; }
        if y < min_y { min_y = y; }
        if y > max_y { max_y = y; }
    }
    let w2 = max_x - min_x + 1;
    let h2 = max_y - min_y + 1;
    // build submask
    let mut submask = vec![0u8; w2*h2];
    for &idx in pixels {
        let y = idx / width; let x = idx % width;
        let sx = x - min_x; let sy = y - min_y; submask[sy*w2 + sx] = 1u8;
    }
    // distance transform: initialize distances
    let mut dist = vec![-1i32; w2*h2];
    let mut q = VecDeque::new();
    for sy in 0..h2 {
        for sx in 0..w2 {
            let idx = sy*w2 + sx;
            if submask[idx] == 0 { dist[idx] = 0; q.push_back(idx); }
        }
    }
    while let Some(i) = q.pop_front() {
        let x = i % w2; let y = i / w2;
        for (dx, dy) in [(-1isize,0),(1,0),(0,-1),(0,1)] {
            let nx = x as isize + dx; let ny = y as isize + dy;
            if nx < 0 || ny < 0 || nx >= w2 as isize || ny >= h2 as isize { continue; }
            let ni = ny as usize * w2 + nx as usize;
            if dist[ni] == -1 {
                dist[ni] = dist[i] + 1;
                q.push_back(ni);
            }
        }
    }

    // find local maxima in dist for foreground pixels
    let mut peaks: Vec<(usize, usize, i32)> = Vec::new();
    for sy in 0..h2 {
        for sx in 0..w2 {
            let idx = sy*w2 + sx;
            if submask[idx] == 0 { continue; }
            let d = dist[idx]; if d <= 0 { continue; }
            let mut is_peak = true;
            for (dx, dy) in [(-1isize,0),(1,0),(0,-1),(0,1)] {
                let nx = sx as isize + dx; let ny = sy as isize + dy;
                if nx < 0 || ny < 0 || nx >= w2 as isize || ny >= h2 as isize { continue; }
                let ni = ny as usize * w2 + nx as usize;
                if dist[ni] > d { is_peak = false; break; }
            }
            if is_peak { peaks.push((sx, sy, d)); }
        }
    }

    // sort peaks by distance desc
    peaks.sort_by(|a,b| b.2.cmp(&a.2));
    let mut detections = Vec::new();
    for (sx, sy, d) in peaks {
        if d < 3 { continue; }
        // global center
        let cx = (min_x + sx) as f32;
        let cy = (min_y + sy) as f32;
        let radius = d as f32; // approx
        // compute mean color in small neighborhood
        let mut sum_r = 0u32; let mut sum_g = 0u32; let mut sum_b = 0u32; let mut cnt = 0usize;
        let gx_min = (cx as isize - radius as isize - 1).max(0) as usize;
        let gy_min = (cy as isize - radius as isize - 1).max(0) as usize;
        let gx_max = (cx as usize + radius as usize + 1).min(width - 1);
        let gy_max = (cy as usize + radius as usize + 1).min(height - 1);
        for gy in gy_min..=gy_max {
            for gx in gx_min..=gx_max {
                let dx = gx as f32 - cx; let dy = gy as f32 - cy; if (dx*dx+dy*dy).sqrt() > radius+1.0 { continue; }
                let idx = gy * width + gx; let rgb_idx = idx*3;
                if rgb_idx + 2 < rgb.len() {
                    sum_r += rgb[rgb_idx] as u32; sum_g += rgb[rgb_idx+1] as u32; sum_b += rgb[rgb_idx+2] as u32; cnt += 1;
                }
            }
        }
        if cnt == 0 { continue; }
        let mean_r = sum_r as f32 / cnt as f32; let mean_g = sum_g as f32 / cnt as f32; let mean_b = sum_b as f32 / cnt as f32;
        let dot_color = Rgba([mean_r as u8, mean_g as u8, mean_b as u8, 255]);
        let type_color = classify_color(mean_r, mean_g, mean_b, config);
        detections.push(DotDetection { center: (cx, cy), radius, dot_color, type_color });
    }

    detections
}

/// Attempt to infer grid rows/columns and produce a simple textual representation.
pub fn detect_grid(dots: &[DotDetection], config: &AnotoConfig) -> Option<GridDetectionResult> {
    if dots.is_empty() { return None; }
    // extract centers
    let centers: Vec<(f32,f32)> = dots.iter().map(|d| d.center).collect();

    // compute centroid
    let mean_x = centers.iter().map(|c| c.0).sum::<f32>() / centers.len() as f32;
    let mean_y = centers.iter().map(|c| c.1).sum::<f32>() / centers.len() as f32;

    // compute covariance matrix
    let mut sxx = 0f32; let mut sxy = 0f32; let mut syy = 0f32;
    for &(x,y) in &centers {
        let dx = x - mean_x; let dy = y - mean_y;
        sxx += dx * dx; sxy += dx * dy; syy += dy * dy;
    }
    let n = centers.len() as f32;
    sxx /= n; sxy /= n; syy /= n;
    // principal eigenvector of 2x2 [[sxx,sxy],[sxy,syy]]
    let trace = sxx + syy;
    let det = sxx*syy - sxy*sxy;
    let temp = ((trace*trace)/4.0 - det).max(0.0);
    let lambda = trace/2.0 + temp.sqrt();
    // eigenvector (a - c, b)
    let vx = lambda - syy; let vy = sxy;
    let angle = vy.atan2(vx); // direction of principal axis

    // rotate points by -angle to align grid axes
    let cos_a = angle.cos(); let sin_a = angle.sin();
    let mut rotated: Vec<(f32,f32)> = Vec::new();
    for &(x,y) in &centers {
        let dx = x - mean_x; let dy = y - mean_y;
        let rx = dx * cos_a + dy * sin_a;
        let ry = -dx * sin_a + dy * cos_a;
        rotated.push((rx, ry));
    }

    // cluster x and y into unique columns/rows

    let xs: Vec<f32> = rotated.iter().map(|c| c.0).collect();
    let ys: Vec<f32> = rotated.iter().map(|c| c.1).collect();
    let cols = cluster_positions(&xs);
    let rows = cluster_positions(&ys);

    // Expand rows/cols to include missing grid positions due to empty rows/columns
    fn expand_positions(mut centers: Vec<f32>, min_b: f32, max_b: f32) -> Vec<f32> {
        if centers.is_empty() { return centers; }
        centers.sort_by(|a,b| a.partial_cmp(b).unwrap());
        if centers.len() == 1 {
            return vec![centers[0]];
        }
        let mut diffs: Vec<f32> = Vec::new();
        for i in 1..centers.len() { diffs.push((centers[i] - centers[i-1]).abs()); }
        diffs.sort_by(|a,b| a.partial_cmp(b).unwrap());
        let spacing = if diffs.is_empty() { 1.0 } else { diffs[diffs.len()/2].max(1e-3) };
        // align to the first center
        let first = centers[0];
        let rel = ((first - min_b) / spacing).round();
        let start = min_b + rel * spacing;
        let mut pos = Vec::new();
        let mut p = start;
        while p - spacing >= min_b - spacing * 0.5 { p -= spacing; }
        while p <= max_b + spacing * 0.5 {
            pos.push(p);
            p += spacing;
            if pos.len() > 2000 { break; }
        }
        pos
    }

    // derive rotated bounds based on extremes of the detected dots with padding
    let min_x = centers.iter().map(|c| c.0).fold(f32::MAX, |a,b| a.min(b));
    let max_x = centers.iter().map(|c| c.0).fold(f32::MIN, |a,b| a.max(b));
    let min_y = centers.iter().map(|c| c.1).fold(f32::MAX, |a,b| a.min(b));
    let max_y = centers.iter().map(|c| c.1).fold(f32::MIN, |a,b| a.max(b));
    let pad = 10.0_f32;
    let corners = [(min_x - pad, min_y - pad),(max_x + pad, min_y - pad),(min_x - pad, max_y + pad),(max_x + pad, max_y + pad)];
    let mut min_rx = f32::MAX; let mut max_rx = f32::MIN; let mut min_ry = f32::MAX; let mut max_ry = f32::MIN;
    for &(cx, cy) in corners.iter() {
        let dx = cx - mean_x; let dy = cy - mean_y;
        let rx = dx * cos_a + dy * sin_a;
        let ry = -dx * sin_a + dy * cos_a;
        min_rx = min_rx.min(rx); max_rx = max_rx.max(rx); min_ry = min_ry.min(ry); max_ry = max_ry.max(ry);
    }
    let cols_full = expand_positions(cols.clone(), min_rx, max_rx);
    let rows_full = expand_positions(rows.clone(), min_ry, max_ry);
    if cols.is_empty() || rows.is_empty() { return None; }

    // build grid (use space for empty cells)
    let mut grid = vec![vec![' '; cols_full.len()]; rows_full.len()];
    for (i, &(rx, ry)) in rotated.iter().enumerate() {
        // find nearest col and row
        let mut best_c = 0usize; let mut best_cd = f32::MAX;
        for (ci, &cx) in cols_full.iter().enumerate() { let d = (rx - cx).abs(); if d < best_cd { best_cd = d; best_c = ci; } }
        let mut best_r = 0usize; let mut best_rd = f32::MAX;
        for (ri, &ryv) in rows_full.iter().enumerate() { let d = (ry - ryv).abs(); if d < best_rd { best_rd = d; best_r = ri; } }
        // get color char
        let color_ch = color_char_from_type(dots[i].type_color, config);
        grid[best_r][best_c] = color_ch;
    }

    // build ASCII grid string
    let mut lines = Vec::new();
    for row in grid.iter() { let line: String = row.iter().collect(); lines.push(line); }
    let arrow_grid = lines.join("\n");
    // approximate origin: top-left grid center -> convert to original coordinates
    let origin = Some((mean_x + cols_full[0] * cos_a - rows_full[0] * sin_a, mean_y + cols_full[0] * sin_a + rows_full[0] * cos_a));
    Some((rows_full.len(), cols_full.len(), arrow_grid, origin))
}

// Expand positions across bounds using median spacing of centers
fn expand_positions(mut centers: Vec<f32>, min_b: f32, max_b: f32) -> Vec<f32> {
    if centers.is_empty() { return centers; }
    centers.sort_by(|a,b| a.partial_cmp(b).unwrap());
    if centers.len() == 1 { return vec![centers[0]]; }
    let mut diffs: Vec<f32> = Vec::new();
    for i in 1..centers.len() { diffs.push((centers[i] - centers[i-1]).abs()); }
    diffs.sort_by(|a,b| a.partial_cmp(b).unwrap());
    let spacing = if diffs.is_empty() { 1.0 } else { diffs[diffs.len()/2].max(1e-3) };
    let first = centers[0];
    // Anchor the expanded positions so that the first detected center is included exactly
    let start = first;
    let mut pos = Vec::new();
    let mut p = start;
    while p - spacing >= min_b - spacing * 0.5 { p -= spacing; }
    while p <= max_b + spacing * 0.5 {
        pos.push(p);
        p += spacing;
        if pos.len() > 2000 { break; }
    }
    pos
}

// Group nearby positions into cluster centers (median spacing threshold)
fn cluster_positions(vals: &[f32]) -> Vec<f32> {
    let mut sorted = vals.to_vec();
    sorted.sort_by(|a,b| a.partial_cmp(b).unwrap());
    // find median spacing
    let mut diffs = Vec::new();
    for i in 1..vals.len() { diffs.push(vals[i] - vals[i-1]); }
    let spacing = if diffs.is_empty() { 1.0 } else { let mut ds = diffs.clone(); ds.sort_by(|a,b| a.partial_cmp(b).unwrap()); ds[ds.len()/2] };
    let mut clusters: Vec<f32> = Vec::new();
    if vals.is_empty() { return clusters; }
    let mut cur = vals[0];
    for v in vals.iter().skip(1) {
        if (v - cur).abs() > spacing * 0.6 {
            clusters.push(cur);
            cur = *v;
        } else {
            // average into cluster center
            cur = (cur + *v) / 2.0;
        }
    }
    clusters.push(cur);
    clusters
}

fn color_char_from_type(c: Rgba<u8>, config: &AnotoConfig) -> char {
    // Canonical mapping used by the decoder / expected minified grids:
    // green=↑, orange=↓, blue=←, magenta=→
    let palette = [
        config.color_green,
        config.color_orange,
        config.color_blue,
        config.color_magenta,
    ];
    let chars = ['↑', '↓', '←', '→'];
    let mut best = 0usize;
    let mut best_d = color_distance(c, palette[0]);

    for (i, &color) in palette.iter().enumerate().skip(1) {
        let d = color_distance(c, color);
        if d < best_d {
            best_d = d;
            best = i;
        }
    }

    chars[best]
}
