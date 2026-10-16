//! Parity test for `source_ramp` against Python reference (`mzlab/ionfamily.py:source_ramp`).

#![allow(clippy::disallowed_methods)]

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::source_ramp::*;
use serde_json::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a.is_nan() && b.is_nan()) || a == b || (a - b).abs() <= tol * a.abs().max(b.abs()).max(1.0)
}

fn nums(v: &Value) -> Vec<f64> {
    v.as_array()
        .map(|arr| arr.iter().map(|x| x.as_f64().unwrap_or(f64::NAN)).collect())
        .unwrap_or_default()
}

fn verify_arrays(tag: &str, res: &SourceRampResult, exp: &Value, tol: f64) {
    let exp_n = exp["n"].as_u64().unwrap_or(0) as usize;
    assert_eq!(res.n, exp_n, "{tag} n mismatch");

    let exp_dp = nums(&exp["dp"]);
    assert_eq!(res.dp.len(), exp_dp.len(), "{tag} dp len mismatch");
    for (k, (&g, &w)) in res.dp.iter().zip(&exp_dp).enumerate() {
        assert!(close(g, w, tol), "{tag} dp[{k}] mismatch: got {g}, exp {w}");
    }

    let exp_ratio = nums(&exp["ratio"]);
    assert_eq!(res.ratio.len(), exp_ratio.len(), "{tag} ratio len mismatch");
    for (k, (&g, &w)) in res.ratio.iter().zip(&exp_ratio).enumerate() {
        assert!(
            close(g, w, tol),
            "{tag} ratio[{k}] mismatch: got {g}, exp {w}"
        );
    }

    let exp_warns: Vec<String> = exp["warnings"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|w| w.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let res_warns: Vec<String> = res.warnings.iter().map(|&s| s.to_string()).collect();
    assert_eq!(res_warns, exp_warns, "{tag} warnings mismatch");
}

fn verify_metrics(tag: &str, res: &SourceRampResult, exp: &Value, tol: f64) {
    match exp["spearman"].as_f64() {
        Some(w_sp) => assert!(
            close(res.spearman.unwrap(), w_sp, tol),
            "{tag} spearman mismatch"
        ),
        None => assert!(res.spearman.is_none(), "{tag} expected None spearman"),
    }
    match exp["slope_per_v"].as_f64() {
        Some(w_sl) => assert!(
            close(res.slope_per_v.unwrap(), w_sl, tol),
            "{tag} slope mismatch"
        ),
        None => assert!(res.slope_per_v.is_none(), "{tag} expected None slope"),
    }
    match exp["dp_dependence"].as_f64() {
        Some(w_dep) => assert!(
            close(res.dp_dependence.unwrap(), w_dep, tol),
            "{tag} dp_dependence mismatch"
        ),
        None => assert!(
            res.dp_dependence.is_none(),
            "{tag} expected None dp_dependence"
        ),
    }
}

fn verify_sigmoid(tag: &str, res: &SourceRampResult, exp_sig: &Value, tol: f64) {
    if exp_sig.is_null() {
        assert!(res.sigmoid.is_none(), "{tag} expected None sigmoid");
    } else {
        let sig = res.sigmoid.as_ref().expect("expected Some sigmoid");
        assert!(
            close(sig.ymax, exp_sig["ymax"].as_f64().unwrap(), tol),
            "{tag} ymax"
        );
        assert!(
            close(sig.dp50, exp_sig["dp50"].as_f64().unwrap(), tol),
            "{tag} dp50"
        );
        assert!(close(sig.s, exp_sig["s"].as_f64().unwrap(), tol), "{tag} s");
        assert!(
            close(
                sig.dp_appearance,
                exp_sig["dp_appearance"].as_f64().unwrap(),
                tol
            ),
            "{tag} app"
        );
        assert!(
            close(sig.r2, exp_sig["r2"].as_f64().unwrap_or(f64::NAN), tol),
            "{tag} r2"
        );
    }
}

fn verify_ramp_case(case: &Value, tol: f64) {
    let name = case["name"].as_str().unwrap_or("unnamed");
    let dp_in = nums(&case["input"]["dp"]);
    let f_in = nums(&case["input"]["f_int"]);
    let p_in = nums(&case["input"]["p_int"]);
    let res = source_ramp(&dp_in, &f_in, &p_in);
    let exp = &case["output"];

    verify_arrays(name, &res, exp, tol);
    verify_metrics(name, &res, exp, tol);
    verify_sigmoid(name, &res, &exp["sigmoid"], tol);
}

#[test]
fn source_ramp_python_parity() {
    let out = std::env::temp_dir().join(format!("mzlab_source_ramp_{}.json", std::process::id()));
    let status = Command::new("python3")
        .arg(root().join("tools/source_ramp_riferimento.py"))
        .arg(&out)
        .status();

    let ok = status.map(|s| s.success()).unwrap_or(false);
    assert!(ok, "failed to run tools/source_ramp_riferimento.py");

    let content = std::fs::read_to_string(&out).expect("failed to read reference JSON");
    let _ = std::fs::remove_file(&out);

    let data: Value = serde_json::from_str(&content).expect("invalid JSON from reference");
    let cases = data["cases"].as_array().expect("cases must be array");
    assert!(
        cases.len() >= 40,
        "expected at least 40 cases, got {}",
        cases.len()
    );

    for case in cases {
        verify_ramp_case(case, 1e-6);
    }
}

#[test]
fn negative_and_boundary_conditions() {
    let r_empty = source_ramp(&[], &[], &[]);
    assert_eq!(r_empty.n, 0);
    assert_eq!(r_empty.warnings, vec!["of.warn.dp3"]);

    let r_mismatch = source_ramp(&[10.0, 20.0, 30.0], &[1.0, 2.0], &[9.0, 8.0, 7.0]);
    assert_eq!(r_mismatch.n, 2);
    assert_eq!(r_mismatch.warnings, vec!["of.warn.dp3"]);

    let r_const_dp = source_ramp(
        &[30.0; 4],
        &[10.0, 20.0, 30.0, 40.0],
        &[40.0, 30.0, 20.0, 10.0],
    );
    assert_eq!(r_const_dp.n, 4);
    assert!(r_const_dp.sigmoid.is_none());

    let r_zeros = source_ramp(&[10.0, 20.0], &[0.0, 0.0], &[0.0, 0.0]);
    assert_eq!(r_zeros.n, 0);
    assert_eq!(r_zeros.warnings, vec!["of.warn.dp3"]);
}

#[test]
fn mutational_fuzzing_test() {
    let base_dp = [10.0, 20.0, 30.0, 40.0, 50.0, 60.0];
    let base_f = [10.0, 30.0, 80.0, 180.0, 300.0, 450.0];
    let base_p = [490.0, 470.0, 420.0, 320.0, 200.0, 50.0];
    let baseline = source_ramp(&base_dp, &base_f, &base_p);
    assert_eq!(baseline.n, 6);
    assert!(baseline.sigmoid.is_some());

    let corruptions = [f64::NAN, f64::INFINITY, 0.0, -100.0, 1e30];
    for &mut_val in &corruptions {
        for idx in 0..6 {
            let mut dp_mut = base_dp;
            dp_mut[idx] = mut_val;
            assert!(source_ramp(&dp_mut, &base_f, &base_p).n <= 6);

            let mut f_mut = base_f;
            f_mut[idx] = mut_val;
            assert!(source_ramp(&base_dp, &f_mut, &base_p).n <= 6);

            let mut p_mut = base_p;
            p_mut[idx] = mut_val;
            assert!(source_ramp(&base_dp, &base_f, &p_mut).n <= 6);
        }
    }
}
