//! `peak_area` of the engine: the measure of the Data view against zero or a line the user sets (`mzlab_core::baseline`), the same JSON
//! bytes as `mzlab-cli peak-area`; the page reaches it through `peaks-worker.js` (`QUALITA.peakArea`). `window_end`: where a dragged end of the window
//! goes (`mzlab_core::intwin`, the bytes of `mzlab-cli peak-window`, `QUALITA.windowEnd`). `lr_auto_integrate`: every peak of a low-resolution trace
//! (`mzlab_core::peakq_lrauto`, the bytes of `mzlab-cli peak-auto-lr`, `QUALITA.lrAuto`).

use mzlab_core::baseline::{peak_area_json, Baseline};
use mzlab_core::intwin::move_end_json;
use mzlab_core::peakq_lrauto::{lr_auto_integrate as lr_auto, lr_auto_json, LrAutoParams};
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

/// Every peak of a low-resolution trace with its apex between `x0` and `x1` (min): the JSON of `peakq_lrauto::lr_auto_json`, the bytes
/// `mzlab-cli peak-auto-lr` prints; `params` = the numbers of the dialog (`LrAutoParams::from_slice`).
#[wasm_bindgen]
pub fn lr_auto_integrate(rt: &[f64], y: &[f64], x0: f64, x1: f64, params: &[f64]) -> String {
    let n = rt.len().min(y.len());
    let r = lr_auto(&rt[..n], &y[..n], x0, x1, LrAutoParams::from_slice(params));
    lr_auto_json(&rt[..n], &r)
}

#[cfg(test)]
mod tests {
    #[test]
    fn lr_auto_integrate_is_the_core_text() {
        use mzlab_core::peakq_lrauto::{lr_auto_integrate, lr_auto_json, LrAutoParams};
        let rt: Vec<f64> = (0..280).map(|i| f64::from(i) * 0.025).collect();
        let g = |t: f64, c: f64, h: f64| h * (-((t - c) / 0.06).powi(2) / 2.0).exp();
        let y: Vec<f64> = rt
            .iter()
            .enumerate()
            .map(|(i, &t)| {
                g(t, 2.0, 2.0e4)
                    + g(t, 2.24, 1.2e4)
                    + 200.0
                    + f64::from((i * 7919 % 13) as u32) * 9.0
            })
            .collect();
        let p = LrAutoParams {
            half: 3,
            ..Default::default()
        };
        let want = lr_auto_json(&rt, &lr_auto_integrate(&rt, &y, 0.0, 7.0, p));
        assert_eq!(super::lr_auto_integrate(&rt, &y, 0.0, 7.0, &[3.0]), want);
        assert!(want.contains("\"end1\":\"valley\""), "{want}");
    }

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
        // dragged past its second point the line is held at 0: the page gets the text with `clamped`, the same as the CLI (`peak-area`)
        let (rt, y) = (
            (0..40).map(|i| f64::from(i) * 0.1).collect::<Vec<_>>(),
            vec![5.0; 40],
        );
        let ext = Baseline::Line {
            t0: 1.0,
            y0: 4.0,
            t1: 2.0,
            y1: 1.0,
        };
        let held = super::peak_area(&rt, &y, 1.0, 3.5, 2, &[1.0, 4.0, 2.0, 1.0]);
        assert_eq!(held, peak_area_json(&rt, &y, 1.0, 3.5, ext).unwrap());
        assert!(held.contains("\"clamped\""), "{held}");
        assert_eq!(super::peak_area(&rt, &y, 0.0, 1.5, 0, &[]), "");
        assert_eq!(super::peak_area(&rt, &y, 0.0, 1.5, 2, &[1.0]), "");
    }
}
