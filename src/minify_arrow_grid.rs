pub fn grid_from_arrow_text(grid: &str) -> Vec<Vec<String>> {
    let mut rows: Vec<Vec<String>> = grid
        .lines()
        .map(|line| line.chars().map(|ch| ch.to_string()).collect::<Vec<String>>())
        .collect();

    let max_w = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    if max_w == 0 {
        return Vec::new();
    }

    for row in &mut rows {
        if row.len() < max_w {
            row.extend(std::iter::repeat_n(" ".to_string(), max_w - row.len()));
        }
    }

    rows
}

pub fn write_grid_json_string(grid: &[Vec<String>]) -> String {
    let mut out = String::new();
    out.push_str("[\n");
    for (ri, row) in grid.iter().enumerate() {
        out.push_str("  [");
        for (ci, cell) in row.iter().enumerate() {
            if ci > 0 {
                out.push_str(", ");
            }
            out.push('"');
            out.push_str(cell);
            out.push('"');
        }
        out.push(']');
        if ri + 1 < grid.len() {
            out.push_str(",\n");
        } else {
            out.push('\n');
        }
    }
    out.push(']');
    out
}

pub fn minify_from_full_grid(grid: &[Vec<String>]) -> Vec<Vec<String>> {
    if grid.is_empty() || grid[0].is_empty() {
        return Vec::new();
    }

    let height = grid.len();
    let width = grid[0].len();
    if !grid.iter().all(|r| r.len() == width) {
        return Vec::new();
    }

    let (top, bottom, left, right) = match crop_bounds_any_anchor(grid) {
        Some(v) => v,
        None => return Vec::new(),
    };

    // Preserve the crop origin (top/left) to keep the 3x3 phase consistent.
    let row_start = top;
    let col_start = left;
    let mut row_end = boundary_high(bottom).min(height.saturating_sub(1));
    let mut col_end = boundary_high(right).min(width.saturating_sub(1));

    // If boundary expansion pulls in extremely sparse border rows/cols, shave them.
    let row_non_space =
        |r: usize, c0: usize, c1: usize| grid[r][c0..=c1].iter().filter(|v| v.as_str() != " ").count();
    let col_non_space = |c: usize, r0: usize, r1: usize| {
        grid.iter()
            .take(r1 + 1)
            .skip(r0)
            .filter(|row| row[c].as_str() != " ")
            .count()
    };

    while row_end > bottom && row_non_space(row_end, col_start, col_end) <= 1 {
        row_end -= 1;
    }
    while col_end > right && col_non_space(col_end, row_start, row_end) <= 1 {
        col_end -= 1;
    }

    if row_start > row_end || col_start > col_end {
        return Vec::new();
    }

    let cropped: Vec<Vec<String>> = grid
        .iter()
        .take(row_end + 1)
        .skip(row_start)
        .map(|row| row[col_start..=col_end].to_vec())
        .collect();

    let cropped = pad_to_multiple_of_3(cropped);
    if cropped.is_empty() {
        return Vec::new();
    }
    let h = cropped.len();
    let w = cropped[0].len();
    if h % 3 != 0 || w % 3 != 0 {
        return Vec::new();
    }

    let h3 = h / 3;
    let w3 = w / 3;

    let mut out: Vec<Vec<Option<char>>> = vec![vec![None; w3]; h3];
    let mut total_counts = [0usize; 4];

    for by in 0..h3 {
        for bx in 0..w3 {
            let mut picked: Option<char> = None;
            for dy in 0..3 {
                for dx in 0..3 {
                    let cell = &cropped[by * 3 + dy][bx * 3 + dx];
                    let ch = cell.chars().next().unwrap_or(' ');
                    if arrow_index(ch).is_some() && picked.is_none() {
                        picked = Some(ch);
                    }
                }
            }

            if let Some(ch) = picked && let Some(i) = arrow_index(ch) {
                total_counts[i] += 1;
            }
            out[by][bx] = picked;
        }
    }

    let fallback = pick_majority_from_counts(&total_counts);

    // Trim away any fully-empty block rows/cols.
    let mut min_r: Option<usize> = None;
    let mut max_r: Option<usize> = None;
    let mut min_c: Option<usize> = None;
    let mut max_c: Option<usize> = None;
    for (r, row) in out.iter().enumerate().take(h3) {
        for (c, cell) in row.iter().enumerate().take(w3) {
            if cell.is_some() {
                min_r = Some(min_r.map_or(r, |v| v.min(r)));
                max_r = Some(max_r.map_or(r, |v| v.max(r)));
                min_c = Some(min_c.map_or(c, |v| v.min(c)));
                max_c = Some(max_c.map_or(c, |v| v.max(c)));
            }
        }
    }

    let (min_r, max_r, min_c, max_c) = match (min_r, max_r, min_c, max_c) {
        (Some(a), Some(b), Some(c), Some(d)) => (a, b, c, d),
        _ => return Vec::new(),
    };

    let mut cropped_blocks: Vec<Vec<Option<char>>> = out
        .iter()
        .take(max_r + 1)
        .skip(min_r)
        .map(|row| row[min_c..=max_c].to_vec())
        .collect();

    trim_incomplete_edges(&mut cropped_blocks);
    fill_unknown_blocks(&mut cropped_blocks, fallback);

    cropped_blocks
        .into_iter()
        .map(|r| r.into_iter().map(|v| v.unwrap_or(fallback).to_string()).collect())
        .collect()
}

