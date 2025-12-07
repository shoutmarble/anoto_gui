use std::collections::HashSet;
use std::collections::VecDeque;

#[test]
fn stats_on_assets() {
    let paths = vec!["src/kornia/assets/anoto_dots2.png"];    
    for p in paths {
        let img = image::open(p).expect("failed to open");
        let rgb = img.to_rgb8();
        let (w,h) = (rgb.width(), rgb.height());
        eprintln!("Image {} size {}x{}", p, w, h);
        let mut unique = HashSet::new();
        let mut sum = 0u64;
        for px in rgb.pixels() {
            unique.insert((px[0], px[1], px[2]));
            sum += (0.2126*px[0] as f32 + 0.7152*px[1] as f32 + 0.0722*px[2] as f32) as u64;
        }
        eprintln!("unique colors: {} avg brightness: {}", unique.len(), sum as f64 / (w as f64 * h as f64));

        // Count darker-than-mid pixels
        let mut dark_count = 0usize;
        for px in rgb.pixels() {
            let l = (0.2126*px[0] as f32 + 0.7152*px[1] as f32 + 0.0722*px[2] as f32) as u8;
            if l < 128 { dark_count += 1; }
        }
        eprintln!("dark pixels: {}", dark_count);

        // run detection and print components count
        let dets = anoto_dot_reader::kornia::anoto::detect_components_from_image(&img, &anoto_dot_reader::kornia::anoto::AnotoConfig::default()).expect("detect failed");
        eprintln!("components (filtered): {}", dets.len());

        // Now compute raw connected components sizes (before config filtering)
        // We'll replicate Otsu threshold and flood-fill to gather component sizes
        let gray_img = img.to_luma8();
        // histogram
        let mut histogram = [0u32; 256];
        for v in gray_img.pixels() { histogram[v[0] as usize] += 1; }
        let total_pixels = gray_img.pixels().len() as f64;
        let sum_total = histogram.iter().enumerate().map(|(v, &c)| v as f64 * c as f64).sum::<f64>();
        let mut sum_b = 0f64;
        let mut w_b = 0f64;
        let mut max_var = f64::MIN;
        let mut thresh = 128u8;
        for (v, &count) in histogram.iter().enumerate() {
            w_b += count as f64;
            if w_b == 0.0 { continue; }
            let w_f = total_pixels - w_b;
            if w_f == 0.0 { break; }
            sum_b += v as f64 * count as f64;
            let m_b = sum_b / w_b;
            let m_f = (sum_total - sum_b) / w_f;
            let var = w_b * w_f * (m_b - m_f).powi(2);
            if var > max_var { max_var = var; thresh = v as u8; }
        }
        eprintln!("otsu threshold {}", thresh);
        // binary mask
        let mut mask = vec![0u8; (w*h) as usize];
        for (i, p) in gray_img.pixels().enumerate() { mask[i] = if p[0] <= thresh { 255u8 } else { 0u8 }; }
        let fg_count = mask.iter().filter(|&&v| v != 0).count();
        eprintln!("mask fg pixels: {}", fg_count);
        if fg_count * 2 > mask.len() {
            eprintln!("flipping mask due to majority foreground");
            for b in mask.iter_mut() { *b = if *b == 0 { 255u8 } else { 0u8 } }
        }
        // collect components
        let mut labels = vec![0usize; mask.len()];
        let mut comps = Vec::new();
        let mut current_label = 1usize;
        for y in 0..h as usize {
            for x in 0..w as usize {
                let idx = y*w as usize + x;
                if mask[idx] == 0 || labels[idx] != 0 { continue; }
                let mut stack = VecDeque::new();
                stack.push_back(idx);
                labels[idx] = current_label;
                let mut sum_x: usize = 0;
                let mut sum_y: usize = 0;
                let mut count: usize = 0;
                while let Some(p) = stack.pop_front() {
                    let py = p / w as usize;
                    let px = p % w as usize;
                    sum_x += px; sum_y += py; count += 1;
                    // neighbors
                    if px > 0 { let n = p - 1; if mask[n] != 0 && labels[n]==0 { labels[n] = current_label; stack.push_back(n); } }
                    if px + 1 < w as usize { let n = p + 1; if mask[n] != 0 && labels[n]==0 { labels[n] = current_label; stack.push_back(n); } }
                    if py > 0 { let n = p - w as usize; if mask[n] != 0 && labels[n]==0 { labels[n] = current_label; stack.push_back(n); } }
                    if py + 1 < h as usize { let n = p + w as usize; if mask[n] != 0 && labels[n]==0 { labels[n] = current_label; stack.push_back(n); } }
                }
                comps.push((sum_x, sum_y, count));
                current_label += 1;
            }
        }
        eprintln!("raw components count {}", comps.len());
        let mut small=0; let mut mid=0; let mut big=0; for (_,_,c) in comps.iter() {
            if *c <= 10 { small+=1; } else if *c <= 250 { mid+=1; } else { big+=1; }
        }
        eprintln!("component size counts small <=10: {} mid <=250: {} big >250: {}", small, mid, big);

        // Try morphological erosion (1 iteration) to see if we break large components
        let mut mask_eroded = mask.clone();
        let width_usize = w as usize; let height_usize = h as usize;
        for y in 1..height_usize-1 {
            for x in 1..width_usize-1 {
                let idx = y*width_usize + x;
                if mask[idx] == 0 { continue; }
                // check 8 neighbors for background; if any neighbor background -> remove
                let mut remove = false;
                for oy in -1..=1 {
                    for ox in -1..=1 {
                        if ox==0 && oy==0 { continue; }
                        let nx = (x as isize + ox) as usize; let ny = (y as isize + oy) as usize;
                        let nidx = ny*width_usize + nx;
                        if mask[nidx] == 0 { remove = true; break; }
                    }
                    if remove { break; }
                }
                if remove { mask_eroded[idx] = 0u8; }
            }
        }
        // compute components on eroded mask
        let mut labels2 = vec![0usize; mask_eroded.len()];
        let mut comps2 = Vec::new();
        let mut current_label2 = 1usize;
        for y in 0..height_usize {
            for x in 0..width_usize {
                let idx = y*width_usize + x;
                if mask_eroded[idx] == 0 || labels2[idx] != 0 { continue; }
                let mut stack = VecDeque::new();
                stack.push_back(idx);
                labels2[idx] = current_label2;
                let mut sum_x: usize = 0; let mut sum_y: usize = 0; let mut count: usize = 0;
                while let Some(p) = stack.pop_front() {
                    let py = p / width_usize; let px = p % width_usize;
                    sum_x += px; sum_y += py; count += 1;
                    if px > 0 { let n = p - 1; if mask_eroded[n] != 0 && labels2[n] == 0 { labels2[n] = current_label2; stack.push_back(n);} }
                    if px + 1 < width_usize { let n = p + 1; if mask_eroded[n] != 0 && labels2[n] == 0 { labels2[n] = current_label2; stack.push_back(n);} }
                    if py > 0 { let n = p - width_usize; if mask_eroded[n] != 0 && labels2[n] == 0 { labels2[n] = current_label2; stack.push_back(n);} }
                    if py + 1 < height_usize { let n = p + width_usize; if mask_eroded[n] != 0 && labels2[n] == 0 { labels2[n] = current_label2; stack.push_back(n);} }
                }
                comps2.push((sum_x, sum_y, count)); current_label2 += 1;
            }
        }
        let mut small2=0; let mut mid2=0; let mut big2=0; for (_,_,c) in comps2.iter() {
            if *c <= 10 { small2+=1; } else if *c <= 250 { mid2+=1; } else { big2+=1; }
        }
        eprintln!("after 1 erosion: raw comps {} small {} mid {} big {}", comps2.len(), small2, mid2, big2);
    }
}
