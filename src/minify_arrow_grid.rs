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

    for by in 0..h3 {
        for bx in 0..w3 {
            let mut counts = [0usize; 4];
            for dy in 0..3 {
                for dx in 0..3 {
                    let cell = &cropped[by * 3 + dy][bx * 3 + dx];
                    let ch = cell.chars().next().unwrap_or(' ');
                    if let Some(idx) = arrow_index(ch) {
                        counts[idx] += 1;
                    }
                }
            }

            // A minified cell is present if the 3×3 block contains any detected arrows.
            // Missing blocks must NOT be filled with a fallback arrow.
            let total = counts.iter().sum::<usize>();
            if total == 0 {
                out[by][bx] = None;
            } else {
                let (best_idx, _) = counts
                    .iter()
                    .enumerate()
                    .max_by_key(|(_i, v)| *v)
                    .unwrap();
                out[by][bx] = Some(match best_idx {
                    0 => '↑',
                    1 => '↓',
                    2 => '←',
                    _ => '→',
                });
            }
        }
    }

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

    let cropped_blocks: Vec<Vec<Option<char>>> = out
        .iter()
        .take(max_r + 1)
        .skip(min_r)
        .map(|row| row[min_c..=max_c].to_vec())
        .collect();

    let minified: Vec<Vec<String>> = cropped_blocks
        .into_iter()
        .map(|r| {
            r.into_iter()
                .map(|v| v.map(|ch| ch.to_string()).unwrap_or_else(|| " ".to_string()))
                .collect()
        })
        .collect();

    crop_to_largest_arrow_rectangle(&minified)
}

pub fn extract_fully_filled_arrow_window(
    grid: &[Vec<String>],
    target_rows: usize,
    target_cols: usize,
) -> Option<Vec<Vec<String>>> {
    if target_rows == 0 || target_cols == 0 {
        return None;
    }
    if grid.is_empty() || grid[0].is_empty() {
        return None;
    }

    let h = grid.len();
    let w = grid[0].len();
    if !grid.iter().all(|r| r.len() == w) {
        return None;
    }
    if h < target_rows || w < target_cols {
        return None;
    }

    let is_arrow = |s: &str| matches!(s, "↑" | "↓" | "←" | "→");

    for top in 0..=(h - target_rows) {
        for left in 0..=(w - target_cols) {
            let mut all_arrows = true;
            'rows: for r in top..(top + target_rows) {
                for c in left..(left + target_cols) {
                    if !is_arrow(grid[r][c].as_str()) {
                        all_arrows = false;
                        break 'rows;
                    }
                }
            }

            if all_arrows {
                let window: Vec<Vec<String>> = grid
                    .iter()
                    .take(top + target_rows)
                    .skip(top)
                    .map(|row| row[left..(left + target_cols)].to_vec())
                    .collect();
                return Some(window);
            }
        }
    }

    None
}

