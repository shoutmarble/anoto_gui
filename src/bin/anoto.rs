use clap::{ArgGroup, Parser};
use image::GenericImageView;
use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use anoto_dot_reader::anoto_decode::{
    decode_all_windows_from_minified_arrows, extract_best_decodable_window_from_minified_arrows,
};
use anoto_dot_reader::kornia::anoto::{detect_components_from_image, AnotoConfig};
use anoto_dot_reader::minify_arrow_grid::{minify_from_full_grid, write_grid_json_string};
use anoto_dot_reader::plot_grid::{
    build_intersection_grid, build_intersection_grid_observed, render_plot_rgba,
};

#[derive(Parser, Debug)]
#[command(
    name = "anoto",
    about = "Generate plot, grid JSON, and minified arrow-grid JSON from images",
    version,
    group(
        ArgGroup::new("action")
            .required(true)
            .multiple(true)
            .args(["plot", "grid", "anoto", "verify", "verify_json", "find_pattern", "generate"])
    )
)]
struct Cli {
    /// Generate an Anoto arrow grid and write it as pretty-printed JSON.
    ///
    /// Usage: `--generate <WIDTH> <HEIGHT> [U] [V]`
    ///
    /// If U,V are omitted, defaults to 10 10.
    #[arg(long = "generate", value_name = "N", num_args = 2..=4)]
    generate: Option<Vec<i32>>,

    /// Image file or directory containing input images.
    ///
    /// If a directory is provided, it is scanned (non-recursively) for images.
    #[arg(
        short = 'i',
        long = "input",
        value_name = "PATH",
        required_unless_present = "generate"
    )]
    input: Option<PathBuf>,

    /// (Deprecated) Directory containing input images. Use --input.
    #[arg(short = 'd', long = "dir", value_name = "DIR", hide = true)]
    dir: Option<PathBuf>,

    /// Output directory for generated files.
    ///
    /// Defaults to `READER`.
    #[arg(long = "out-dir", value_name = "DIR", default_value = "READER")]
    out_dir: PathBuf,

    /// Generate scatter/annotated plot PNGs.
    ///
    /// Note: `--plot` also writes the arrow-grid JSON (same as `--anoto`).
    #[arg(long = "plot", short = 'p', alias = "p")]
    plot: bool,

    /// Generate JSON grid of every intersection
    #[arg(long = "grid", short = 'g', alias = "g")]
    grid: bool,

    /// Generate Anoto arrow-grid JSON (written as `...__ANOTO.json`)
    #[arg(long = "anoto", short = 'm', alias = "m")]
    anoto: bool,

    /// Verify that the detected (full) minified arrow grid matches the expected JSON file exactly.
    ///
    /// This is intended for validating that an image (e.g. GUI_G__...__Y.png) matches the
    /// corresponding arrow-grid JSON (e.g. GUI_G__...__X.json) for the same Anoto section.
    #[arg(long = "verify-json", value_name = "EXPECTED_JSON")]
    verify_json: Option<PathBuf>,

    /// Decode all valid 6x6 windows and write the decoded (x,y) coordinates as pretty-printed JSON.
    ///
    /// This also writes the arrow-grid JSON (`...__ANOTO.json`).
    #[arg(long = "verify")]
    verify: bool,

    /// Search for the built-in 8x7 Anoto pattern snippet in each image and report the first match.
    #[arg(long = "find-pattern")]
    find_pattern: bool,

    /// Decode all valid 6x6 windows and print a short summary per image.
    #[arg(long = "decode")]
    decode: bool,

    /// Write full outputs (no 8x8 cropping).
    ///
    /// Without `--full`, `--plot/--grid/--anoto` will write at most an 8x8 arrow window.
    #[arg(long = "full")]
    full: bool,
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

fn input_path(cli: &Cli) -> Result<&Path, Box<dyn Error>> {
    if let Some(dir) = cli.dir.as_deref() {
        return Ok(dir);
    }
    if let Some(input) = cli.input.as_deref() {
        return Ok(input);
    }
    Err("missing --input (or use --generate)".into())
}

