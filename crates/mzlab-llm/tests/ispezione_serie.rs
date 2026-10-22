//! `ispezione/1` for a series: the QC files give their drift, the blanks their own signal, and a small budget drops the curves first.

mod ispez;

use ispez::*;
use mzlab_core::json::Value;
use mzlab_llm::inspect::build;

const QC: [&str; 11] = [
    "Q_blank1.mzML",
    "Q_QC1.mzML",
    "Q_S1.mzML",
    "Q_S2.mzML",
    "Q_S3.mzML",
    "Q_QC2.mzML",
    "Q_S4.mzML",
    "Q_S5.mzML",
    "Q_S6.mzML",
    "Q_QC3.mzML",
    "Q_blank2.mzML",
];

#[test]
fn the_qc_series_gives_its_drift_and_its_blank_signal() {
    let Some(dir) = synthetic() else { return };
    let fwhm = |_: &str| ",\"estimate\":{\"fwhm_min\":0.14,\"fwhm_n\":20}".to_string();
    let v = j(&build(&sources(dir, &QC, &fwhm, true), 40000).unwrap());
    let series = v.get("series").unwrap();
    let qc = series.get("qc").expect("three QC files give qc/1");
    assert_eq!(text(qc, "schema"), "qc/1");
    assert_eq!(list(qc, "qc"), ["f2", "f6", "f10"]);
    let order = qc.get("order").unwrap().arr();
    assert_eq!(order.iter().filter_map(Value::f64).sum::<f64>(), 18.0);
    assert_eq!(text(qc, "order_from"), "sample_table");
    assert_eq!(text(qc, "confidence"), "high");
    assert!(qc.get("common_signals").unwrap().f64().unwrap() >= 3.0);
    let rt = qc.get("rt_drift_min").unwrap();
    let near = |x: f64, y: f64, tol: f64| assert!((x - y).abs() <= tol, "{x} against {y}");
    near(rt.get("first_last").unwrap().f64().unwrap(), 0.05, 0.01);
    near(
        rt.get("slope_per_inj").unwrap().f64().unwrap(),
        0.00625,
        0.0015,
    );
    assert!(rt.get("robust_sd").unwrap().f64().unwrap() < 0.02);
    near(
        qc.get("area_drift")
            .unwrap()
            .get("first_last_pct")
            .unwrap()
            .f64()
            .unwrap(),
        -6.1,
        3.0,
    );
    // R-rt-1: three robust deviations, never below the median peak width
    let prop = qc.get("proposal").unwrap();
    near(
        prop.get("features.align.rt_gap").unwrap().f64().unwrap(),
        0.14,
        0.001,
    );
    assert_eq!(
        prop.get("features.align.rt_warp").unwrap().bool(),
        Some(false)
    );
    // the contaminant sits in both blanks and in no other file
    let blanks = series.get("blanks").unwrap().arr();
    assert_eq!(blanks.len(), 2);
    for b in blanks {
        let top = &b.get("signals").unwrap().arr()[0];
        near(top.get("mz").unwrap().f64().unwrap(), 391.3, 0.1);
        near(top.get("rt").unwrap().f64().unwrap(), 3.42, 0.1);
        assert_eq!(top.get("in_samples").unwrap().bool(), Some(false));
    }
    let roles = series.get("roles").unwrap();
    assert_eq!(
        (
            roles.get("blank").unwrap().f64(),
            roles.get("qc").unwrap().f64()
        ),
        (Some(2.0), Some(3.0))
    );
    // with one QC there is no drift to measure
    let few = sources(dir, &QC[..3], &|_| String::new(), false);
    assert!(j(&build(&few, 40000).unwrap())
        .get("series")
        .unwrap()
        .get("qc")
        .is_none());
}

#[test]
fn a_small_budget_drops_the_curves_first_and_says_so() {
    let Some(dir) = synthetic() else { return };
    let src = sources(dir, &QC[..4], &|_| String::new(), true);
    let big = build(&src, 60000).unwrap();
    assert!(big.contains("\"tic\":{"));
    let small = build(&src, 3500).unwrap();
    assert!(
        !small.contains("\"tic\":{") && (small.len() as u64).div_ceil(3) <= 3500,
        "{} bytes",
        small.len()
    );
    assert!(list(&j(&small), "truncated").contains(&"tic".to_string()));
    assert_eq!(build(&src, 1).unwrap_err().key, "err.llm.context.budget");
}