fn pick_majority(counts: &[usize; 4]) -> Option<char> {
    // Order: ↑ ↓ ← →
    let max = *counts.iter().max().unwrap_or(&0);
    if max == 0 {
        return None;
    }
    let chars = ['↑', '↓', '←', '→'];
    for (i, ch) in chars.iter().enumerate() {
        if counts[i] == max {
            return Some(*ch);
        }
    }
    None
}

fn arrow_index(ch: char) -> Option<usize> {
    match ch {
        '↑' => Some(0),
        '↓' => Some(1),
        '←' => Some(2),
        '→' => Some(3),
        _ => None,
    }
}

fn boundary_high(idx: usize) -> usize {
    idx + (2 - (idx % 3))
}

fn crop_bounds_any_anchor(grid: &[Vec<String>]) -> Option<(usize, usize, usize, usize)> {
    if grid.is_empty() || grid[0].is_empty() {
        return None;
    }
    let h = grid.len();
    let w = grid[0].len();
    if !grid.iter().all(|r| r.len() == w) {
        return None;
    }

    let top = (0..h).find(|&r| grid[r].iter().any(|v| v == "↑"));
    let bottom = (0..h).rev().find(|&r| grid[r].iter().any(|v| v == "↓"));
    let left = (0..w).find(|&c| (0..h).any(|r| grid[r][c] == "←"));
    let right = (0..w).rev().find(|&c| (0..h).any(|r| grid[r][c] == "→"));

    match (top, bottom, left, right) {
        (Some(t), Some(b), Some(l), Some(r)) if t <= b && l <= r => Some((t, b, l, r)),
        _ => None,
    }
}

fn pad_to_multiple_of_3(mut grid: Vec<Vec<String>>) -> Vec<Vec<String>> {
    if grid.is_empty() || grid[0].is_empty() {
        return grid;
    }
    let w = grid[0].len();
    if !grid.iter().all(|r| r.len() == w) {
        return Vec::new();
    }

    let pad_rows = (3 - (grid.len() % 3)) % 3;
    let pad_cols = (3 - (w % 3)) % 3;

    if pad_cols > 0 {
        for r in &mut grid {
            r.extend(std::iter::repeat_n(" ".to_string(), pad_cols));
        }
    }
    if pad_rows > 0 {
        let new_w = grid[0].len();
        for _ in 0..pad_rows {
            grid.push(vec![" ".to_string(); new_w]);
        }
    }
    grid
}

fn pick_majority_from_counts(counts: &[usize; 4]) -> char {
    pick_majority(counts).unwrap_or('↑')
}

