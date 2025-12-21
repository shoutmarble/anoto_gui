use plotters::prelude::*;

use crate::kornia::anoto::{AnotoConfig, DotDetection};

use std::collections::{HashMap, HashSet};

fn rgba_to_plotters(c: image::Rgba<u8>) -> RGBColor {
    RGBColor(c[0], c[1], c[2])
}

fn color_distance(a: image::Rgba<u8>, b: image::Rgba<u8>) -> u32 {
    let dr = a[0].abs_diff(b[0]) as u32;
    let dg = a[1].abs_diff(b[1]) as u32;
    let db = a[2].abs_diff(b[2]) as u32;
    dr * dr + dg * dg + db * db
}

fn nearest_type_color(c: image::Rgba<u8>, config: &AnotoConfig) -> image::Rgba<u8> {
    let palette = [
        config.color_green,
        config.color_orange,
        config.color_blue,
        config.color_magenta,
    ];

    let mut best = palette[0];
    let mut best_d = color_distance(c, palette[0]);
    for &p in palette.iter().skip(1) {
        let d = color_distance(c, p);
        if d < best_d {
            best_d = d;
            best = p;
        }
    }
    best
}

fn arrow_for_dot(dot: &DotDetection, config: &AnotoConfig) -> Option<char> {
    // Be tolerant: type_color can differ slightly due to preprocessing.
    let t = nearest_type_color(dot.type_color, config);
    // Canonical mapping used by the decoder / expected minified grids:
    // green=↑, orange=↓, blue=←, magenta=→
    if t == config.color_green {
        Some('↑')
    } else if t == config.color_orange {
        Some('↓')
    } else if t == config.color_blue {
        Some('←')
    } else if t == config.color_magenta {
        Some('→')
    } else {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RowKind {
    Green,
    Mid,
    Orange,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ColKind {
    Blue,
    Vertical,
    Magenta,
    Unknown,
}

fn row_kind_for_y(dots: &[DotDetection], y: i32, config: &AnotoConfig) -> RowKind {
    let mut green = 0usize;
    let mut mid = 0usize;
    let mut orange = 0usize;

    for d in dots {
        if d.center.1.round() as i32 != y {
            continue;
        }
        // Classify by dot-type color (robust + avoids coupling to arrow mapping).
        let t = nearest_type_color(d.type_color, config);
        if t == config.color_green {
            green += 1;
        } else if t == config.color_orange {
            orange += 1;
        } else if t == config.color_blue || t == config.color_magenta {
            mid += 1;
        }
    }

    let max = green.max(mid).max(orange);
    if max == 0 {
        return RowKind::Unknown;
    }
    if green == max {
        RowKind::Green
    } else if mid == max {
        RowKind::Mid
    } else {
        RowKind::Orange
    }
}

fn phase_for_kind(kind: RowKind) -> Option<usize> {
    match kind {
        RowKind::Green => Some(0),
        RowKind::Mid => Some(1),
        RowKind::Orange => Some(2),
        RowKind::Unknown => None,
    }
}

fn col_kind_for_x(dots: &[DotDetection], x: i32, config: &AnotoConfig) -> ColKind {
    let mut blue = 0usize;
    let mut vertical = 0usize;
    let mut magenta = 0usize;

    for d in dots {
        if d.center.0.round() as i32 != x {
            continue;
        }

        // Classify by dot-type color (robust + avoids coupling to arrow mapping).
        let t = nearest_type_color(d.type_color, config);
        if t == config.color_blue {
            blue += 1;
        } else if t == config.color_magenta {
            magenta += 1;
        } else if t == config.color_green || t == config.color_orange {
            vertical += 1;
        }
    }

    let max = blue.max(vertical).max(magenta);
    if max == 0 {
        return ColKind::Unknown;
    }

    if blue == max {
        ColKind::Blue
    } else if vertical == max {
        ColKind::Vertical
    } else {
        ColKind::Magenta
    }
}

fn phase_for_col_kind(kind: ColKind) -> Option<usize> {
    match kind {
        ColKind::Blue => Some(0),
        ColKind::Vertical => Some(1),
        ColKind::Magenta => Some(2),
        ColKind::Unknown => None,
    }
}

fn median_step(mut diffs: Vec<i32>) -> Option<i32> {
    diffs.retain(|d| *d > 0);
    if diffs.is_empty() {
        return None;
    }
    diffs.sort_unstable();
    Some(diffs[diffs.len() / 2])
}

fn cluster_positions_f32(vals: &[f32]) -> Vec<f32> {
    if vals.is_empty() {
        return Vec::new();
    }

    let mut sorted = vals.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    if sorted.len() == 1 {
        return vec![sorted[0]];
    }

    let mut diffs: Vec<f32> = Vec::with_capacity(sorted.len().saturating_sub(1));
    for w in sorted.windows(2) {
        diffs.push((w[1] - w[0]).abs());
    }
    diffs.retain(|d| *d > 0.0);
    diffs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let spacing = if diffs.is_empty() {
        1.0
    } else {
        diffs[diffs.len() / 2].max(1e-3)
    };
    let threshold = spacing * 0.6;

    let mut clusters: Vec<f32> = Vec::new();
    let mut cur = sorted[0];
    for v in sorted.iter().skip(1) {
        if (*v - cur).abs() > threshold {
            clusters.push(cur);
            cur = *v;
        } else {
            cur = (cur + *v) / 2.0;
        }
    }
    clusters.push(cur);
    clusters
}

fn nearest_cluster_index(centers: &[f32], v: f32) -> Option<usize> {
    if centers.is_empty() {
        return None;
    }
    match centers.binary_search_by(|c| c.partial_cmp(&v).unwrap()) {
        Ok(i) => Some(i),
        Err(i) => {
            if i == 0 {
                Some(0)
            } else if i >= centers.len() {
                Some(centers.len() - 1)
            } else {
                let a = centers[i - 1];
                let b = centers[i];
                if (v - a).abs() <= (b - v).abs() {
                    Some(i - 1)
                } else {
                    Some(i)
                }
            }
        }
    }
}

/// Returns (all_x_lines, synthetic_x_set)
fn xs_with_synthetic_cols(
    dots: &[DotDetection],
    config: &AnotoConfig,
    bounds: Option<(i32, i32)>,
    pad_leading: bool,
) -> (Vec<i32>, HashSet<i32>) {
    let mut xs: Vec<i32> = dots.iter().map(|d| d.center.0.round() as i32).collect();
    xs.sort_unstable();
    xs.dedup();

    if xs.len() < 2 {
        return (xs, HashSet::new());
    }

    let diffs: Vec<i32> = xs.windows(2).map(|w| w[1] - w[0]).collect();
    let Some(dx) = median_step(diffs) else {
        return (xs, HashSet::new());
    };
    if dx <= 0 {
        return (xs, HashSet::new());
    }

    let in_bounds = |x: i32| match bounds {
        Some((min_x, max_x)) => (min_x..=max_x).contains(&x),
        None => true,
    };

    // Classify each observed column.
    let mut kinds: HashMap<i32, ColKind> = HashMap::new();
    for &x in &xs {
        kinds.insert(x, col_kind_for_x(dots, x, config));
    }

    let mut all_xs: Vec<i32> = Vec::with_capacity(xs.len() + 4);
    let mut synthetic: HashSet<i32> = HashSet::new();

    // Optional leading phase padding (for decode/minify phase alignment).
    if pad_leading {
        let first_kind = kinds.get(&xs[0]).copied().unwrap_or(ColKind::Unknown);
        if let Some(first_phase) = phase_for_col_kind(first_kind) {
            // If first col is Vertical, we missed Blue to the left.
            // If first col is Magenta, we missed Blue+Vertical to the left.
            let missing_before = first_phase;
            for k in (1..=missing_before).rev() {
                let x = xs[0] - dx * (k as i32);
                if in_bounds(x) {
                    all_xs.push(x);
                    synthetic.insert(x);
                }
            }
        }
    }

    // Walk observed columns and insert synthetic gaps only when the phase sequence indicates
    // a missing column between them (and the spacing supports it).
    for window in xs.windows(2) {
        let x0 = window[0];
        let x1 = window[1];
        all_xs.push(x0);

        let diff = x1 - x0;
        if diff <= 0 {
            continue;
        }

        let kind0 = kinds.get(&x0).copied().unwrap_or(ColKind::Unknown);
        let kind1 = kinds.get(&x1).copied().unwrap_or(ColKind::Unknown);
        let phase0 = phase_for_col_kind(kind0);
        let phase1 = phase_for_col_kind(kind1);

        // How many physical steps separate the phases? (0=>same phase, 1=>adjacent, 2=>skips one)
        // Map that to how many missing columns exist between x0 and x1.
        let missing_by_phase = match (phase0, phase1) {
            (Some(p0), Some(p1)) => {
                let steps = (p1 + 3 - p0) % 3;
                match steps {
                    1 => 0,
                    2 => 1,
                    _ => 2, // 0 means same phase => missing 2
                }
            }
            _ => {
                // Fallback: purely spacing-based if we can't classify either side.
                let steps = ((diff + (dx / 2)) / dx).max(1) as usize;
                steps.saturating_sub(1).min(2)
            }
        };

        // Only insert if the spatial gap is large enough to plausibly contain the missing columns.
        // (tolerance: allow up to ~0.5*dx rounding noise)
        let min_required = dx * (missing_by_phase as i32 + 1) - (dx / 2);
        if diff < min_required {
            continue;
        }

        // Place synthetic columns by subdividing the observed gap to avoid drift.
        // missing=1 => midpoint; missing=2 => thirds.
        match missing_by_phase {
            1 => {
                let x = x0 + (diff / 2);
                if in_bounds(x) {
                    all_xs.push(x);
                    synthetic.insert(x);
                }
            }
            2 => {
                let x_a = x0 + (diff / 3);
                let x_b = x0 + ((2 * diff) / 3);
                for x in [x_a, x_b] {
                    if in_bounds(x) {
                        all_xs.push(x);
                        synthetic.insert(x);
                    }
                }
            }
            _ => {}
        }
    }

    // Push last observed column.
    if let Some(&last) = xs.last() {
        all_xs.push(last);
    }

    all_xs.sort_unstable();
    all_xs.dedup();

    (all_xs, synthetic)
}

/// Returns (all_y_lines, synthetic_y_set)
fn ys_with_synthetic_rows(
    dots: &[DotDetection],
    config: &AnotoConfig,
    bounds: Option<(i32, i32)>,
    pad_leading: bool,
) -> (Vec<i32>, HashSet<i32>) {
    let mut ys: Vec<i32> = dots.iter().map(|d| d.center.1.round() as i32).collect();
    ys.sort_unstable();
    ys.dedup();
    if ys.len() < 2 {
        return (ys, HashSet::new());
    }

    let diffs: Vec<i32> = ys.windows(2).map(|w| w[1] - w[0]).collect();
    let Some(dy) = median_step(diffs) else {
        return (ys, HashSet::new());
    };
    if dy <= 0 {
        return (ys, HashSet::new());
    }

    let in_bounds = |y: i32| match bounds {
        Some((min_y, max_y)) => (min_y..=max_y).contains(&y),
        None => true,
    };

    // Classify each observed row.
    let mut kinds: HashMap<i32, RowKind> = HashMap::new();
    for &y in &ys {
        kinds.insert(y, row_kind_for_y(dots, y, config));
    }

    let mut all_ys: Vec<i32> = Vec::with_capacity(ys.len() + 4);
    let mut synthetic: HashSet<i32> = HashSet::new();

    // Optional leading phase padding (for decode/minify phase alignment).
    if pad_leading {
        let first_kind = kinds.get(&ys[0]).copied().unwrap_or(RowKind::Unknown);
        if let Some(first_phase) = phase_for_kind(first_kind) {
            // If first row is Mid, we missed Green above. If Orange, missed Green+Mid above.
            let missing_before = first_phase;
            for k in (1..=missing_before).rev() {
                let y = ys[0] - dy * (k as i32);
                if in_bounds(y) {
                    all_ys.push(y);
                    synthetic.insert(y);
                }
            }
        }
    }

    // Walk observed rows and insert synthetic gaps only when the phase sequence indicates
    // a missing row between them (and the spacing supports it).
    for window in ys.windows(2) {
        let y0 = window[0];
        let y1 = window[1];
        all_ys.push(y0);

        let diff = y1 - y0;
        if diff <= 0 {
            continue;
        }

        let kind0 = kinds.get(&y0).copied().unwrap_or(RowKind::Unknown);
        let kind1 = kinds.get(&y1).copied().unwrap_or(RowKind::Unknown);
        let phase0 = phase_for_kind(kind0);
        let phase1 = phase_for_kind(kind1);

        let missing_by_phase = match (phase0, phase1) {
            (Some(p0), Some(p1)) => {
                let steps = (p1 + 3 - p0) % 3;
                match steps {
                    1 => 0,
                    2 => 1,
                    _ => 2,
                }
            }
            _ => {
                let steps = ((diff + (dy / 2)) / dy).max(1) as usize;
                steps.saturating_sub(1).min(2)
            }
        };

        let min_required = dy * (missing_by_phase as i32 + 1) - (dy / 2);
        if diff < min_required {
            continue;
        }

        // Place synthetic rows by subdividing the observed gap to avoid drift.
        match missing_by_phase {
            1 => {
                let y = y0 + (diff / 2);
                if in_bounds(y) {
                    all_ys.push(y);
                    synthetic.insert(y);
                }
            }
            2 => {
                let y_a = y0 + (diff / 3);
                let y_b = y0 + ((2 * diff) / 3);
                for y in [y_a, y_b] {
                    if in_bounds(y) {
                        all_ys.push(y);
                        synthetic.insert(y);
                    }
                }
            }
            _ => {}
        }
    }
    // Push last observed row.
    if let Some(&last) = ys.last() {
        all_ys.push(last);
    }

    all_ys.sort_unstable();
    all_ys.dedup();

    (all_ys, synthetic)
}

/// Builds a full intersection grid whose row/column structure matches the plot:
/// sorted unique rounded dot-center Y coordinates by sorted unique rounded X coordinates.
pub fn build_intersection_grid(dots: &[DotDetection], config: &AnotoConfig) -> Vec<Vec<String>> {
    if dots.is_empty() {
        return Vec::new();
    }

    // Use synthetic empty rows/cols so 3x3 minification keeps phase alignment
    // even when an entire Anoto pattern row/column has 0 detected dots.
    let (xs, _) = xs_with_synthetic_cols(dots, config, None, true);
    let (ys, _) = ys_with_synthetic_rows(dots, config, None, true);

    if xs.is_empty() || ys.is_empty() {
        return Vec::new();
    }

    let width = xs.len();
    let height = ys.len();

    let mut grid: Vec<Vec<String>> = vec![vec![" ".to_string(); width]; height];

    for d in dots {
        let Some(ch) = arrow_for_dot(d, config) else {
            continue;
        };
        let x = d.center.0.round() as i32;
        let y = d.center.1.round() as i32;
        let (Ok(cx), Ok(ry)) = (xs.binary_search(&x), ys.binary_search(&y)) else {
            continue;
        };

        // Stable collision handling: keep first non-space.
        if grid[ry][cx] == " " {
            grid[ry][cx] = ch.to_string();
        }
    }

    grid
}

/// Builds an intersection grid using ONLY observed dot-center coordinates.
///
/// This intentionally does NOT insert synthetic empty rows/columns. It is meant
/// for producing a minified preview that reflects visible dots only.
pub fn build_intersection_grid_observed(dots: &[DotDetection], config: &AnotoConfig) -> Vec<Vec<String>> {
    if dots.is_empty() {
        return Vec::new();
    }

    // Cluster near-identical positions into stable row/col centers.
    // This avoids creating sparse extra rows/cols that force minification to over-trim.
    let xs_raw: Vec<f32> = dots.iter().map(|d| d.center.0).collect();
    let ys_raw: Vec<f32> = dots.iter().map(|d| d.center.1).collect();
    let mut xs_centers = cluster_positions_f32(&xs_raw);
    let mut ys_centers = cluster_positions_f32(&ys_raw);
    xs_centers.sort_by(|a, b| a.partial_cmp(b).unwrap());
    ys_centers.sort_by(|a, b| a.partial_cmp(b).unwrap());

    if xs_centers.is_empty() || ys_centers.is_empty() {
        return Vec::new();
    }

    let width = xs_centers.len();
    let height = ys_centers.len();
    let mut grid: Vec<Vec<String>> = vec![vec![" ".to_string(); width]; height];

    for d in dots {
        let Some(ch) = arrow_for_dot(d, config) else {
            continue;
        };
        let (Some(cx), Some(ry)) = (
            nearest_cluster_index(&xs_centers, d.center.0),
            nearest_cluster_index(&ys_centers, d.center.1),
        ) else {
            continue;
        };

        if grid[ry][cx] == " " {
            grid[ry][cx] = ch.to_string();
        }
    }

    grid
}

/// Renders a plot in-memory as an RGBA pixel buffer.
///
/// This matches the CLI "--plot" style: grid lines through every dot coordinate,
/// plus small filled dot markers (no kornia overlay/circles).
pub fn render_plot_rgba(
    width: u32,
    height: u32,
    dots: &[DotDetection],
    config: &AnotoConfig,
) -> Result<Vec<u8>, String> {
    if width == 0 || height == 0 {
        return Ok(Vec::new());
    }

    let pixel_count = (width as usize)
        .checked_mul(height as usize)
        .ok_or_else(|| "width*height overflow".to_string())?;

    let mut rgb = vec![255u8; pixel_count * 3];

    {
        let root = BitMapBackend::with_buffer(&mut rgb, (width, height)).into_drawing_area();
        root.fill(&WHITE).map_err(|e| e.to_string())?;

        // Gridline coordinates are based on rounded dot centers.
        // Also insert up to 2 synthetic empty rows/columns for corner cases where
        // an entire pattern row/column has 0 detected dots.
        let x_bounds = Some((0, width.saturating_sub(1) as i32));
        let y_bounds = Some((0, height.saturating_sub(1) as i32));
        let (xs_all, synthetic_xs) = xs_with_synthetic_cols(dots, config, x_bounds, false);
        let (ys_all, synthetic_ys) = ys_with_synthetic_rows(dots, config, y_bounds, false);

        let grid_color = RGBColor(210, 210, 210);
        let synthetic_row_color = RGBColor(255, 0, 0);
        let synthetic_col_color = RGBColor(255, 0, 0);
        for x in xs_all {
            let x = x.clamp(0, width.saturating_sub(1) as i32);
            let color = if synthetic_xs.contains(&x) {
                synthetic_col_color
            } else {
                grid_color
            };
            root.draw(&PathElement::new(
                [(x, 0), (x, height.saturating_sub(1) as i32)],
                color,
            ))
            .map_err(|e| e.to_string())?;
        }
        for y in ys_all {
            let y = y.clamp(0, height.saturating_sub(1) as i32);
            let color = if synthetic_ys.contains(&y) {
                synthetic_row_color
            } else {
                grid_color
            };
            root.draw(&PathElement::new(
                [(0, y), (width.saturating_sub(1) as i32, y)],
                color,
            ))
            .map_err(|e| e.to_string())?;
        }

        for d in dots {
            let x = (d.center.0.round() as i32).clamp(0, width.saturating_sub(1) as i32);
            let y = (d.center.1.round() as i32).clamp(0, height.saturating_sub(1) as i32);

            let color = if d.type_color == config.color_green {
                rgba_to_plotters(config.color_green)
            } else if d.type_color == config.color_orange {
                rgba_to_plotters(config.color_orange)
            } else if d.type_color == config.color_blue {
                rgba_to_plotters(config.color_blue)
            } else if d.type_color == config.color_magenta {
                rgba_to_plotters(config.color_magenta)
            } else {
                rgba_to_plotters(d.type_color)
            };

            root.draw(&Circle::new((x, y), 4, color.filled()))
                .map_err(|e| e.to_string())?;
        }

        root.present().map_err(|e| e.to_string())?;
    }

    let mut rgba = vec![255u8; pixel_count * 4];
    for i in 0..pixel_count {
        rgba[i * 4] = rgb[i * 3];
        rgba[i * 4 + 1] = rgb[i * 3 + 1];
        rgba[i * 4 + 2] = rgb[i * 3 + 2];
        rgba[i * 4 + 3] = 255;
    }

    Ok(rgba)
}
