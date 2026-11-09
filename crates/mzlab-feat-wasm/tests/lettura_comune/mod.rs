//! The synthetic HR file of `tests/dati/stima_lettura.json` (every other scan an MS2 with a precursor, deterministic), shared through `#[path]` by the parity
//! tests of the read estimate: the Rust route, `flow run` and the `.wasm` `flow_ms2` (the example `stima_lettura --fixture` writes it). The fixture also
//! holds the expected estimate and, per budget, the way: `memory` or `refused`.
#![allow(dead_code)]

use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter};
use serde_json::Value;

pub fn spec() -> Value {
    serde_json::from_str(include_str!("../../../../tests/dati/stima_lettura.json")).unwrap()
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

/// The mzML of the fixture: `scans` scans of `points` points over 100-1000, the odd ones MS2 with a precursor.
pub fn mzml(s: &Value) -> Vec<u8> {
    let n = s["scans"].as_u64().unwrap() as usize;
    let k = s["points"].as_u64().unwrap() as usize;
    let mut w = MzmlWriter::new(Vec::new(), &Header::default()).unwrap();
    w.open_spectra(n).unwrap();
    for sc in 0..n {
        let ms2 = sc % 2 == 1;
        w.spectrum(&Spectrum {
            index: sc,
            id: format!("scan={}", sc + 1),
            level: if ms2 { 2 } else { 1 },
            rt: sc as f64 * 0.005,
            polarity: 1,
            precursor: ms2.then_some(300.0 + (sc % 50) as f64),
            mz: (0..k)
                .map(|i| 100.0 + i as f64 * 900.0 / k as f64)
                .collect(),
            intensity: (0..k)
                .map(|i| 100.0 + ((sc * 31 + i * 17) % 977) as f32)
                .collect(),
            ..Default::default()
        })
        .unwrap();
    }
    w.close_spectra().unwrap();
    w.finish().unwrap()
}
