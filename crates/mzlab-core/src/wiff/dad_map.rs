//! Serialization of the 2D optical DAD / PDA absorbance matrix into the mzLab map format (`/api/map?kind=pda`, native and browser alike).
//! A matrix larger than `MAP_MAX_RT` x `MAP_MAX_NM` is averaged into that many bins first, so the answer has a bounded size.

use super::dad::DadData;

/// Most time bins of the map; a longer run is averaged in groups of consecutive rows.
pub const MAP_MAX_RT: usize = 2000;
/// Most wavelength bins of the map; a finer matrix is averaged in groups of consecutive channels.
pub const MAP_MAX_NM: usize = 500;

/// `m` (`nt` rows x `nl` columns, row-major) averaged in blocks of `ft` rows x `fl` columns; the last block of each axis may be smaller.
/// Returns the new matrix with its row and column counts.
pub fn reduce(m: &[f32], nt: usize, nl: usize, ft: usize, fl: usize) -> (Vec<f32>, usize, usize) {
    let (ft, fl) = (ft.max(1), fl.max(1));
    let (rt, rl) = (nt.div_ceil(ft), nl.div_ceil(fl));
    let mut out = Vec::with_capacity(rt * rl);
    for i in 0..rt {
        let (a, b) = (i * ft, ((i + 1) * ft).min(nt));
        for j in 0..rl {
            let (c, d) = (j * fl, ((j + 1) * fl).min(nl));
            let mut sum = 0.0f64;
            for r in a..b {
                sum += m
                    .get(r * nl + c..r * nl + d)
                    .unwrap_or_default()
                    .iter()
                    .map(|&v| f64::from(v))
                    .sum::<f64>();
            }
            // the mean of f32 values fits f32
            #[allow(clippy::cast_possible_truncation)]
            out.push((sum / ((b - a) * (d - c)) as f64) as f32);
        }
    }
    (out, rt, rl)
}

/// The values to send and their shape: the matrix as read, or averaged into at most `MAP_MAX_RT` x `MAP_MAX_NM` bins. Also the block size along the wavelengths and whether it was reduced.
fn map_values(dad: &DadData) -> (Vec<f32>, usize, usize, usize, bool) {
    let (nt, nl) = (dad.points(), dad.channels());
    let (ft, fl) = (nt.div_ceil(MAP_MAX_RT), nl.div_ceil(MAP_MAX_NM));
    if ft > 1 || fl > 1 {
        let (v, rt, rl) = reduce(&dad.matrix_mau, nt, nl, ft, fl);
        (v, rt, rl, fl, true)
    } else {
        (dad.matrix_mau.clone(), nt, nl, 1, false)
    }
}

/// The values as little-endian `f32`, base64.
fn encode_f32(vals: &[f32]) -> String {
    use base64::Engine;
    let mut bytes = Vec::with_capacity(vals.len().saturating_mul(4));
    for &val in vals {
        bytes.extend_from_slice(&val.to_le_bytes());
    }
    base64::engine::general_purpose::STANDARD.encode(&bytes)
}

/// Encodes the 2D absorbance matrix of `dad` as JSON for `/api/map?kind=pda`; `agg` says whether it was reduced (`mean`) or sent as read (`none`).
pub fn dad_to_map_json(dad: &DadData) -> String {
    let (rt0, rt1) = (
        dad.time_min.first().copied().unwrap_or(0.0),
        dad.time_min.last().copied().unwrap_or(dad.duration_min),
    );
    let (vals, nrt, nmz, fl, reduced) = map_values(dad);
    let b64_data = encode_f32(&vals);
    format!(
        "{{\"data\":\"{b64_data}\",\"rt0\":{rt0},\"rt1\":{rt1},\"nrt\":{nrt},\"mz0\":{},\"mz1\":{},\"nmz\":{nmz},\"dmz\":{},\"level\":1,\"region\":false,\"agg\":\"{}\",\"log\":false,\"bg\":false,\"hr\":false,\"kind\":\"pda\"}}",
        dad.wl_start_nm,
        dad.wl_stop_nm,
        dad.wl_step_nm * fl as f64,
        if reduced { "mean" } else { "none" }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dad(nt: usize, nl: usize) -> DadData {
        DadData {
            time_min: (0..nt).map(|i| i as f64 * 0.01).collect(),
            twc_mau: vec![0.0; nt],
            wavelengths_nm: (0..nl).map(|j| 200.0 + j as f64).collect(),
            wl_start_nm: 200.0,
            wl_stop_nm: 200.0 + (nl - 1) as f64,
            wl_step_nm: 1.0,
            duration_min: nt as f64 * 0.01,
            matrix_mau: (0..nt * nl).map(|k| (k % 97) as f32).collect(),
        }
    }

    #[test]
    fn small_matrix_is_sent_as_read() {
        let j = dad_to_map_json(&dad(40, 21));
        assert!(
            j.contains("\"nrt\":40")
                && j.contains("\"nmz\":21")
                && j.contains("\"agg\":\"none\"")
                && j.contains("\"dmz\":1,")
        );
    }

    #[test]
    fn large_matrix_is_averaged_within_the_caps() {
        let (nt, nl) = (5500, 1201);
        let j = dad_to_map_json(&dad(nt, nl));
        let (ft, fl) = (nt.div_ceil(MAP_MAX_RT), nl.div_ceil(MAP_MAX_NM));
        assert!(
            j.contains(&format!("\"nrt\":{}", nt.div_ceil(ft)))
                && j.contains(&format!("\"nmz\":{}", nl.div_ceil(fl)))
        );
        assert!(
            nt.div_ceil(ft) <= MAP_MAX_RT
                && nl.div_ceil(fl) <= MAP_MAX_NM
                && j.contains("\"agg\":\"mean\"")
                && j.contains(&format!("\"dmz\":{fl},"))
        );
    }

    #[test]
    fn reduce_averages_blocks_and_keeps_the_edge() {
        // 3 x 3 in blocks of 2 x 2: the last row and column are blocks of one
        let m: Vec<f32> = (1..=9).map(|v| v as f32).collect();
        let (r, rt, rl) = reduce(&m, 3, 3, 2, 2);
        assert_eq!((rt, rl), (2, 2));
        assert_eq!(r, vec![3.0, 4.5, 7.5, 9.0]);
    }
}