fn write_pretty_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), Box<dyn Error>> {
    let s = serde_json::to_string_pretty(value)?;
    write_text_file(path, &s)
}

fn write_verify_rows_compact(path: &Path, rows: &[Vec<[i32; 2]>]) -> Result<(), Box<dyn Error>> {
    // Format as valid JSON with each decoded y-row on one line, e.g.
    // [
    //   [[40,0],[41,0]],
    //   [[40,1],[41,1]]
    // ]
    let mut out = String::new();
    out.push_str("[\n");
    for (ri, row) in rows.iter().enumerate() {
        out.push_str("  [");
        for (ci, [x, y]) in row.iter().copied().enumerate() {
            if ci > 0 {
                out.push(',');
            }
            out.push_str(&format!("[{x},{y}]"));
        }
        out.push(']');
        if ri + 1 < rows.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push(']');
    out.push('\n');
    write_text_file(path, &out)
}

fn decoded_xy_rows_pretty(minified: &[Vec<String>]) -> Vec<Vec<[i32; 2]>> {
    use std::collections::{BTreeMap, BTreeSet};

    // Group decoded coordinates into rows by y, with x sorted within each row.
    let mut rows = BTreeMap::<i32, BTreeSet<i32>>::new();
    for d in decode_all_windows_from_minified_arrows(minified) {
        rows.entry(d.y).or_default().insert(d.x);
    }

    rows.into_iter()
        .map(|(y, xs)| xs.into_iter().map(|x| [x, y]).collect::<Vec<[i32; 2]>>())
        .collect()
}

fn parse_generate_args(args: &[i32]) -> Result<(usize, usize, i32, i32), Box<dyn Error>> {
    if !(2..=4).contains(&args.len()) {
        return Err("--generate expects 2 to 4 integers".into());
    }
    let w = args[0];
    let h = args[1];
    if w <= 0 || h <= 0 {
        return Err("--generate WIDTH and HEIGHT must be > 0".into());
    }
    let (u, v) = if args.len() >= 4 {
        (args[2], args[3])
    } else {
        (10, 10)
    };
    Ok((w as usize, h as usize, u, v))
}

fn generate_grid_from_assets(
    w: usize,
    h: usize,
    u: i32,
    v: i32,
) -> Result<Vec<Vec<String>>, Box<dyn Error>> {
    let candidate = PathBuf::from("assets").join(format!("GUI_G__{w}__{h}__{u}__{v}__X.json"));
    if !candidate.is_file() {
        return Err(format!(
            "No built-in generator for section (u={u}, v={v}) with size {w}x{h}. Expected asset not found: {}",
            candidate.display()
        )
        .into());
    }
    read_json_grid(&candidate)
}

fn collect_images(input: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    if input.is_file() {
        if is_image_file(input) {
            return Ok(vec![input.to_path_buf()]);
        }
        return Err(format!("Not an image file: {}", input.display()).into());
    }
    if !input.is_dir() {
        return Err(format!("Not a file or directory: {}", input.display()).into());
    }

    let mut images: Vec<PathBuf> = fs::read_dir(input)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file() && is_image_file(p))
        .collect();
    images.sort();
    Ok(images)
}

fn read_json_grid(path: &Path) -> Result<Vec<Vec<String>>, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let grid: Vec<Vec<String>> = serde_json::from_slice(&bytes)?;
    Ok(grid)
}

fn first_grid_mismatch(a: &[Vec<String>], b: &[Vec<String>]) -> Option<(usize, usize, String, String)> {
    if a.len() != b.len() {
        return Some((0, 0, format!("height {}", a.len()), format!("height {}", b.len())));
    }
    for (r, (ra, rb)) in a.iter().zip(b.iter()).enumerate() {
        if ra.len() != rb.len() {
            return Some((r, 0, format!("width {}", ra.len()), format!("width {}", rb.len())));
        }
        for c in 0..ra.len() {
            if ra[c] != rb[c] {
                return Some((r, c, ra[c].clone(), rb[c].clone()));
            }
        }
    }
    None
}

