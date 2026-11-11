//! `peak_area` of the engine: the measure of the Data view against zero or a line the user sets (`mzlab_core::baseline`), the same JSON
//! bytes as `mzlab-cli peak-area`; the page reaches it through `peaks-worker.js` (`QUALITA.peakArea`). `window_end`: where a dragged end of the window
//! goes (`mzlab_core::intwin`, the bytes of `mzlab-cli peak-window`, `QUALITA.windowEnd`).

use mzlab_core::baseline::{peak_area_json, Baseline};
use mzlab_core::intwin::move_end_json;
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

/// The point one end of the window goes to (`intwin::move_end_json`): `cur` that end, `other` the other one, `at` the pointer (NaN: move `step`
/// points, the keyboard), `lo`/`hi` the borders of the neighbouring integrations (NaN: none). Empty when no point is free.
#[wasm_bindgen]
pub fn window_end(
    rt: &[f64],
    cur: f64,
    other: f64,
    at: f64,
    step: i32,
    lo: f64,
    hi: f64,
) -> String {
    let opt = |v: f64| (!v.is_nan()).then_some(v);
    move_end_json(rt, cur, other, opt(at), step as i64, opt(lo), opt(hi)).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    #[test]
    fn window_end_is_the_core_text() {
        let rt = [0.0, 0.1, 0.2, 0.3, 0.4];
        assert_eq!(
            super::window_end(&rt, 0.0, 0.4, 0.18, 0, f64::NAN, f64::NAN),
            "{\"t\":0.2,\"i\":2}"
        );
        assert_eq!(
            super::window_end(&rt, 0.4, 0.0, f64::NAN, -1, f64::NAN, f64::NAN),
            "{\"t\":0.3,\"i\":3}"
        );
        assert_eq!(
            super::window_end(&rt, 0.3, 0.4, 0.0, 0, 0.1, f64::NAN),
            "{\"t\":0.1,\"i\":1}"
        );
        assert_eq!(
            super::window_end(&rt[..1], 0.0, 0.0, 0.0, 0, f64::NAN, f64::NAN),
            ""
        );
    }

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
