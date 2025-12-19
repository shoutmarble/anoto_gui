use std::collections::HashSet;

#[derive(Debug, Clone, thiserror::Error)]
pub enum DecodeError {
    #[error("patch must be 6x6")]
    BadPatchSize,

    #[error("invalid arrow character: {0}")]
    InvalidArrow(char),

    #[error("pattern not found in main number sequence")]
    MainSequenceNotFound,

    #[error("delta value out of expected range")]
    DeltaOutOfRange,

    #[error("coefficients not found in secondary sequences")]
    SecondarySequenceNotFound,

    #[error("crt solve failed: {0}")]
    CrtSolveFailed(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DecodedWindow {
    pub window_row: usize,
    pub window_col: usize,
    pub x: i32,
    pub y: i32,
}

/// Decode every valid 6x6 window in the minified arrow grid.
///
/// Cells must be one of: `↑`, `↓`, `←`, `→`.
pub fn decode_all_windows_from_minified_arrows(minified: &[Vec<String>]) -> Vec<DecodedWindow> {
    let h = minified.len();
    let Some(w) = minified.first().map(|r| r.len()) else {
        return Vec::new();
    };
    if w == 0 || !minified.iter().all(|r| r.len() == w) {
        return Vec::new();
    }
    if h < 6 || w < 6 {
        return Vec::new();
    }

    let codec = AnotoCodec::anoto_6x6_a4_fixed();
    let mut out = Vec::new();
    let mut seen = HashSet::<(usize, usize, i32, i32)>::new();

    for r0 in 0..=(h - 6) {
        for c0 in 0..=(w - 6) {
            let Some(patch) = patch_from_arrows(minified, r0, c0) else {
                continue;
            };
            if let Ok((x, y)) = codec.decode_position(&patch) {
                if seen.insert((r0, c0, x, y)) {
                    out.push(DecodedWindow {
                        window_row: r0,
                        window_col: c0,
                        x,
                        y,
                    });
                }
            }
        }
    }

    out.sort_unstable_by(|a, b| (a.window_row, a.window_col, a.y, a.x).cmp(&(b.window_row, b.window_col, b.y, b.x)));
    out
}

fn patch_from_arrows(
    grid: &[Vec<String>],
    row0: usize,
    col0: usize,
) -> Option<[[[i8; 2]; 6]; 6]> {
    let mut out = [[[0i8; 2]; 6]; 6];

    for r in 0..6 {
        for c in 0..6 {
            let s = grid.get(row0 + r)?.get(col0 + c)?;
            let ch = s.chars().next()?;
            let (b0, b1) = arrow_to_bits(ch)?;
            out[r][c] = [b0, b1];
        }
    }

    Some(out)
}

/// Mapping used by AnotoPdfGenerator:
/// - `↑` => (0,0)
/// - `←` => (1,0)
/// - `→` => (0,1)
/// - `↓` => (1,1)
fn arrow_to_bits(ch: char) -> Option<(i8, i8)> {
    match ch {
        '↑' => Some((0, 0)),
        '←' => Some((1, 0)),
        '→' => Some((0, 1)),
        '↓' => Some((1, 1)),
        _ => None,
    }
}

#[derive(Debug, Clone)]
struct NumberBasis {
    factors: [i64; 4],
}

impl NumberBasis {
    fn new(factors: [i64; 4]) -> Self {
        Self { factors }
    }

    fn reconstruct(&self, coeffs: &[i8; 4]) -> i64 {
        let mut result = 0i64;
        let mut base = 1i64;
        for (i, &coeff) in coeffs.iter().enumerate() {
            result += coeff as i64 * base;
            base *= self.factors[i];
        }
        result
    }

    fn project(&self, values: &[i64; 5]) -> [[i8; 4]; 5] {
        let mut out = [[0i8; 4]; 5];

        for (i, &val) in values.iter().enumerate() {
            let mut remaining = val;
            for (j, &factor) in self.factors.iter().enumerate() {
                out[i][j] = (remaining % factor) as i8;
                remaining /= factor;
            }
        }

        out
    }
}

#[derive(Debug, Clone)]
struct Crt {
    moduli: [i64; 4],
}

impl Crt {
    fn new(moduli: [i64; 4]) -> Self {
        Self { moduli }
    }

    fn solve(&self, remainders: [i64; 4]) -> Result<i64, DecodeError> {
        let product: i64 = self.moduli.iter().product();
        let mut result = 0i64;

        for i in 0..4 {
            let modulus = self.moduli[i];
            let remainder = remainders[i];
            let partial_product = product / modulus;
            let inverse = mod_inverse(partial_product, modulus)
                .map_err(|e| DecodeError::CrtSolveFailed(e))?;
            result = (result + remainder * partial_product * inverse) % product;
        }

        Ok(result)
    }
}

fn mod_inverse(a: i64, m: i64) -> Result<i64, String> {
    let (gcd, x, _) = extended_gcd(a, m);
    if gcd != 1 {
        return Err("modular inverse does not exist".to_string());
    }
    Ok((x % m + m) % m)
}

fn extended_gcd(a: i64, b: i64) -> (i64, i64, i64) {
    if a == 0 {
        return (b, 0, 1);
    }
    let (gcd, x1, y1) = extended_gcd(b % a, a);
    let x = y1 - (b / a) * x1;
    let y = x1;
    (gcd, x, y)
}

#[derive(Debug, Clone)]
struct AnotoCodec {
    mns: [i8; 63],
    mns_cyclic: Vec<i8>,
    sns_lengths: [usize; 4],
    sns_cyclic: [Vec<i8>; 4],
    num_basis: NumberBasis,
    crt: Crt,
    delta_range: (i32, i32),
}

impl AnotoCodec {
    fn anoto_6x6_a4_fixed() -> Self {
        // These sequences are taken from public Anoto specifications/patents
        // and match the widely-used 6x6 codec parameters.
        let mns: [i8; 63] = [
            0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 1, 1, 1, 1, 0, 1, 0, 0, 1, 0, 0,
            0, 0, 1, 1, 1, 0, 1, 1, 1, 0, 0, 1, 0, 1, 0, 1, 0, 0, 0, 1, 0,
            1, 1, 0, 1, 1, 0, 0, 1, 1, 0, 1, 0, 1, 1, 1, 1, 0, 0, 0, 1, 1,
        ];

        let sns_order = 5;
        let mns_order = 6;

        let a1: Vec<i8> = include!("anoto_sequences/a1.incl");
        let a2: Vec<i8> = include!("anoto_sequences/a2.incl");
        let a3: Vec<i8> = include!("anoto_sequences/a3.incl");
        let a4: Vec<i8> = include!("anoto_sequences/a4_alt.incl");

        let sns_lengths: [usize; 4] = [a1.len(), a2.len(), a3.len(), a4.len()];
        let sns_cyclic: [Vec<i8>; 4] = [
            make_cyclic(&a1, sns_order),
            make_cyclic(&a2, sns_order),
            make_cyclic(&a3, sns_order),
            make_cyclic(&a4, sns_order),
        ];

        let mns_cyclic = make_cyclic(&mns, mns_order);

        let factors = [3i64, 3i64, 2i64, 3i64];
        let num_basis = NumberBasis::new(factors);
        let crt = Crt::new([
            sns_lengths[0] as i64,
            sns_lengths[1] as i64,
            sns_lengths[2] as i64,
            sns_lengths[3] as i64,
        ]);

        Self {
            mns,
            mns_cyclic,
            sns_lengths,
            sns_cyclic,
            num_basis,
            crt,
            delta_range: (5, 58),
        }
    }

    fn mns_length(&self) -> i32 {
        self.mns.len() as i32
    }

    fn delta(&self, pos: i32) -> i32 {
        let pos_usize = pos as usize;
        let mut coeffs = [0i8; 4];
        for i in 0..4 {
            let r = pos_usize % self.sns_lengths[i];
            coeffs[i] = self.sns_cyclic[i][r];
        }
        (self.num_basis.reconstruct(&coeffs) + self.delta_range.0 as i64) as i32
    }

    fn integrate_roll(&self, pos: i32, first_roll: i32) -> i32 {
        let mut r = 0i64;
        for i in 0..pos {
            r += self.delta(i) as i64;
        }
        ((first_roll as i64 + r) % self.mns_length() as i64) as i32
    }

    fn rotate_mns_for_roll(&self, roll: i32) -> [i8; 63] {
        // Match the reference implementation's `rotate_vec(&mns, -(roll))` behavior.
        // With its normalization, this is equivalent to rotating the MNS left by `roll`.
        let len = self.mns.len();
        let r = ((roll % len as i32) + len as i32) % len as i32;
        let mut out = [0i8; 63];
        for i in 0..len {
            out[i] = self.mns[(i + r as usize) % len];
        }
        out
    }

    #[allow(dead_code)]
    fn encode_patch(&self, pos: (i32, i32), section_start_rolls: (i32, i32)) -> [[[i8; 2]; 6]; 6] {
        let (x_start, y_start) = pos;
        let mut out = [[[0i8; 2]; 6]; 6];

        // channel 0 (x-direction)
        for c in 0..6 {
            let abs_x = x_start + c as i32;
            let roll = self.integrate_roll(abs_x, section_start_rolls.0);
            let rolled = self.rotate_mns_for_roll(roll);

            for r in 0..6 {
                let abs_y = y_start + r as i32;
                out[r][c][0] = rolled[(abs_y as usize) % self.mns.len()];
            }
        }

        // channel 1 (y-direction)
        for r in 0..6 {
            let abs_y = y_start + r as i32;
            let roll = self.integrate_roll(abs_y, section_start_rolls.1);
            let rolled = self.rotate_mns_for_roll(roll);

            for c in 0..6 {
                let abs_x = x_start + c as i32;
                out[r][c][1] = rolled[(abs_x as usize) % self.mns.len()];
            }
        }

        out
    }

    fn decode_position(&self, bits: &[[[i8; 2]; 6]; 6]) -> Result<(i32, i32), DecodeError> {
        // x-direction uses transpose of channel 0
        let mut x_bits = [[0i8; 6]; 6];
        let mut y_bits = [[0i8; 6]; 6];

        for r in 0..6 {
            for c in 0..6 {
                x_bits[r][c] = bits[c][r][0];
                y_bits[r][c] = bits[r][c][1];
            }
        }

        let x = self.decode_along_direction(&x_bits)?;
        let y = self.decode_along_direction(&y_bits)?;
        Ok((x, y))
    }

    fn decode_along_direction(&self, bits: &[[i8; 6]; 6]) -> Result<i32, DecodeError> {
        let mut locs = [0i32; 6];

        for r in 0..6 {
            let row = &bits[r];
            let pos = find_subsequence(&self.mns_cyclic, row).ok_or(DecodeError::MainSequenceNotFound)?;
            locs[r] = pos as i32;
        }

        // differences between consecutive MNS locations
        let mut deltae = [0i64; 5];
        let mns_len = self.mns_length();
        for i in 1..6 {
            let diff = (locs[i] - locs[i - 1] + mns_len) % mns_len;
            if diff < self.delta_range.0 || diff > self.delta_range.1 {
                return Err(DecodeError::DeltaOutOfRange);
            }
            deltae[i - 1] = (diff - self.delta_range.0) as i64;
        }

        let coeffs = self.num_basis.project(&deltae);

        let mut remainders = [0i64; 4];
        for i in 0..4 {
            let mut coeff_seq = [0i8; 5];
            for j in 0..5 {
                coeff_seq[j] = coeffs[j][i];
            }

            let p = find_subsequence(&self.sns_cyclic[i], &coeff_seq)
                .ok_or(DecodeError::SecondarySequenceNotFound)?;
            remainders[i] = p as i64;
        }

        self.crt.solve(remainders).map(|v| v as i32)
    }
}

fn make_cyclic<T: Copy>(seq: &[T], order: usize) -> Vec<T> {
    let mut out = Vec::with_capacity(seq.len() + order.saturating_sub(1));
    out.extend_from_slice(seq);
    if order > 0 {
        out.extend_from_slice(&seq[0..order - 1]);
    }
    out
}

fn find_subsequence(haystack: &[i8], needle: &[i8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_encode_decode_patch() {
        let codec = AnotoCodec::anoto_6x6_a4_fixed();
        let patch = codec.encode_patch((10, 10), (10, 10));
        let pos = codec.decode_position(&patch).expect("decode");
        assert_eq!(pos, (10, 10));
    }

    #[test]
    fn arrow_mapping_matches_reference() {
        assert_eq!(super::arrow_to_bits('↑'), Some((0, 0)));
        assert_eq!(super::arrow_to_bits('←'), Some((1, 0)));
        assert_eq!(super::arrow_to_bits('→'), Some((0, 1)));
        assert_eq!(super::arrow_to_bits('↓'), Some((1, 1)));
    }
}
