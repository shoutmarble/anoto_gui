use clap::{ArgGroup, Parser};
use image::GenericImageView;
use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use anoto_dot_reader::kornia::anoto::{detect_components_from_image, AnotoConfig};
use anoto_dot_reader::minify_arrow_grid::{minify_from_full_grid, write_grid_json_string};
use anoto_dot_reader::plot_grid::{build_intersection_grid, render_plot_rgba};

#[derive(Parser, Debug)]
#[command(
    name = "anoto",
    about = "Generate plot, grid JSON, and minified arrow-grid JSON from images",
    version,
    group(
        ArgGroup::new("action")
            .required(true)
            .multiple(true)
            .args(["plot", "grid", "minified"])
    )
)]
struct Cli {
    /// Directory containing input images
    #[arg(short = 'd', long = "dir")]
    dir: PathBuf,

    /// Generate scatter/annotated plot PNGs
    #[arg(long = "plot", short = 'p', alias = "p")]
    plot: bool,

    /// Generate JSON grid of every intersection
    #[arg(long = "grid", short = 'g', alias = "g")]
    grid: bool,

    /// Generate minified JSON of all arrows
    #[arg(long = "minified", short = 'm', alias = "m")]
    minified: bool,
}

fn is_image_file(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(OsStr::to_str) else {
        return false;
    };
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "png" | "jpg" | "jpeg" | "bmp" | "gif" | "tif" | "tiff" | "webp"
    )
}

fn write_text_file(path: &Path, contents: &str) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)?;
    Ok(())
}

fn grid_json_or_empty(grid: &[Vec<String>]) -> String {
    if grid.is_empty() {
        "[]".to_string()
    } else {
        write_grid_json_string(grid)
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    if !cli.dir.is_dir() {
        return Err(format!("Not a directory: {}", cli.dir.display()).into());
    }

    let mut images: Vec<PathBuf> = fs::read_dir(&cli.dir)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file() && is_image_file(p))
        .collect();

    images.sort();

    if images.is_empty() {
        eprintln!("No images found in {}", cli.dir.display());
        return Ok(());
    }

    let config = AnotoConfig::default();

    for (i, image_path) in images.iter().enumerate() {
        let img = match image::open(image_path) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Failed to open {}: {e}", image_path.display());
                continue;
            }
        };

        let components = match detect_components_from_image(&img, &config) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Detection failed for {}: {e}", image_path.display());
                continue;
            }
        };

        if cli.plot {
            let out_plot = PathBuf::from(format!("anoto_{i}_plot.png"));
            let (w, h) = img.dimensions();
            match render_plot_rgba(w, h, &components, &config) {
                Ok(pixels) if !pixels.is_empty() => {
                    if let Some(rgba) = image::RgbaImage::from_raw(w, h, pixels) {
                        if let Err(e) = rgba.save(&out_plot) {
                            eprintln!(
                                "Failed to save plot {} for {}: {e}",
                                out_plot.display(),
                                image_path.display()
                            );
                        }
                    } else {
                        eprintln!(
                            "Failed to build RGBA image for plot {} ({w}x{h})",
                            out_plot.display()
                        );
                    }
                }
                Ok(_) => {
                    eprintln!("Plot skipped (empty image) for {}", image_path.display());
                }
                Err(e) => {
                    eprintln!(
                        "Failed to render plot {} for {}: {e}",
                        out_plot.display(),
                        image_path.display()
                    );
                }
            }
        }

        if !cli.grid && !cli.minified {
            continue;
        }

        // The grid output should match the structure used by --plot: the unique
        // x/y dot coordinates define the full intersection grid.
        let full_grid = build_intersection_grid(&components, &config);

        if cli.grid {
            let out_grid = PathBuf::from(format!("anoto_{i}_grid.json"));
            let s = grid_json_or_empty(&full_grid);
            if let Err(e) = write_text_file(&out_grid, &s) {
                eprintln!(
                    "Failed to write grid {} for {}: {e}",
                    out_grid.display(),
                    image_path.display()
                );
            }
        }

        if cli.minified {
            let out_minified = PathBuf::from(format!("anoto_{i}_minified.json"));
            let minified = minify_from_full_grid(&full_grid);
            let s = grid_json_or_empty(&minified);
            if let Err(e) = write_text_file(&out_minified, &s) {
                eprintln!(
                    "Failed to write minified {} for {}: {e}",
                    out_minified.display(),
                    image_path.display()
                );
            }
        }
    }

    Ok(())
}
