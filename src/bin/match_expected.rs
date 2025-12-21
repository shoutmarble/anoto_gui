use std::path::{Path, PathBuf};

use anoto_dot_reader::kornia::anoto::{detect_components_from_image, AnotoConfig};
use anoto_dot_reader::anoto_decode::decode_all_windows_from_minified_arrows;
use anoto_dot_reader::minify_arrow_grid::{minify_from_full_grid, write_grid_json_string};
use anoto_dot_reader::plot_grid::{build_intersection_grid, build_intersection_grid_observed};

fn expected() -> Vec<Vec<char>> {
    vec![
        vec!['↑', '↓', '←', '↓', '↓', '←', '↓'],
        vec!['→', '↑', '↑', '←', '↑', '↓', '↓'],
        vec!['↓', '↓', '→', '↓', '↑', '→', '←'],
        vec!['↓', '←', '↑', '←', '↓', '↑', '→'],
        vec!['←', '→', '↓', '←', '→', '→', '↑'],
        vec!['↓', '↑', '→', '↑', '→', '↓', '↓'],
        vec!['↓', '↓', '←', '←', '→', '←', '→'],
        vec!['↑', '↓', '↓', '→', '→', '→', '←'],
    ]
}

fn is_arrow(ch: char) -> bool {
    matches!(ch, '↑' | '↓' | '←' | '→')
}

fn grid_to_chars(grid: &[Vec<String>]) -> Option<Vec<Vec<char>>> {
    if grid.is_empty() || grid[0].is_empty() {
        return None;
    }
    let w = grid[0].len();
    if !grid.iter().all(|r| r.len() == w) {
        return None;
    }
    let out: Vec<Vec<char>> = grid
        .iter()
        .map(|row| {
            row.iter()
                .map(|s| s.chars().next().unwrap_or(' '))
                .collect::<Vec<char>>()
        })
        .collect();
    Some(out)
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

fn apply_xform(g: &[Vec<char>], t: GridXform) -> Vec<Vec<char>> {
    let h = g.len();
    let w = g[0].len();
    match t {
        GridXform::Identity => g.to_vec(),
        GridXform::Rot90 => {
            let mut out = vec![vec![' '; h]; w];
            for r in 0..h {
                for c in 0..w {
                    out[c][h - 1 - r] = g[r][c];
                }
            }
            out
        }
        GridXform::Rot180 => {
            let mut out = vec![vec![' '; w]; h];
            for r in 0..h {
                for c in 0..w {
                    out[h - 1 - r][w - 1 - c] = g[r][c];
                }
            }
            out
        }
        GridXform::Rot270 => {
            let mut out = vec![vec![' '; h]; w];
            for r in 0..h {
                for c in 0..w {
                    out[w - 1 - c][r] = g[r][c];
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
            let mut out = vec![vec![' '; h]; w];
            for r in 0..h {
                for c in 0..w {
                    out[c][r] = g[r][c];
                }
            }
            out
        }
        GridXform::AntiTranspose => {
            // transpose + rotate180
            let mut out = vec![vec![' '; h]; w];
            for r in 0..h {
                for c in 0..w {
                    out[w - 1 - c][h - 1 - r] = g[r][c];
                }
            }
            out
        }
    }
}

fn remap_arrow(ch: char, perm: &[char; 4]) -> char {
    // perm gives mapping for [↑, ↓, ←, →] in that order.
    match ch {
        '↑' => perm[0],
        '↓' => perm[1],
        '←' => perm[2],
        '→' => perm[3],
        other => other,
    }
}

fn apply_arrow_perm(g: &[Vec<char>], perm: &[char; 4]) -> Vec<Vec<char>> {
    g.iter()
        .map(|row| row.iter().copied().map(|ch| remap_arrow(ch, perm)).collect())
        .collect()
}

fn score_window(g: &[Vec<char>], top: usize, left: usize, expected: &[Vec<char>]) -> usize {
    let eh = expected.len();
    let ew = expected[0].len();
    let mut ok = 0usize;
    for r in 0..eh {
        for c in 0..ew {
            if g[top + r][left + c] == expected[r][c] {
                ok += 1;
            }
        }
    }
    ok
}

fn all_arrow_perms() -> Vec<[char; 4]> {
    let base = ['↑', '↓', '←', '→'];
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
                    perms.push([a, b, c, d]);
                }
            }
        }
    }
    perms
}

