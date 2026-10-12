//! Golden traces of the peak finders (`tests/golden/picchi_qualita.json`, `tools/golden_picchi.py`): the CWT must give the limits of the
//! Python engine (indices identical, areas 1e-4); "prominence + local minimum" has no Python twin, so its limits are asserted here.

use std::collections::BTreeMap;

use mzlab_core::peakq::{detect_prominence, ProminenceParams};
use mzlab_core::peaks::{detect_peaks, Params, Peak};
use serde::Deserialize;

#[derive(Deserialize)]
struct GPeak {
    apex_i: usize,
    lo: usize,
    hi: usize,
    area: f64,
}
#[derive(Deserialize)]
struct Case {
    rt: Vec<f64>,
    y: Vec<f64>,
    cwt: Vec<GPeak>,
}

fn load() -> BTreeMap<String, Case> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/golden/picchi_qualita.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).expect("tests/golden/picchi_qualita.json"))
        .unwrap()
}

fn by_apex(mut v: Vec<Peak>) -> Vec<Peak> {
    v.sort_by_key(|p| p.apex_i);
    v
}

#[test]
fn cwt_keeps_the_limits_of_the_python_engine() {
    for (name, c) in load() {
        let got = by_apex(detect_peaks(&c.rt, &c.y, Params::default()));
        assert_eq!(got.len(), c.cwt.len(), "{name}: {got:?}");
        for (g, e) in got.iter().zip(&c.cwt) {
            assert_eq!((g.apex_i, g.lo, g.hi), (e.apex_i, e.lo, e.hi), "{name}");
            assert!(
                (g.area / e.area - 1.0).abs() < 1e-4,
                "{name}: area {} vs {}",
                g.area,
                e.area
            );
        }
    }
}

#[test]
fn prominence_finds_the_expected_peaks_and_limits() {
    for (name, c) in load() {
        let got = by_apex(detect_prominence(&c.rt, &c.y, ProminenceParams::default()));
        let cwt = by_apex(detect_peaks(&c.rt, &c.y, Params::default()));
        match name.as_str() {
            "singolo" | "coda" | "spalla" => assert_eq!(got.len(), 1, "{name}: {got:?}"),
            "doppio" | "doppio_stretto" => {
                assert_eq!(got.len(), 2, "{name}: {got:?}");
                assert_eq!(
                    got[0].hi, got[1].lo,
                    "{name}: the limits meet at the valley"
                );
                // the CWT integrates the points between the two valleys twice, the valley limits never do
                assert!(cwt.len() == 2 && cwt[0].hi > cwt[1].lo, "{name}: {cwt:?}");
                let total: f64 = got.iter().map(|p| p.area).sum();
                let whole: f64 = c.y.iter().sum::<f64>() * 0.015;
                assert!(total < whole, "{name}");
            }
            other => panic!("unexpected case {other}"),
        }
    }
}
