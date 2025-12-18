use plotters::prelude::*;

use crate::kornia::anoto::{AnotoConfig, DotDetection};

fn rgba_to_plotters(c: image::Rgba<u8>) -> RGBColor {
    RGBColor(c[0], c[1], c[2])
}

fn arrow_for_dot(dot: &DotDetection, config: &AnotoConfig) -> Option<char> {
    if dot.type_color == config.color_green {
        Some('↑')
    } else if dot.type_color == config.color_orange {
        Some('↓')
    } else if dot.type_color == config.color_blue {
        Some('←')
    } else if dot.type_color == config.color_magenta {
        Some('→')
    } else {
        None
    }
}

/// Builds a full intersection grid whose row/column structure matches the plot:
/// sorted unique rounded dot-center Y coordinates by sorted unique rounded X coordinates.
pub fn build_intersection_grid(dots: &[DotDetection], config: &AnotoConfig) -> Vec<Vec<String>> {
    if dots.is_empty() {
        return Vec::new();
    }

    let mut xs: Vec<i32> = dots.iter().map(|d| d.center.0.round() as i32).collect();
    let mut ys: Vec<i32> = dots.iter().map(|d| d.center.1.round() as i32).collect();
    xs.sort_unstable();
    xs.dedup();
    ys.sort_unstable();
    ys.dedup();

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

        // Gridline coordinates are the same rounded dot centers used for the grid JSON.
        let mut xs: Vec<i32> = dots.iter().map(|d| d.center.0.round() as i32).collect();
        let mut ys: Vec<i32> = dots.iter().map(|d| d.center.1.round() as i32).collect();
        xs.sort_unstable();
        xs.dedup();
        ys.sort_unstable();
        ys.dedup();

        let grid_color = RGBColor(210, 210, 210);
        for x in xs {
            let x = x.clamp(0, width.saturating_sub(1) as i32);
            root.draw(&PathElement::new(
                [(x, 0), (x, height.saturating_sub(1) as i32)],
                grid_color,
            ))
            .map_err(|e| e.to_string())?;
        }
        for y in ys {
            let y = y.clamp(0, height.saturating_sub(1) as i32);
            root.draw(&PathElement::new(
                [(0, y), (width.saturating_sub(1) as i32, y)],
                grid_color,
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
