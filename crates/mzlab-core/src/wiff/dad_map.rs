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

/// The values to send and their shape: the matrix as read (borrowed), or averaged into at most `MAP_MAX_RT` x `MAP_MAX_NM` bins. Also the block size along the wavelengths and whether it was reduced.
fn map_values(dad: &DadData) -> (std::borrow::Cow<'_, [f32]>, usize, usize, usize, bool) {
    let (nt, nl) = (dad.points(), dad.channels());
    let (ft, fl) = (nt.div_ceil(MAP_MAX_RT), nl.div_ceil(MAP_MAX_NM));
    if ft > 1 || fl > 1 {
        let (v, rt, rl) = reduce(&dad.matrix_mau, nt, nl, ft, fl);
        (v.into(), rt, rl, fl, true)
    } else {
        (dad.matrix_mau.as_slice().into(), nt, nl, 1, false)
    }
}

/// Appends the values as little-endian `f32`, base64, to `out`: in pieces of a multiple of 3 bytes, so no padding falls inside and no copy of the
/// whole array is made.
fn encode_f32(vals: &[f32], out: &mut String) {
    use base64::Engine;
    // 3 values = 12 bytes, a multiple of 3
    const PIECE: usize = 3 * 1024;
    let mut bytes = Vec::with_capacity(PIECE * 4);
    out.reserve(vals.len().saturating_mul(4).div_ceil(3) * 4);
    for piece in vals.chunks(PIECE) {
        bytes.clear();
        for &val in piece {
            bytes.extend_from_slice(&val.to_le_bytes());
        }
        base64::engine::general_purpose::STANDARD.encode_string(&bytes, out);
    }
}

/// Encodes the 2D absorbance matrix of `dad` as JSON for `/api/map?kind=pda`; `agg` says whether it was reduced (`mean`) or sent as read (`none`).
pub fn dad_to_map_json(dad: &DadData) -> String {
    use std::fmt::Write;
    let (rt0, rt1) = (
        dad.time_min.first().copied().unwrap_or(0.0),
        dad.time_min.last().copied().unwrap_or(dad.duration_min),
    );
    let (vals, nrt, nmz, fl, reduced) = map_values(dad);
    let mut out = String::from("{\"data\":\"");
    encode_f32(&vals, &mut out);
    let _ = write!(
        out,
        "\",\"rt0\":{rt0},\"rt1\":{rt1},\"nrt\":{nrt},\"mz0\":{},\"mz1\":{},\"nmz\":{nmz},\"dmz\":{},\"level\":1,\"region\":false,\"agg\":\"{}\",\"log\":false,\"bg\":false,\"hr\":false,\"kind\":\"pda\"}}",
        dad.wl_start_nm,
        dad.wl_stop_nm,
        dad.wl_step_nm * fl as f64,
        if reduced { "mean" } else { "none" }
    );
    out
}

#[cfg(test)]
#[path = "dad_map_tests.rs"]
mod tests;
