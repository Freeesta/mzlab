//! The synthetic HR file of `tests/dati/stima_detect.json` (noise and a few Gaussian ions, deterministic), shared through `#[path]` by the parity tests of
//! the detection estimate: the Rust route, `flow run`, the app's `feat/estimate_detect` and the `.wasm` `estimate_detect` (the example `stima_detect`
//! writes its index). The fixture also holds the expected estimate and, per budget, the way: `memory` or `refused`.
#![allow(dead_code)]

use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter};
use serde_json::Value;

const FULL: &str = "FTMS + p ESI Full ms [100.0000-1000.0000]";

pub fn spec() -> Value {
    serde_json::from_str(include_str!("../../../../tests/dati/stima_detect.json")).unwrap()
}

/// The budgets of the fixture with their way: `(budget_mb, "memory" | "refused")`.
pub fn cases(s: &Value) -> Vec<(f64, String)> {
    s["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| {
            (
                c["budget_mb"].as_f64().unwrap(),
                c["via"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

/// The mzML of the fixture: one MS1 scan every 0.01 min, `noise_per_scan` peaks spread over 100-1000 by a golden-ratio walk and the ions as Gaussians
/// of 4 scans around their apex scan.
pub fn mzml(s: &Value) -> Vec<u8> {
    let n = s["scans"].as_u64().unwrap() as usize;
    let noise = s["noise_per_scan"].as_u64().unwrap() as usize;
    let level = s["noise_intensity"].as_f64().unwrap();
    let ions: Vec<[f64; 3]> = s["ions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| [0, 1, 2].map(|k| i[k].as_f64().unwrap()))
        .collect();
    let mut w = MzmlWriter::new(Vec::new(), &Header::default()).unwrap();
    w.open_spectra(n).unwrap();
    for sc in 0..n {
        let mut peaks: Vec<(f64, f32)> = (0..noise)
            .map(|k| {
                let u = ((sc * noise + k) as f64 * 0.618_033_988_75).fract();
                (100.0 + 900.0 * u, (level * (0.5 + u)) as f32)
            })
            .collect();
        for [mz, apex, h] in &ions {
            let y = h * (-0.5 * ((sc as f64 - apex) / 4.0).powi(2)).exp();
            if y > level {
                peaks.push((*mz, y as f32));
            }
        }
        peaks.sort_by(|a, b| a.0.total_cmp(&b.0));
        w.spectrum(&Spectrum {
            index: sc,
            id: format!("scan={}", sc + 1),
            level: 1,
            rt: sc as f64 * 0.01,
            polarity: 1,
            filter: FULL.into(),
            mz: peaks.iter().map(|p| p.0).collect(),
            intensity: peaks.iter().map(|p| p.1).collect(),
            ..Default::default()
        })
        .unwrap();
    }
    w.close_spectra().unwrap();
    w.finish().unwrap()
}

/// The `.mzidx` of `mzml(s)`, built as an upload is.
pub fn index(s: &Value) -> Vec<u8> {
    let b = mzml(s);
    let mut src = b.clone();
    let p = mzlab_core::upload::prepare(&mut src, b.len() as u64, "stima.mzML").unwrap();
    mzlab_core::mzidx::build_mzml_to_vec(&p.table, &b, 64 << 20)
        .unwrap()
        .0
}
