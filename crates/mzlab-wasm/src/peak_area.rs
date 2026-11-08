//! `peak_area` of the engine: the measure of the Data view against zero or a line the user sets (`mzlab_core::baseline`), the same JSON
//! bytes as `mzlab-cli peak-area`; the page reaches it through `peaks-worker.js` (`QUALITA.peakArea`).

use mzlab_core::baseline::{peak_area_json, Baseline};
use wasm_bindgen::prelude::*;

/// The area of the Data view between the times `a` and `b` against zero (`mode` 1) or the line `line` = `[t0, y0, t1, y1]` (`mode` 2): the JSON of
/// `baseline::peak_area_json`, the bytes `mzlab-cli peak-area` prints; empty for any other mode or an empty trace.
#[wasm_bindgen]
pub fn peak_area(rt: &[f64], y: &[f64], a: f64, b: f64, mode: u32, line: &[f64]) -> String {
    let base = match (mode, line) {
        (1, _) => Baseline::Zero,
        (2, &[t0, y0, t1, y1]) => Baseline::Line { t0, y0, t1, y1 },
        _ => return String::new(),
    };
    peak_area_json(rt, y, a, b, base).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    #[test]
    fn peak_area_is_the_core_text() {
        use super::{peak_area_json, Baseline};
        let (rt, y) = (vec![0.0, 0.5, 1.0, 1.5], vec![1.0, 9.0, 3.0, 1.0]);
        assert_eq!(
            super::peak_area(&rt, &y, 0.0, 1.5, 1, &[]),
            peak_area_json(&rt, &y, 0.0, 1.5, Baseline::Zero).unwrap()
        );
        let line = Baseline::Line {
            t0: 0.0,
            y0: 2.0,
            t1: 1.5,
            y1: 4.0,
        };
        assert_eq!(
            super::peak_area(&rt, &y, 0.0, 1.5, 2, &[0.0, 2.0, 1.5, 4.0]),
            peak_area_json(&rt, &y, 0.0, 1.5, line).unwrap()
        );
        assert_eq!(super::peak_area(&rt, &y, 0.0, 1.5, 0, &[]), "");
        assert_eq!(super::peak_area(&rt, &y, 0.0, 1.5, 2, &[1.0]), "");
    }
}