fn crop_to_largest_arrow_rectangle(grid: &[Vec<String>]) -> Vec<Vec<String>> {
    if grid.is_empty() || grid[0].is_empty() {
        return Vec::new();
    }
    let h = grid.len();
    let w = grid[0].len();
    if !grid.iter().all(|r| r.len() == w) {
        return grid.to_vec();
    }

    let is_arrow = |s: &str| matches!(s, "↑" | "↓" | "←" | "→");

    // Maximal rectangle in a binary matrix (arrow-only cells).
    let mut heights = vec![0usize; w];
    let mut best = (0usize, 0usize, 0usize, 0usize); // top, left, bottom, right inclusive
    let mut best_area = 0usize;

    for r in 0..h {
        for c in 0..w {
            if is_arrow(grid[r][c].as_str()) {
                heights[c] += 1;
            } else {
                heights[c] = 0;
            }
        }

        // Largest rectangle in histogram for this row.
        let mut stack: Vec<usize> = Vec::new();
        let mut i = 0usize;
        while i <= w {
            let cur_h = if i == w { 0 } else { heights[i] };
            if stack.is_empty() || cur_h >= heights[*stack.last().unwrap()] {
                stack.push(i);
                i += 1;
            } else {
                let top = stack.pop().unwrap();
                let height = heights[top];
                if height == 0 {
                    continue;
                }
                let right = i.saturating_sub(1);
                let left = if stack.is_empty() { 0 } else { stack.last().unwrap() + 1 };
                let width = right + 1 - left;
                let area = height * width;
                if area > best_area {
                    best_area = area;
                    let bottom = r;
                    let top_row = r + 1 - height;
                    best = (top_row, left, bottom, right);
                }
            }
        }
    }

    if best_area == 0 {
        return grid.to_vec();
    }

    let (top, left, bottom, right) = best;
    let cropped: Vec<Vec<String>> = grid
        .iter()
        .take(bottom + 1)
        .skip(top)
        .map(|row| row[left..=right].to_vec())
        .collect();

    // Prefer returning something decoding-friendly: if we can't get at least 6×6,
    // keep the original grid so the user can see what was detected.
    if cropped.len() >= 6 && cropped[0].len() >= 6 {
        cropped
    } else {
        grid.to_vec()
    }
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

    let is_arrow = |v: &str| matches!(v, "↑" | "↓" | "←" | "→");

    // Find bounds based on ANY arrow (robust when a whole phase row/col is missing).
    let top_any = (0..h).find(|&r| grid[r].iter().any(|v| is_arrow(v.as_str())));
    let bottom_any = (0..h)
        .rev()
        .find(|&r| grid[r].iter().any(|v| is_arrow(v.as_str())));
    let left_any = (0..w).find(|&c| (0..h).any(|r| is_arrow(grid[r][c].as_str())));
    let right_any = (0..w)
        .rev()
        .find(|&c| (0..h).any(|r| is_arrow(grid[r][c].as_str())));

    let (top_any, bottom_any, left_any, right_any) = match (top_any, bottom_any, left_any, right_any) {
        (Some(t), Some(b), Some(l), Some(r)) if t <= b && l <= r => (t, b, l, r),
        _ => return None,
    };

    // Classify a row by arrow counts: Green=↑, Mid=←/→, Orange=↓.
    let row_kind = |r: usize| {
        let mut up = 0usize;
        let mut down = 0usize;
        let mut mid = 0usize;
        for v in &grid[r] {
            match v.as_str() {
                "↑" => up += 1,
                "↓" => down += 1,
                "←" | "→" => mid += 1,
                _ => {}
            }
        }
        let max = up.max(mid).max(down);
        if max == 0 {
            0usize
        } else if up == max {
            0usize
        } else if mid == max {
            1usize
        } else {
            2usize
        }
    };

    // Classify a col by arrow counts: Blue=←, Vertical=↑/↓, Magenta=→.
    let col_kind = |c: usize| {
        let mut left = 0usize;
        let mut right = 0usize;
        let mut vertical = 0usize;
        for r in 0..h {
            match grid[r][c].as_str() {
                "←" => left += 1,
                "→" => right += 1,
                "↑" | "↓" => vertical += 1,
                _ => {}
            }
        }
        let max = left.max(vertical).max(right);
        if max == 0 {
            0usize
        } else if left == max {
            0usize
        } else if vertical == max {
            1usize
        } else {
            2usize
        }
    };

    // Backtrack 1–2 rows/cols so the crop origin stays phase-aligned even when
    // the top green row or left blue column is entirely missing.
    let back_rows = row_kind(top_any); // 0=green,1=mid,2=orange
    let back_cols = col_kind(left_any); // 0=blue,1=vertical,2=magenta

    let top = top_any.saturating_sub(back_rows);
    let left = left_any.saturating_sub(back_cols);

    Some((top, bottom_any, left, right_any))
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


