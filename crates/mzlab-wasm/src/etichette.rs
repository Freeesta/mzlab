//! Labels of the main peaks of a chromatogram for the page (etichette-picchi.js through qualita.js): wasm-bindgen wrappers of `mzlab_core::etichette_picchi`.

use wasm_bindgen::prelude::*;

/// Fields per label written by `label_peaks`: apex index, RT, height, S/N.
pub const LABEL_FIELDS: usize = 4;

/// The peaks of a trace that deserve a label (`etichette_picchi::select`): records of `LABEL_FIELDS` numbers by increasing RT; returns how many fit in `out`.
#[wasm_bindgen]
pub fn label_peaks(
    rt: &[f64],
    y: &[f64],
    max_n: u32,
    rel_min: f64,
    snr_min: f64,
    out: &mut [f64],
) -> u32 {
    let found = mzlab_core::etichette_picchi::select(rt, y, max_n as usize, rel_min, snr_min);
    let room = out.len() / LABEL_FIELDS;
    for (i, l) in found.iter().take(room).enumerate() {
        out[i * LABEL_FIELDS..(i + 1) * LABEL_FIELDS].copy_from_slice(&[
            l.apex_i as f64,
            l.rt,
            l.height,
            l.snr,
        ]);
    }
    found.len().min(room) as u32
}

/// Tallest point of a spectrum (`etichette_picchi::main_peak`): `[x, y]` into `out`; returns 2, or 0 when there is none. `refine` = parabola (UV curves).
#[wasm_bindgen]
pub fn main_peak(x: &[f64], y: &[f64], refine: bool, out: &mut [f64]) -> u32 {
    match mzlab_core::etichette_picchi::main_peak(x, y, refine) {
        Some((a, b)) if out.len() >= 2 => {
            out[..2].copy_from_slice(&[a, b]);
            2
        }
        _ => 0,
    }
}