fn grid_dims(grid: &[Vec<String>]) -> Option<(usize, usize)> {
    let h = grid.len();
    let w = grid.first().map(|r| r.len())?;
    if w == 0 || !grid.iter().all(|r| r.len() == w) {
        return None;
    }
    Some((h, w))
}

fn infer_section_from_path(path: &Path) -> Option<(usize, usize, i32, i32)> {
    // Accept patterns like:
    // - GUI_G__81__56__10__10__Y.png
    // - GUI_G__81__56__10__10__X.json
    // - R__81__56__10__10__GRID.json
    // We only care about the first 4 numeric fields after the prefix.
    let name = path.file_name()?.to_string_lossy();
    let parts: Vec<&str> = name.split("__").collect();
    if parts.len() < 5 {
        return None;
    }
    let prefix = parts[0];
    if prefix != "GUI_G" && prefix != "R" {
        return None;
    }
    let w: usize = parts.get(1)?.parse().ok()?;
    let h: usize = parts.get(2)?.parse().ok()?;
    let u: i32 = parts.get(3)?.parse().ok()?;
    let v: i32 = parts.get(4)?.parse().ok()?;
    Some((w, h, u, v))
}

fn output_base_name(
    image_path: &Path,
    expected_path_for_verify: Option<&Path>,
    minified_full: Option<&[Vec<String>]>,
) -> String {
    // Prefer explicit metadata from verify-json (if present), then the input filename.
    let mut meta = expected_path_for_verify
        .and_then(infer_section_from_path)
        .or_else(|| infer_section_from_path(image_path));

    if meta.is_none() {
        if let Some(grid) = minified_full {
            if let Some((rows, cols)) = grid_dims(grid) {
                meta = Some((rows, cols, 10, 10));
            }
        }
    }

    let (w, h, u, v) = meta.unwrap_or((0, 0, 10, 10));
    format!("R__{w}__{h}__{u}__{v}")
}

fn arrow_count(grid: &[Vec<String>]) -> usize {
    grid.iter()
        .flat_map(|r| r.iter())
        .filter(|v| is_arrow_cell(v.as_str()))
        .count()
}

fn patch_matches_expected(
    observed_minified: &[Vec<String>],
    obs_row: usize,
    obs_col: usize,
    expected: &[Vec<String>],
    exp_x: i32,
    exp_y: i32,
) -> bool {
    if exp_x < 0 || exp_y < 0 {
        return false;
    }
    let (eh, ew) = match grid_dims(expected) {
        Some(v) => v,
        None => return false,
    };
    let (oh, ow) = match grid_dims(observed_minified) {
        Some(v) => v,
        None => return false,
    };

    let exp_x = exp_x as usize;
    let exp_y = exp_y as usize;
    if obs_row + 6 > oh || obs_col + 6 > ow {
        return false;
    }
    if exp_y + 6 > eh || exp_x + 6 > ew {
        return false;
    }

    for r in 0..6 {
        for c in 0..6 {
            let a = &observed_minified[obs_row + r][obs_col + c];
            let b = &expected[exp_y + r][exp_x + c];
            if a != b {
                return false;
            }
        }
    }
    true
}

fn is_arrow_cell(s: &str) -> bool {
    matches!(s, "↑" | "↓" | "←" | "→")
}

