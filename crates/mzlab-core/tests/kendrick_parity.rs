//! Parity of Kendrick mass defect analysis with `tests/golden/kmd.json` and null decoy simulation.

use mzlab_core::kendrick::{analyse, kendrick_json, units, Ion, KmdOpts};
use serde_json::Value;
use std::path::PathBuf;

fn golden() -> Value {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/kmd.json");
    serde_json::from_str(&std::fs::read_to_string(p).expect("kmd.json")).unwrap()
}

fn f(v: &Value) -> f64 {
    v.as_f64().unwrap()
}

fn close(a: f64, b: f64, tol: f64) {
    assert!((a - b).abs() <= tol, "{a} vs {b}");
}

#[test]
fn the_kmd_matches_the_golden_of_the_javascript_reference() {
    let g = golden();
    let u_list = units();
    for (name, case) in g.as_object().unwrap() {
        let inp = &case["input"];
        let ions: Vec<Ion> = inp["features"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| Ion {
                mz: f(&x["mz"]),
                rt: x["rt"].as_f64(),
            })
            .collect();
        let mut o = KmdOpts::default();
        o.unit = u_list
            .iter()
            .find(|u| Some(u.id) == inp["unit"].as_str())
            .map_or("CF2", |u| u.id);
        o.tol_mda = inp["tolMDa"].as_f64().unwrap_or(o.tol_mda);
        o.min_members = inp["minMembers"]
            .as_u64()
            .map_or(o.min_members, |n| n as usize);

        let r = analyse(&ions, &o);
        let rows = case["rows"].as_array().unwrap();
        assert_eq!(r.rows.len(), rows.len(), "{name}");
        for (a, b) in r.rows.iter().zip(rows) {
            close(a.km, f(&b["km"]), 1e-9);
            assert_eq!(a.nkm, f(&b["nkm"]), "{name}");
            close(a.kmd, f(&b["kmd"]), 1e-9);
            assert_eq!(a.series.map(|s| s as u64), b["series"].as_u64(), "{name}");
        }
        let series = case["series"].as_array().unwrap();
        assert_eq!(r.series.len(), series.len(), "{name}");
        for (a, b) in r.series.iter().zip(series) {
            let members: Vec<u64> = b["members"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_u64().unwrap())
                .collect();
            assert_eq!(
                a.members.iter().map(|m| *m as u64).collect::<Vec<_>>(),
                members,
                "{name}"
            );
            let steps: Vec<f64> = b["steps"].as_array().unwrap().iter().map(f).collect();
            assert_eq!(a.steps, steps, "{name}");
            close(a.kmd, f(&b["kmd"]), 1e-9);
            close(a.span_mda, f(&b["spanMDa"]), 1e-6);
            close(a.consecutive, f(&b["consecutive"]), 1e-12);
            assert_eq!(a.distinct as u64, b["distinct"].as_u64().unwrap());
            assert_eq!(a.rho.is_some(), b["rho"].as_f64().is_some(), "{name}");
            if let (Some(x), Some(y)) = (a.rho, b["rho"].as_f64()) {
                close(x, y, 1e-12);
            }
        }

        // Parity of JSON route
        let json_str = kendrick_json(&inp.to_string()).unwrap();
        let jv: Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(jv["rows"].as_array().unwrap().len(), rows.len());
        assert_eq!(jv["series"].as_array().unwrap().len(), series.len());
    }
}

/// Deterministic 64-bit XorShift PRNG for reproducible decoy generation.
struct Rng(u64);
impl Rng {
    fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

#[test]
fn decoy_test_200_random_mz_produces_no_spurious_consecutive_series() {
    let mut rng = Rng(0x1234_5678_9ABC_DEF0);
    // 200 random m/z between 100.0 and 900.0 with random RT between 1.0 and 20.0
    let ions: Vec<Ion> = (0..200)
        .map(|_| Ion {
            mz: 100.0 + 800.0 * rng.next_f64(),
            rt: Some(1.0 + 19.0 * rng.next_f64()),
        })
        .collect();

    let opts = KmdOpts {
        unit: "CF2",
        tol_mda: 3.0,
        min_members: 3,
    };
    let out = analyse(&ions, &opts);

    // Any series found purely by random coincidence must not have strictly consecutive unit steps (consecutive == 1.0)
    let consecutive_series = out
        .series
        .iter()
        .filter(|s| s.distinct >= opts.min_members && (s.consecutive - 1.0).abs() < 1e-12)
        .count();
    assert_eq!(
        consecutive_series, 0,
        "random decoys should not form series with consecutive repeating units"
    );

    // Broad threshold on casual non-consecutive series (31 found for this seed)
    assert!(
        out.series.len() <= 50,
        "unexpected number of casual series: {}",
        out.series.len()
    );
}
