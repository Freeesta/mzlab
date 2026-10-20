//! Drift correction (`drift.rs`): the factors, curves and CVs agree with the frozen answers of `tools/deriva_riferimento.py` to 1e-9, the correction is
//! refused with too few QC files or QC files that are not at the ends of a batch, it lowers the CV of the QC on a series with a known drift, and it never
//! changes the measured areas.

use std::path::{Path, PathBuf};

use mzlab_core::disegno::{Disegno, Lotto};
use mzlab_core::json::parse;
use mzlab_feat::drift::{self, Method};
use mzlab_feat::samples::{propose, Sample};
use mzlab_feat::table::{Aligned, CELL_ABSENT, CELL_DETECTED};
use serde_json::Value;

fn golden() -> Value {
    let p: PathBuf =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/feat/deriva.json");
    serde_json::from_slice(&std::fs::read(&p).unwrap_or_else(|e| panic!("{p:?}: {e}"))).unwrap()
}

/// A table from rows of areas (0 = no peak) and samples from `(kind, order, batch)`.
fn build(
    files: &[(String, u32, String)],
    areas: &[Vec<f64>],
) -> (Aligned, Vec<Sample>, Vec<String>) {
    let n = files.len();
    let names: Vec<(String, String)> = (0..n)
        .map(|k| (format!("f{k}.mzML"), format!("{k:02x}").repeat(32)))
        .collect();
    let mut s = propose(&names);
    for (k, (kind, order, _)) in files.iter().enumerate() {
        s[k].kind = kind.clone();
        s[k].order = *order;
    }
    let mut t = Aligned::with_capacity(n, areas.len());
    for (r, row) in areas.iter().enumerate() {
        t.mz.push(200.0 + r as f64);
        t.rt.push(5.0);
        for &a in row {
            t.area.push(a);
            t.height.push((a / 10.0) as f32);
            t.apex_rt.push(5.0);
            t.width.push(0.1);
            t.state
                .push(if a > 0.0 { CELL_DETECTED } else { CELL_ABSENT });
        }
    }
    (t, s, files.iter().map(|f| f.2.clone()).collect())
}

fn golden_series() -> (Aligned, Vec<Sample>, Vec<String>, Value) {
    let g = golden();
    let files: Vec<(String, u32, String)> = g["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["kind"].as_str().unwrap().into(),
                f["order"].as_u64().unwrap() as u32,
                f["batch"].as_str().unwrap().into(),
            )
        })
        .collect();
    let areas: Vec<Vec<f64>> = g["areas"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            r.as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_f64().unwrap())
                .collect()
        })
        .collect();
    let (t, s, b) = build(&files, &areas);
    (t, s, b, g)
}

fn near(got: f64, want: &Value, what: &str) {
    match want.as_f64() {
        None => assert!(got.is_nan(), "{what}: {got} against null"),
        Some(w) => assert!(
            (got - w).abs() <= 1e-9 * w.abs().max(1.0),
            "{what}: {got} against {w}"
        ),
    }
}

#[test]
fn the_correction_agrees_with_the_reference() {
    let (t, s, b, g) = golden_series();
    let n = t.n_files;
    for case in g["cases"].as_array().unwrap() {
        let method = match case["method"].as_str().unwrap() {
            "linear" => Method::Linear,
            _ => Method::Loess(case["span"].as_f64().unwrap()),
        };
        let d = drift::correct(&t, &s, &b, method).unwrap();
        for r in 0..t.len() {
            near(d.reference[r], &case["reference"][r], "reference");
            near(d.cv_before[r], &case["cv_before"][r], "cv_before");
            near(d.cv_after[r], &case["cv_after"][r], "cv_after");
            for k in 0..n {
                near(d.factor[r * n + k], &case["factor"][r][k], "factor");
                near(d.curve[r * n + k], &case["curve"][r][k], "curve");
            }
        }
    }
    // a line needs three QC peaks in a batch: row 4 has none in batch 1, so those cells have no factor
    let d = drift::correct(&t, &s, &b, Method::Linear).unwrap();
    assert!(d.factor[4 * n].is_nan() && d.factor[4 * n + 10].is_finite());
}