fn best_align_against_expected(
    observed_minified: &[Vec<String>],
    expected: &[Vec<String>],
) -> Option<(usize, usize, usize, usize)> {
    // Returns (best_y, best_x, matches, considered)
    let (oh, ow) = grid_dims(observed_minified)?;
    let (eh, ew) = grid_dims(expected)?;
    if oh == 0 || ow == 0 || eh < oh || ew < ow {
        return None;
    }

    let mut best: Option<(usize, usize, usize, usize)> = None;
    for top in 0..=(eh - oh) {
        for left in 0..=(ew - ow) {
            let mut considered = 0usize;
            let mut matches = 0usize;
            for r in 0..oh {
                for c in 0..ow {
                    let cell = observed_minified[r][c].as_str();
                    if !is_arrow_cell(cell) {
                        continue;
                    }
                    considered += 1;
                    if cell == expected[top + r][left + c].as_str() {
                        matches += 1;
                    }
                }
            }

            if considered == 0 {
                continue;
            }

            let better = best
                .as_ref()
                .map_or(true, |(_, _, bm, bc)| matches > *bm || (matches == *bm && considered > *bc));
            if better {
                best = Some((top, left, matches, considered));
            }
        }
    }
    best
}

fn best_align_against_expected_with_xforms_and_perms(
    observed_minified: &[Vec<String>],
    expected: &[Vec<String>],
) -> Option<(GridXform, [String; 4], usize, usize, usize, usize)> {
    // Returns (xform, perm, best_y, best_x, matches, considered)
    if observed_minified.is_empty() || observed_minified[0].is_empty() {
        return None;
    }
    let w = observed_minified[0].len();
    if !observed_minified.iter().all(|r| r.len() == w) {
        return None;
    }

    let xforms = [
        GridXform::Identity,
        GridXform::Rot90,
        GridXform::Rot180,
        GridXform::Rot270,
        GridXform::FlipH,
        GridXform::FlipV,
        GridXform::Transpose,
        GridXform::AntiTranspose,
    ];
    let perms = all_arrow_perms();

    let mut best: Option<(GridXform, [String; 4], usize, usize, usize, usize)> = None;

    for &xf in &xforms {
        let g1 = apply_xform(observed_minified, xf);
        for perm in &perms {
            let g2 = apply_arrow_perm_str(&g1, perm);
            if let Some((top, left, m, c)) = best_align_against_expected(&g2, expected) {
                let better = best.as_ref().map_or(true, |(_, _, _, _, bm, bc)| {
                    m > *bm || (m == *bm && c > *bc)
                });
                if better {
                    best = Some((xf, perm.clone(), top, left, m, c));
                }
            }
        }
    }
    best
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GridBuilder {
    Observed,
    Phase,
}

fn decode_score(minified: &[Vec<String>]) -> usize {
    decode_all_windows_from_minified_arrows(minified).len()
}

fn select_best_minified_for_decode(
    minified_observed: &[Vec<String>],
    minified_phase: &[Vec<String>],
) -> (GridBuilder, Vec<Vec<String>>, usize) {
    let so = decode_score(minified_observed);
    let sp = decode_score(minified_phase);
    if so >= sp {
        (GridBuilder::Observed, minified_observed.to_vec(), so)
    } else {
        (GridBuilder::Phase, minified_phase.to_vec(), sp)
    }
}

fn builtin_expected_pattern_8x7() -> Vec<Vec<String>> {
    vec![
        vec!["↑", "↓", "←", "↓", "↓", "←", "↓"],
        vec!["→", "↑", "↑", "←", "↑", "↓", "↓"],
        vec!["↓", "↓", "→", "↓", "↑", "→", "←"],
        vec!["↓", "←", "↑", "←", "↓", "↑", "→"],
        vec!["←", "→", "↓", "←", "→", "→", "↑"],
        vec!["↓", "↑", "→", "↑", "→", "↓", "↓"],
        vec!["↓", "↓", "←", "←", "→", "←", "→"],
        vec!["↑", "↓", "↓", "→", "→", "→", "←"],
    ]
    .into_iter()
    .map(|row| row.into_iter().map(|s| s.to_string()).collect())
    .collect()
}

#[derive(Clone, Copy, Debug)]
enum GridXform {
    Identity,
    Rot90,
    Rot180,
    Rot270,
    FlipH,
    FlipV,
    Transpose,
    AntiTranspose,
}

fn apply_xform(g: &[Vec<String>], t: GridXform) -> Vec<Vec<String>> {
    let h = g.len();
    let w = g[0].len();
    match t {
        GridXform::Identity => g.to_vec(),
        GridXform::Rot90 => {
            let mut out = vec![vec![" ".to_string(); h]; w];
            for r in 0..h {
                for c in 0..w {
                    out[c][h - 1 - r] = g[r][c].clone();
                }
            }
            out
        }
        GridXform::Rot180 => {
            let mut out = vec![vec![" ".to_string(); w]; h];
            for r in 0..h {
                for c in 0..w {
                    out[h - 1 - r][w - 1 - c] = g[r][c].clone();
                }
            }
            out
        }
        GridXform::Rot270 => {
            let mut out = vec![vec![" ".to_string(); h]; w];
            for r in 0..h {
                for c in 0..w {
                    out[w - 1 - c][r] = g[r][c].clone();
                }
            }
            out
        }
        GridXform::FlipH => {
            let mut out = g.to_vec();
            for r in 0..h {
                out[r].reverse();
            }
            out
        }
        GridXform::FlipV => {
            let mut out = g.to_vec();
            out.reverse();
            out
        }
        GridXform::Transpose => {
            let mut out = vec![vec![" ".to_string(); h]; w];
            for r in 0..h {
                for c in 0..w {
                    out[c][r] = g[r][c].clone();
                }
            }
            out
        }
        GridXform::AntiTranspose => {
            let mut out = vec![vec![" ".to_string(); h]; w];
            for r in 0..h {
                for c in 0..w {
                    out[w - 1 - c][h - 1 - r] = g[r][c].clone();
                }
            }
            out
        }
    }
}

fn all_arrow_perms() -> Vec<[String; 4]> {
    let base = ["↑", "↓", "←", "→"];
    let mut perms = Vec::new();
    for &a in &base {
        for &b in &base {
            if b == a {
                continue;
            }
            for &c in &base {
                if c == a || c == b {
                    continue;
                }
                for &d in &base {
                    if d == a || d == b || d == c {
                        continue;
                    }
                    perms.push([
                        a.to_string(),
                        b.to_string(),
                        c.to_string(),
                        d.to_string(),
                    ]);
                }
            }
        }
    }
    perms
}

fn apply_arrow_perm_str(g: &[Vec<String>], perm: &[String; 4]) -> Vec<Vec<String>> {
    g.iter()
        .map(|row| {
            row.iter()
                .map(|cell| match cell.as_str() {
                    "↑" => perm[0].clone(),
                    "↓" => perm[1].clone(),
                    "←" => perm[2].clone(),
                    "→" => perm[3].clone(),
                    other => other.to_string(),
                })
                .collect::<Vec<String>>()
        })
        .collect()
}

fn find_pattern_strict(grid: &[Vec<String>], pattern: &[Vec<String>]) -> Option<(usize, usize)> {
    if grid.is_empty() || pattern.is_empty() {
        return None;
    }
    let gh = grid.len();
    let gw = grid[0].len();
    if gw == 0 || !grid.iter().all(|r| r.len() == gw) {
        return None;
    }
    let ph = pattern.len();
    let pw = pattern[0].len();
    if pw == 0 || !pattern.iter().all(|r| r.len() == pw) {
        return None;
    }
    if gh < ph || gw < pw {
        return None;
    }

    for top in 0..=(gh - ph) {
        for left in 0..=(gw - pw) {
            let mut ok = true;
            'outer: for r in 0..ph {
                for c in 0..pw {
                    let cell = &grid[top + r][left + c];
                    if cell.trim().is_empty() {
                        ok = false;
                        break 'outer;
                    }
                    if cell != &pattern[r][c] {
                        ok = false;
                        break 'outer;
                    }
                }
            }
            if ok {
                return Some((top, left));
            }
        }
    }
    None
}

