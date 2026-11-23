//! `peak_area` of the engine: the measure of the Data view against zero or a line the user sets (`mzlab_core::baseline`), the same JSON
//! bytes as `mzlab-cli peak area`; the page reaches it through `peaks-worker.js` (`QUALITA.peakArea`). `window_end`: where a dragged end of the window
//! goes (`mzlab_core::intwin`, the bytes of `mzlab-cli peak window`, `QUALITA.windowEnd`). `lr_auto_integrate`: every peak of a low-resolution trace
//! (`mzlab_core::peakq_lrauto`, the bytes of `mzlab-cli peak auto-lr`, `QUALITA.lrAuto`). `noise_floor`: the noise with a floor of one ion
//! (`mzlab_core::ionstat`, the bytes of `mzlab-cli peak noise`, `QUALITA.noiseFloor`).

use mzlab_core::area_unit::{lr_auto_json_in, AreaUnit};
use mzlab_core::baseline::{peak_area_json, Baseline};
use mzlab_core::intwin::move_end_json;
use mzlab_core::ionstat::{floor_json, measure, FloorAsk, FloorParams};
use mzlab_core::peakq_lrauto::{lr_auto_integrate as lr_auto, LrAutoParams};
use wasm_bindgen::prelude::*;

/// The area of the Data view between the times `a` and `b` against zero (`mode` 1) or the line `line` = `[t0, y0, t1, y1]` (`mode` 2): the JSON of
/// `baseline::peak_area_json`, the bytes `mzlab-cli peak area` prints; empty for any other mode or an empty trace.
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
/// `mzlab-cli peak auto-lr` prints; `params` = the numbers of the dialog (`LrAutoParams::from_slice`).
#[wasm_bindgen]
pub fn lr_auto_integrate(rt: &[f64], y: &[f64], x0: f64, x1: f64, params: &[f64]) -> String {
    let n = rt.len().min(y.len());
    let r = lr_auto(&rt[..n], &y[..n], x0, x1, LrAutoParams::from_slice(params));
    lr_auto_json_in(&rt[..n], &r, AreaUnit::from_number(params.get(6).copied()))
}

/// The noise of the trace with a floor of one ion and the S/N of a peak (`ionstat::floor_json`, the bytes `mzlab-cli peak noise` prints):
/// `ask` = [from, to, peak start, peak end] (min, NaN = not given), `params` = [one_ion (NaN or < 0 auto, 0 off, cps), sd_weight, min_sn];
/// "E" + `{"key"}` when no background point is left.
#[wasm_bindgen]
pub fn noise_floor(rt: &[f64], y: &[f64], ask: &[f64], params: &[f64]) -> String {
    let n = rt.len().min(y.len());
    match measure(
        &rt[..n],
        &y[..n],
        FloorAsk::from_slice(ask),
        FloorParams::from_slice(params),
    ) {
        Ok(r) => floor_json(&r),
        Err(key) => format!("E{{\"key\":\"{key}\"}}"),
    }
}

/// `api/manauto`: every peak of every channel of every sample, found by the engine and grouped by time (`manint::auto_text`): `{"windows","log","groups",
/// "unmatched"}`; `params` is `{}` or the engine's numbers; "E" + JSON = the user's error. Here, not in `rotte`: `lr_auto_integrate` is already in this module.
#[wasm_bindgen]
pub fn manual_auto(files: &str, params: &str) -> String {
    match mzlab_core::manint::auto_text(files, params) {
        Ok(json) => json,
        Err(e) => format!("E{}", e.to_json()),
    }
}

#[cfg(test)]
#[path = "peak_area_tests.rs"]
mod tests;
