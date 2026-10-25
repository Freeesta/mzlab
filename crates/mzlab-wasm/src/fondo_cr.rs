//! Automatic background of a chromatogram for the page (grafici.js through qualita.js): wasm-bindgen wrapper of `mzlab_core::chrom_fondo`.

use wasm_bindgen::prelude::*;

/// The steadiest flat window of a trace (`chrom_fondo::flat_window`): `[first index, end index (exclusive), median]` into `out`.
/// `win_frac` = window width as a fraction of the run (0 = the default), `min_pts` = smallest window (0 = the default). Returns 3, or 0 when the trace is too short.
#[wasm_bindgen]
pub fn flat_window(y: &[f64], win_frac: f64, min_pts: u32, out: &mut [f64]) -> u32 {
    let mut p = mzlab_core::chrom_fondo::FondoParams::default();
    if win_frac > 0.0 {
        p.win_frac = win_frac;
    }
    if min_pts > 0 {
        p.min_pts = min_pts as usize;
    }
    match (
        mzlab_core::chrom_fondo::flat_window(y, &p),
        out.get_mut(..3),
    ) {
        (Some((s, e, m)), Some(o)) => {
            o.copy_from_slice(&[s as f64, e as f64, m]);
            3
        }
        _ => 0,
    }
}
