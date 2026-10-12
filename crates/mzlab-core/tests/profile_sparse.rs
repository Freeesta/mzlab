//! `/api/spectrum` on a profile level whose scans are mostly a few points (a converter that leaves the zero bins out, as the `.wiff` reader does):
//! the spacing of the points is read from the scans that have enough of them, wherever they are, so the route answers instead of giving the
//! level to Python (the page has no Python: the spectrum panel stayed blank). Synthetic data only.

use mzlab_core::idxread::Index;
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter};
use mzlab_core::routes::{Query, Routes};
use mzlab_core::upload::prepare;

const STEP: f64 = 0.06;

fn scan(i: usize, prec: f64, dense: bool) -> Spectrum {
    let n = if dense { 30 } else { 1 + i % 4 };
    let first = 100 + (i % 7) as usize;
    let mz: Vec<f64> = (0..n).map(|b| 50.0 + (first + b) as f64 * STEP).collect();
    let intensity: Vec<f32> = (0..n).map(|b| 10.0 + (i + b) as f32).collect();
    Spectrum {
        index: i,
        id: format!("controllerType=0 controllerNumber=1 scan={}", i + 1),
        level: 2,
        rt: 0.05 * i as f64,
        polarity: 1,
        profile: true,
        filter: String::new(),
        precursor: Some(prec),
        mz,
        intensity,
        wide: None,
    }
}

/// Two precursors alternating; the only scans with more than 20 points are the first ones, far from the middle of the run.
fn routes() -> Routes<Vec<u8>> {
    let mut w = MzmlWriter::new(Vec::new(), &Header { run: "sparse".into(), ..Default::default() }).unwrap();
    let total = 100;
    w.open_spectra(total).unwrap();
    for i in 0..total {
        w.spectrum(&scan(i, if i % 5 == 1 { 305.0 } else { 229.1 }, i < 6)).unwrap();
    }
    w.close_spectra().unwrap();
    let mzml = w.finish().unwrap();
    let size = mzml.len() as u64;
    let mut src = mzml.clone();
    let p = prepare(&mut src, size, "sparse.mzML").unwrap();
    let (idx, _) = build_mzml_to_vec(&p.table, &mzml, 64 << 20).unwrap();
    let len = idx.len() as u64;
    Routes::new(Index::open(idx, len).unwrap())
}

#[test]
fn sparse_profile_level_still_answers_the_spectrum() {
    let mut r = routes();
    for (prec, from, to) in [("229.1", 1.0, 4.0), ("305", 1.0, 4.0), ("229.1", 0.0, 5.0)] {
        let q = format!("k=0&rt0={from}&rt1={to}&level=2&precursor={prec}");
        let got = r.handle("/api/spectrum", &Query::parse(&q)).unwrap();
        let text = got.unwrap_or_else(|| panic!("{q}: gave the spectrum to Python"));
        let j: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert!(!j["mz"].as_array().unwrap().is_empty(), "{q}: empty spectrum");
        assert!(j["scans"].as_i64().unwrap() > 0, "{q}: no scans");
    }
}
