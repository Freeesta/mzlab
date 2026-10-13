//! The MRM integration of the low-resolution Dig: `find_peak` against the Python engine (`TP_Mine/py/tpmine/peaks.py`, values computed on this very
//! trace) and the tables of `analyse` on three synthetic files.

use mzlab_core::json::{self, Value};
use mzlab_feat::legacy::lr::mrm::{analyse, MrmFile, Trace};
use mzlab_feat::legacy::lr::peak1d::find_peak;

const N: usize = 300;

fn rt() -> Vec<f64> {
    (0..N)
        .map(|i| 4.0 + 3.0 * i as f64 / (N - 1) as f64)
        .collect()
}

fn trace(c: f64, h: f64) -> Vec<f64> {
    rt().iter()
        .enumerate()
        .map(|(i, t)| {
            h * (-((t - c) / 0.08).powi(2) / 2.0).exp() + 20.0 + 5.0 * (37.0 * i as f64).sin()
        })
        .collect()
}

fn near(a: f64, b: f64, what: &str) {
    assert!(
        (a - b).abs() <= 1e-4 * b.abs().max(1.0),
        "{what}: {a} against {b}"
    );
}

#[test]
fn find_peak_as_python() {
    let t = rt();
    for (c, h, apex, height, area, fwhm, snr, pts, l, r) in [
        (
            5.0,
            1000.0,
            5.00334,
            988.99028,
            11641.70227,
            10.83612,
            257.51634,
            39,
            4.76254,
            5.24415,
        ),
        (
            5.1, 300.0, 5.09365, 294.34832, 3443.76104, 10.83612, 76.99892, 38, 4.86288, 5.34448,
        ),
    ] {
        for center in [None, Some(5.0)] {
            let p = find_peak(&t, &trace(c, h), center, 0.25, 5, 5).unwrap();
            near(p.apex_rt, apex, "apex");
            near(p.height, height, "height");
            near(p.area, area, "area");
            near(p.fwhm, fwhm, "fwhm");
            near(p.snr, snr, "snr");
            assert_eq!(p.points, pts);
            near(p.left, l, "left");
            near(p.right, r, "right");
            assert!(p.ok);
        }
    }
    assert!(find_peak(&t[..2], &[1.0, 2.0], None, 0.25, 5, 5).is_none());
    assert!(find_peak(&t, &vec![0.0; N], None, 0.25, 5, 5).is_none());
}

fn file(name: &str, kind: &str, time: Option<f64>, hs: [f64; 2]) -> MrmFile {
    let tr = |q3: f64, name: &str, h: f64| Trace {
        q1: 300.0,
        q3,
        ce: Some(25.0),
        name: name.into(),
        rt: rt(),
        y: trace(5.0, h),
    };
    MrmFile {
        name: name.into(),
        label: name.into(),
        kind_type: kind.into(),
        time,
        traces: vec![
            tr(200.0, "qualifier", hs[1]),
            tr(150.0, "quantifier", hs[0]),
        ],
    }
}

#[test]
fn tables_of_three_files() {
    let files = [
        file("s1", "standard", None, [1000.0, 500.0]),
        file("s2", "standard", None, [2000.0, 1000.0]),
        file("x", "sample", Some(10.0), [800.0, 1200.0]),
    ];
    let v = json::parse(analyse(&files, 0.25).unwrap().as_bytes()).unwrap();
    assert_eq!(v.get("quantifier").and_then(Value::str), Some("300>150"));
    assert!(v
        .get("calibration")
        .is_some_and(|c| matches!(c, Value::Null)));
    let ratio = v
        .get("ratio_ref")
        .and_then(|r| r.get("300>200"))
        .and_then(Value::f64)
        .unwrap();
    assert!((ratio - 0.5).abs() < 0.05, "{ratio}");
    let rows = v.get("rows").map(Value::arr).unwrap();
    let ok = |k: usize| rows[k].get("ratio_ok").and_then(Value::bool);
    assert_eq!((ok(0), ok(1), ok(2)), (Some(true), Some(true), Some(false)));
    assert!(rows[2]
        .get("ion")
        .and_then(|i| i.get("300>150"))
        .and_then(|i| i.get("detected"))
        .and_then(Value::bool)
        .unwrap());
}