fn find_pattern_with_xforms_and_perms(
    grid: &[Vec<String>],
    pattern: &[Vec<String>],
) -> Option<(usize, usize, GridXform, [String; 4])> {
    if grid.is_empty() || grid[0].is_empty() {
        return None;
    }
    let w = grid[0].len();
    if !grid.iter().all(|r| r.len() == w) {
        return None;
    }

    let xforms = [
        GridXform::Identity,
        GridXform::Rot90,
        GridXform::Rot180,
        GridXform::Rot270,
        GridXform::FlipH,
        GridXform::FlipV,
        GridXform::Transpose,
        GridXform::AntiTranspose,
    ];
    let perms = all_arrow_perms();

    for &xf in &xforms {
        let g1 = apply_xform(grid, xf);
        for perm in &perms {
            let g2 = apply_arrow_perm_str(&g1, perm);
            if let Some((row, col)) = find_pattern_strict(&g2, pattern) {
                return Some((row, col, xf, perm.clone()));
            }
        }
    }
    None
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    // UX: If you ask for a plot, also write the arrow-grid JSON.
    let write_minified = cli.anoto || cli.plot || cli.verify;
    let full_output = cli.full;

    if !cli.out_dir.as_os_str().is_empty() {
        fs::create_dir_all(&cli.out_dir)?;
    }

    if let Some(args) = cli.generate.as_deref() {
        let (w, h, u, v) = parse_generate_args(args)?;
        let grid = generate_grid_from_assets(w, h, u, v)?;
        let out_name = format!("R__{w}__{h}__{u}__{v}__GRID.json");
        let out_path = cli.out_dir.join(out_name);
        write_pretty_json(&out_path, &grid)?;
        println!("wrote {}", out_path.display());
        return Ok(());
    }

    let input = input_path(&cli)?;
    let images = collect_images(input)?;

    if images.is_empty() {
        eprintln!("No images found in {}", input.display());
        return Ok(());
    }

    let config = AnotoConfig::default();

    let expected_for_verify = if let Some(p) = cli.verify_json.as_deref() {
        Some((p.to_path_buf(), read_json_grid(p)?))
    } else {
        None
    };

    let expected_pattern = if cli.find_pattern {
        Some(builtin_expected_pattern_8x7())
    } else {
        None
    };

    for image_path in images.iter() {
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

        // Build both grid variants ("phase" and "observed") and choose the best one
        // for decoding/search/verification, similar to match_expected.rs.
        let full_grid_observed = build_intersection_grid_observed(&components, &config);
        let full_grid_phase = build_intersection_grid(&components, &config);

        let need_minified_full = write_minified || cli.verify_json.is_some() || cli.find_pattern || cli.decode;
        let (minified_observed, minified_phase, best_builder, minified_best, best_score) =
            if need_minified_full {
                let mo = minify_from_full_grid(&full_grid_observed);
                let mp = minify_from_full_grid(&full_grid_phase);
                let (bb, mb, bs) = select_best_minified_for_decode(&mo, &mp);
                (mo, mp, bb, mb, bs)
            } else {
                (Vec::new(), Vec::new(), GridBuilder::Observed, Vec::new(), 0)
            };

        // Choose a "best" full minified grid for naming and writing:
        // - Prefer the variant with the best arrow coverage (fewest blanks).
        // - Tie-break with decode score.
        // This is important for `--full --anoto` on clean crops, where one builder
        // can yield a complete arrow grid while the other has gaps.
        let minified_for_writes_full = if need_minified_full {
            let ao = arrow_count(&minified_observed);
            let ap = arrow_count(&minified_phase);
            if ap > ao {
                minified_phase.clone()
            } else if ao > ap {
                minified_observed.clone()
            } else {
                let so = decode_score(&minified_observed);
                let sp = decode_score(&minified_phase);
                if sp > so {
                    minified_phase.clone()
                } else {
                    minified_observed.clone()
                }
            }
        } else {
            Vec::new()
        };

        let base = output_base_name(
            image_path,
            expected_for_verify.as_ref().map(|(p, _)| p.as_path()),
            if need_minified_full {
                Some(&minified_for_writes_full)
            } else {
                None
            },
        );

        if cli.plot {
            let out_plot = cli.out_dir.join(format!("{base}__PLOT.png"));
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

        if !cli.grid && !cli.anoto {
            // Still allow verify/find-pattern without writing JSON files.
            // (The action group enforces at least one action overall.)
        }

        if cli.decode {
            let got = &minified_best;
            let decoded = decode_all_windows_from_minified_arrows(got);
            if let Some(first) = decoded.first() {
                println!(
                    "DECODE: {} windows; first window(row={}, col={}) -> (x={}, y={}) for {}",
                    decoded.len(),
                    first.window_row,
                    first.window_col,
                    first.x,
                    first.y,
                    image_path.display()
                );
                println!("  grid={best_builder:?} decode_score={best_score}");
            } else {
                println!("DECODE: 0 windows for {}", image_path.display());
                println!("  grid={best_builder:?} decode_score={best_score}");
            }
        }

        if let Some((expected_path, expected_grid)) = expected_for_verify.as_ref() {
            let got = &minified_best;

            // If the grids are the same shape, do an exact full-grid comparison.
            let same_shape = grid_dims(got).is_some()
                && grid_dims(expected_grid).is_some()
                && grid_dims(got) == grid_dims(expected_grid);
            if same_shape {
                // Try both grid builders for exact match, prefer observed if both match.
                let mismatch_obs = first_grid_mismatch(&minified_observed, expected_grid);
                let mismatch_phase = first_grid_mismatch(&minified_phase, expected_grid);

                if mismatch_obs.is_none() {
                    println!("VERIFY OK: {} matches {}", image_path.display(), expected_path.display());
                    println!("  grid=Observed");
                } else if mismatch_phase.is_none() {
                    println!("VERIFY OK: {} matches {}", image_path.display(), expected_path.display());
                    println!("  grid=Phase");
                } else {
                    println!("VERIFY FAIL: {} does not match {}", image_path.display(), expected_path.display());
                    // Report the first mismatch from the currently selected best grid.
                    if let Some((r, c, ga, eb)) = first_grid_mismatch(got, expected_grid) {
                        println!("  first mismatch at (row={r}, col={c}): got={ga:?} expected={eb:?}");
                    }
                    println!("  grid={best_builder:?} decode_score={best_score}");
                }
            } else {
                // For smaller crops, verify by decoding 6x6 windows and ensuring they map to matching
                // patches inside the expected full section grid.
                let decoded_obs = decode_all_windows_from_minified_arrows(&minified_observed);
                let decoded_phase = decode_all_windows_from_minified_arrows(&minified_phase);

                let mut matched_obs = 0usize;
                for d in &decoded_obs {
                    if patch_matches_expected(
                        &minified_observed,
                        d.window_row,
                        d.window_col,
                        expected_grid,
                        d.x,
                        d.y,
                    ) {
                        matched_obs += 1;
                    }
                }
                let mut matched_phase = 0usize;
                for d in &decoded_phase {
                    if patch_matches_expected(
                        &minified_phase,
                        d.window_row,
                        d.window_col,
                        expected_grid,
                        d.x,
                        d.y,
                    ) {
                        matched_phase += 1;
                    }
                }

                // Choose the better verifier result.
                let (grid_name, decoded_len, matched_len, grid_ref) = if matched_obs >= matched_phase {
                    ("Observed", decoded_obs.len(), matched_obs, &minified_observed)
                } else {
                    ("Phase", decoded_phase.len(), matched_phase, &minified_phase)
                };

                println!(
                    "VERIFY SECTION: {} matched {matched_len}/{decoded_len} decoded windows against {}",
                    image_path.display(),
                    expected_path.display()
                );
                println!("  grid={grid_name}");

                if decoded_len == 0 {
                    // Fallback: try locating the crop by alignment against the expected grid (try both grids).
                    let a_obs = best_align_against_expected_with_xforms_and_perms(&minified_observed, expected_grid);
                    let a_phase = best_align_against_expected_with_xforms_and_perms(&minified_phase, expected_grid);

                    let pick = match (a_obs, a_phase) {
                        (Some(o), Some(p)) => {
                            let ro = (o.4 as f64) / (o.5 as f64);
                            let rp = (p.4 as f64) / (p.5 as f64);
                            if rp > ro { ("Phase", p) } else { ("Observed", o) }
                        }
                        (Some(o), None) => ("Observed", o),
                        (None, Some(p)) => ("Phase", p),
                        (None, None) => {
                            println!("  note: no decodable 6x6 windows found in this image");
                            continue;
                        }
                    };

                    let (picked_grid, (xf, perm, top, left, m, c)) = pick;
                    let ratio = (m as f64) / (c as f64);
                    println!("  no 6x6 decodes; best alignment grid={picked_grid} at (y={top}, x={left}) score={m}/{c} ({ratio:.3})");
                    println!("    xform={xf:?} perm[↑,↓,←,→]->{:?}", perm);
                } else if matched_len == 0 {
                    // No decoded patch matched; try alignment to give a hint.
                    if let Some((xf, perm, top, left, m, c)) =
                        best_align_against_expected_with_xforms_and_perms(grid_ref, expected_grid)
                    {
                        let ratio = (m as f64) / (c as f64);
                        println!("  note: 0 decoded patches matched; best alignment at (y={top}, x={left}) score={m}/{c} ({ratio:.3})");
                        println!("    xform={xf:?} perm[↑,↓,←,→]->{:?}", perm);
                    }
                }
            }
        }

        if let Some(ref pat) = expected_pattern {
            let got = &minified_best;
            if let Some((row, col, xf, perm)) = find_pattern_with_xforms_and_perms(got, pat) {
                println!("PATTERN FOUND: {} at (row={row}, col={col})", image_path.display());
                println!("  xform={xf:?} perm[↑,↓,←,→]->{:?}", perm);
            } else {
                println!("PATTERN NOT FOUND: {}", image_path.display());
            }
        }

        if cli.grid {
            let out_grid = cli.out_dir.join(format!("{base}__GRID.json"));
            // Preserve previous behavior: grid JSON comes from the observed builder.
            let s = grid_json_or_empty(&full_grid_observed);
            if let Err(e) = write_text_file(&out_grid, &s) {
                eprintln!(
                    "Failed to write grid {} for {}: {e}",
                    out_grid.display(),
                    image_path.display()
                );
            }
        }

        if write_minified {
            let out_anoto = cli.out_dir.join(format!("{base}__ANOTO.json"));
            let mut minified_written = minified_for_writes_full.clone();
            if !full_output {
                if let Some(window) = extract_best_decodable_window_from_minified_arrows(&minified_written, 8, 8) {
                    minified_written = window;
                }
            }
            let s = grid_json_or_empty(&minified_written);
            if let Err(e) = write_text_file(&out_anoto, &s) {
                eprintln!(
                    "Failed to write minified {} for {}: {e}",
                    out_anoto.display(),
                    image_path.display()
                );
            }

            if cli.verify {
                let out_verify = cli.out_dir.join(format!("{base}__VERIFY.json"));
                let xy_rows = decoded_xy_rows_pretty(&minified_written);
                if let Err(e) = write_verify_rows_compact(&out_verify, &xy_rows) {
                    eprintln!(
                        "Failed to write verify {} for {}: {e}",
                        out_verify.display(),
                        image_path.display()
                    );
                }
            }
        }
    }

    Ok(())
}