fn find_images_under(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(read) = std::fs::read_dir(dir) else {
        return out;
    };
    for entry in read.flatten() {
        let p = entry.path();
        if p.is_dir() {
            out.extend(find_images_under(&p));
            continue;
        }
        let Some(ext) = p.extension().and_then(|e| e.to_str()) else {
            continue;
        };
        if matches!(ext.to_ascii_lowercase().as_str(), "png" | "jpg" | "jpeg") {
            out.push(p);
        }
    }
    out
}

fn main() {
    let expected = expected();
    let expected_cells = expected.len() * expected[0].len();
    let eh = expected.len();
    let ew = expected[0].len();

    let root = PathBuf::from("assets");
    let mut images = find_images_under(&root);
    images.sort();

    let cfg = AnotoConfig::default();
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

    let mut best: Option<(
        usize,
        PathBuf,
        &'static str,
        GridXform,
        [char; 4],
        (usize, usize),
        Vec<Vec<String>>,
    )> = None;

    for img_path in images {
        let img = match image::open(&img_path) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let dots = match detect_components_from_image(&img, &cfg) {
            Ok(v) => v,
            Err(_) => continue,
        };

        // Try both lattice builders.
        for (label, full_grid) in [
            ("phase", build_intersection_grid(&dots, &cfg)),
            ("observed", build_intersection_grid_observed(&dots, &cfg)),
        ] {
            let minified = minify_from_full_grid(&full_grid);
            let Some(chars) = grid_to_chars(&minified) else {
                continue;
            };

            for &xf in &xforms {
                let g1 = apply_xform(&chars, xf);
                if g1.len() == 0 || g1[0].len() == 0 {
                    continue;
                }
                for perm in &perms {
                    let g2 = apply_arrow_perm(&g1, perm);

                    let gh = g2.len();
                    let gw = g2[0].len();
                    let eh = expected.len();
                    let ew = expected[0].len();
                    if gh < eh || gw < ew {
                        continue;
                    }

                    for top in 0..=(gh - eh) {
                        for left in 0..=(gw - ew) {
                            // Window must be arrow-only.
                            let mut ok_window = true;
                            'outer: for r in 0..eh {
                                for c in 0..ew {
                                    if !is_arrow(g2[top + r][left + c]) {
                                        ok_window = false;
                                        break 'outer;
                                    }
                                }
                            }
                            if !ok_window {
                                continue;
                            }

                            let score = score_window(&g2, top, left, &expected);
                            if best.as_ref().map_or(true, |(bs, ..)| score > *bs) {
                                best = Some((
                                    score,
                                    img_path.clone(),
                                    label,
                                    xf,
                                    *perm,
                                    (top, left),
                                    minified.clone(),
                                ));
                            }
                            if score == expected_cells {
                                break;
                            }
                        }
                        if best.as_ref().map_or(false, |(bs, ..)| *bs == expected_cells) {
                            break;
                        }
                    }
                    if best.as_ref().map_or(false, |(bs, ..)| *bs == expected_cells) {
                        break;
                    }
                }
                if best.as_ref().map_or(false, |(bs, ..)| *bs == expected_cells) {
                    break;
                }
            }
            if best.as_ref().map_or(false, |(bs, ..)| *bs == expected_cells) {
                break;
            }
        }
        if best.as_ref().map_or(false, |(bs, ..)| *bs == expected_cells) {
            break;
        }
    }

    match best {
        Some((score, path, label, xf, perm, (top, left), minified)) => {
            println!("Best match score: {score}/{expected_cells}");
            println!("Image: {}", path.display());
            println!("Grid: {label}");
            println!("Transform: {:?}", xf);
            println!("Arrow perm [↑,↓,←,→] -> {:?}", perm);
            println!("Window offset: (row={}, col={})", top, left);

            // Extract the matched window and print it as minified JSON.
            let window: Vec<Vec<String>> = (0..eh)
                .map(|r| {
                    (0..ew)
                        .map(|c| minified[top + r][left + c].clone())
                        .collect::<Vec<String>>()
                })
                .collect();
            println!("\nMatched window JSON:\n{}", write_grid_json_string(&window));

            // Try decoding from this window.
            let decoded = decode_all_windows_from_minified_arrows(&window);
            println!("Decoded windows from matched window: {}", decoded.len());
            if let Some(first) = decoded.first() {
                println!("First decode: window ({}, {}) -> (x={}, y={})", first.window_col, first.window_row, first.x, first.y);
            }

            println!("Minified JSON (current pipeline):\n{}", write_grid_json_string(&minified));
        }
        None => {
            println!("No candidate image produced a comparable minified grid under assets/.");
        }
    }
}