fn fill_unknown_blocks(grid: &mut [Vec<Option<char>>], fallback: char) {
    if grid.is_empty() || grid[0].is_empty() {
        return;
    }

    let h = grid.len();
    let w = grid[0].len();

    for _ in 0..(h + w) {
        let mut changed = false;

        for r in 0..h {
            for c in 0..w {
                if grid[r][c].is_some() {
                    continue;
                }

                let mut neighbor_counts = [0usize; 4];
                let mut any = false;

                let neighbors = [
                    (r.wrapping_sub(1), c, r > 0),
                    (r + 1, c, r + 1 < h),
                    (r, c.wrapping_sub(1), c > 0),
                    (r, c + 1, c + 1 < w),
                ];

                for (nr, nc, ok) in neighbors {
                    if !ok {
                        continue;
                    }
                    if let Some(ch) = grid[nr][nc] && let Some(i) = arrow_index(ch) {
                        neighbor_counts[i] += 1;
                        any = true;
                    }
                }

                if any {
                    grid[r][c] = Some(pick_majority_from_counts(&neighbor_counts));
                    changed = true;
                }
            }
        }

        if !changed {
            break;
        }
    }

    for row in grid.iter_mut() {
        for cell in row.iter_mut() {
            if cell.is_none() {
                *cell = Some(fallback);
            }
        }
    }
}

fn trim_incomplete_edges(grid: &mut Vec<Vec<Option<char>>>) {
    if grid.is_empty() || grid[0].is_empty() {
        return;
    }
    let h = grid.len();
    let w = grid[0].len();
    if !grid.iter().all(|r| r.len() == w) {
        return;
    }

    let bbox_edge_shave = || {
        let is_row_too_sparse = |row: &[Option<char>]| {
            let total = row.len();
            if total == 0 {
                return true;
            }
            let some = row.iter().filter(|v| v.is_some()).count();
            some * 3 < total
        };

        let is_col_too_sparse = |rows: &[Vec<Option<char>>], col: usize, top: usize, bottom: usize| {
            let total = bottom.saturating_sub(top);
            if total == 0 {
                return true;
            }
            let some = rows
                .iter()
                .take(bottom)
                .skip(top)
                .filter(|row| row[col].is_some())
                .count();
            some * 3 < total
        };

        let mut top = 0usize;
        let mut bottom = h;
        let mut left = 0usize;
        let mut right = w;

        let mut changed = true;
        while changed {
            changed = false;

            if bottom - top > 1 {
                let edge_sparse = is_row_too_sparse(&grid[top][left..right]);
                if edge_sparse {
                    top += 1;
                    changed = true;
                }
            }
            if bottom - top > 1 {
                let edge_sparse = is_row_too_sparse(&grid[bottom - 1][left..right]);
                if edge_sparse {
                    bottom -= 1;
                    changed = true;
                }
            }

            if right - left > 1 {
                let edge_sparse = is_col_too_sparse(grid, left, top, bottom);
                if edge_sparse {
                    left += 1;
                    changed = true;
                }
            }
            if right - left > 1 {
                let edge_sparse = is_col_too_sparse(grid, right - 1, top, bottom);
                if edge_sparse {
                    right -= 1;
                    changed = true;
                }
            }
        }

        (top, bottom, left, right)
    };

    let bbox_complete = || -> Option<(usize, usize, usize, usize)> {
        let row_complete = |r: usize| grid[r].iter().all(|v| v.is_some());
        let col_complete = |c: usize| (0..h).all(|r| grid[r][c].is_some());

        let top = (0..h).find(|&r| row_complete(r))?;
        let bottom_inclusive = (0..h).rev().find(|&r| row_complete(r))?;
        let left = (0..w).find(|&c| col_complete(c))?;
        let right_inclusive = (0..w).rev().find(|&c| col_complete(c))?;

        if top > bottom_inclusive || left > right_inclusive {
            return None;
        }

        Some((top, bottom_inclusive + 1, left, right_inclusive + 1))
    };

    let (t1, b1, l1, r1) = bbox_edge_shave();
    let area1 = (b1 - t1) * (r1 - l1);

    let chosen = match bbox_complete() {
        Some((t2, b2, l2, r2)) => {
            let area2 = (b2 - t2) * (r2 - l2);
            if area1 >= area2 {
                (t1, b1, l1, r1)
            } else {
                (t2, b2, l2, r2)
            }
        }
        None => (t1, b1, l1, r1),
    };

    let (top, bottom, left, right) = chosen;
    if top >= bottom || left >= right {
        return;
    }

    let out: Vec<Vec<Option<char>>> = grid
        .iter()
        .take(bottom)
        .skip(top)
        .map(|row| row[left..right].to_vec())
        .collect();
    *grid = out;
}