#[test]
fn the_correction_is_refused_without_enough_qc_files_at_the_ends() {
    let (t, mut s, b, _) = golden_series();
    // 4 QC files in batch 1
    let first = (0..s.len()).find(|&k| s[k].kind == "qc").unwrap();
    s[first].kind = "sample".into();
    let e = drift::correct(&t, &s, &b, Method::Linear).unwrap_err();
    assert_eq!(e.key, "error.drift.tooFewQc");
    // 5 QC files but the last one injected in batch 1 is a sample
    let (t, mut s, b, _) = golden_series();
    let last = (0..s.len())
        .filter(|&k| b[k] == "1" && s[k].kind == "qc")
        .max_by_key(|&k| s[k].order)
        .unwrap();
    let other =
        (0..s.len()).find(|&k| b[k] == "1" && s[k].kind == "sample" && s[k].order > s[last].order);
    assert!(other.is_none(), "the series ends with a QC");
    s[last].kind = "sample".into();
    s[first].kind = "qc".into();
    let e = drift::correct(&t, &s, &b, Method::Linear).unwrap_err();
    assert!(
        e.key == "error.drift.qcNotAtEnds" || e.key == "error.drift.tooFewQc",
        "{}",
        e.key
    );
    // no QC at all, a span out of range, the wrong number of files
    let (t, mut s, b, _) = golden_series();
    assert!(drift::correct(&t, &s, &b, Method::Loess(0.0)).is_err());
    assert!(drift::correct(&t, &s[..3], &b, Method::Linear).is_err());
    for x in s.iter_mut() {
        x.kind = "sample".into();
    }
    assert_eq!(
        drift::correct(&t, &s, &b, Method::Linear).unwrap_err().key,
        "error.drift.tooFewQc"
    );
}

#[test]
fn a_known_drift_is_removed_from_the_qc() {
    // 3 batches of 12 files, QC at both ends and every third file; every row drifts along the injection order
    let mut files = Vec::new();
    for (bi, b) in ["A", "B", "C"].iter().enumerate() {
        for i in 0..12u32 {
            let kind = if i % 3 == 0 || i == 11 {
                "qc"
            } else {
                "sample"
            };
            files.push((kind.to_string(), bi as u32 * 12 + i + 1, b.to_string()));
        }
    }
    let areas: Vec<Vec<f64>> = (0..8)
        .map(|r| {
            files
                .iter()
                .map(|(_, order, _)| {
                    let wobble = 1.0 + 0.01 * (((*order as usize * 7 + r * 3) % 5) as f64 - 2.0);
                    1e6 * (1.0 + 0.02 * r as f64)
                        * (1.0 - 0.04 * f64::from((*order - 1) % 12)
                            + 0.15 * f64::from((*order - 1) / 12))
                        * wobble
                })
                .collect()
        })
        .collect();
    let (t, s, b) = build(&files, &areas);
    for method in [Method::Linear, Method::Loess(0.8)] {
        let d = drift::correct(&t, &s, &b, method).unwrap();
        for r in 0..t.len() {
            assert!(
                d.cv_after[r] <= d.cv_before[r],
                "row {r} {method:?}: {} after {} before",
                d.cv_after[r],
                d.cv_before[r]
            );
        }
        assert!(
            d.cv_before.iter().sum::<f64>() > 2.0 * d.cv_after.iter().sum::<f64>(),
            "the drift was big: {:?} {:?}",
            d.cv_before,
            d.cv_after
        );
        // the measured areas are untouched; the corrected area is area * factor
        let before = t.area.clone();
        let again = drift::correct(&t, &s, &b, method).unwrap();
        assert_eq!(before, t.area);
        assert_eq!(d, again, "second run");
        let (r, k) = (2, 5);
        assert_eq!(
            d.area(&t, r, k),
            Some(t.area[r * t.n_files + k] * d.factor[r * t.n_files + k])
        );
        // the summary and the curve are JSON
        let sum = parse(d.summary_json(&s).as_bytes()).unwrap();
        assert_eq!(sum.get("schema").unwrap().str(), Some("deriva/1"));
        assert_eq!(sum.get("batches").unwrap().arr().len(), 3);
        assert!(
            sum.get("median_cv_after").unwrap().f64().unwrap()
                < sum.get("median_cv_before").unwrap().f64().unwrap()
        );
        let curve = parse(d.curve_json(&t, &s, 1).as_bytes()).unwrap();
        assert_eq!(curve.get("points").unwrap().arr().len(), files.len());
    }
}

#[test]
fn the_batches_come_from_the_design() {
    let (_, s, _, _) = golden_series();
    let d = Disegno {
        conditions: vec![],
        reference: None,
        comparisons: vec![],
        factors: vec![],
        batches: vec![Lotto {
            file: s[0].file.clone(),
            sha256: s[0].sha256.clone(),
            batch: "9".into(),
        }],
    };
    let b = drift::batches_of(&s, &d);
    assert_eq!(b[0], "9");
    assert!(b[1..].iter().all(|x| x == "1"));
}
