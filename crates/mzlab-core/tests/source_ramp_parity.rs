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
        .map(|arr| {
            arr.iter()
                .map(|x| x.as_f64().unwrap_or(f64::NAN))
                .collect()
        })
        .unwrap_or_default()
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
    assert!(cases.len() >= 40, "expected at least 40 cases, got {}", cases.len());

    let tol = 1e-6;

    for (i, case) in cases.iter().enumerate() {
        let name = case["name"].as_str().unwrap_or("unnamed");
        let dp_in = nums(&case["input"]["dp"]);
        let f_in = nums(&case["input"]["f_int"]);
        let p_in = nums(&case["input"]["p_int"]);

        let res = source_ramp(&dp_in, &f_in, &p_in);
        let exp = &case["output"];

        // 1. Valid points count n
        let exp_n = exp["n"].as_u64().unwrap_or(0) as usize;
        assert_eq!(res.n, exp_n, "case {i} ({name}) n mismatch: got {}, exp {exp_n}", res.n);

        // 2. Sorted DP array
        let exp_dp = nums(&exp["dp"]);
        assert_eq!(res.dp.len(), exp_dp.len(), "case {i} ({name}) dp length mismatch");
        for (k, (&g, &w)) in res.dp.iter().zip(exp_dp.iter()).enumerate() {
            assert!(close(g, w, tol), "case {i} ({name}) dp[{k}] mismatch: got {g}, exp {w}");
        }

        // 3. Ratio array
        let exp_ratio = nums(&exp["ratio"]);
        assert_eq!(res.ratio.len(), exp_ratio.len(), "case {i} ({name}) ratio length mismatch");
        for (k, (&g, &w)) in res.ratio.iter().zip(exp_ratio.iter()).enumerate() {
            assert!(close(g, w, tol), "case {i} ({name}) ratio[{k}] mismatch: got {g}, exp {w}");
        }

        // 4. Warnings
        let exp_warnings: Vec<String> = exp["warnings"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|w| w.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let res_warnings: Vec<String> = res.warnings.iter().map(|&s| s.to_string()).collect();
        assert_eq!(res_warnings, exp_warnings, "case {i} ({name}) warnings mismatch: got {:?}, exp {:?}", res_warnings, exp_warnings);

        // 5. Spearman correlation
        match exp["spearman"].as_f64() {
            Some(w_sp) => {
                let g_sp = res.spearman.expect("expected Some spearman");
                assert!(close(g_sp, w_sp, tol), "case {i} ({name}) spearman: got {g_sp}, exp {w_sp}");
            }
            None => {
                assert!(res.spearman.is_none(), "case {i} ({name}) expected None spearman, got {:?}", res.spearman);
            }
        }

        // 6. Theil-Sen slope
        match exp["slope_per_v"].as_f64() {
            Some(w_sl) => {
                let g_sl = res.slope_per_v.expect("expected Some slope_per_v");
                assert!(close(g_sl, w_sl, tol), "case {i} ({name}) slope_per_v: got {g_sl}, exp {w_sl}");
            }
            None => {
                assert!(res.slope_per_v.is_none(), "case {i} ({name}) expected None slope_per_v, got {:?}", res.slope_per_v);
            }
        }

        // 7. DP dependence
        match exp["dp_dependence"].as_f64() {
            Some(w_dep) => {
                let g_dep = res.dp_dependence.expect("expected Some dp_dependence");
                assert!(close(g_dep, w_dep, tol), "case {i} ({name}) dp_dependence: got {g_dep}, exp {w_dep}");
            }
            None => {
                assert!(res.dp_dependence.is_none(), "case {i} ({name}) expected None dp_dependence, got {:?}", res.dp_dependence);
            }
        }

        // 8. Sigmoid fit
        let exp_sig = &exp["sigmoid"];
        if exp_sig.is_null() {
            assert!(res.sigmoid.is_none(), "case {i} ({name}) expected None sigmoid, got {:?}", res.sigmoid);
        } else {
            let sig = res.sigmoid.as_ref().expect("expected Some sigmoid");
            let w_ymax = exp_sig["ymax"].as_f64().expect("exp ymax");
            let w_dp50 = exp_sig["dp50"].as_f64().expect("exp dp50");
            let w_s = exp_sig["s"].as_f64().expect("exp s");
            let w_app = exp_sig["dp_appearance"].as_f64().expect("exp dp_appearance");
            let w_r2 = exp_sig["r2"].as_f64().unwrap_or(f64::NAN);

            assert!(close(sig.ymax, w_ymax, tol), "case {i} ({name}) ymax: got {}, exp {w_ymax}", sig.ymax);
            assert!(close(sig.dp50, w_dp50, tol), "case {i} ({name}) dp50: got {}, exp {w_dp50}", sig.dp50);
            assert!(close(sig.s, w_s, tol), "case {i} ({name}) s: got {}, exp {w_s}", sig.s);
            assert!(close(sig.dp_appearance, w_app, tol), "case {i} ({name}) dp_appearance: got {}, exp {w_app}", sig.dp_appearance);
            assert!(close(sig.r2, w_r2, tol), "case {i} ({name}) r2: got {}, exp {w_r2}", sig.r2);
        }
    }
}

#[test]
fn negative_and_mutational_tests() {
    // 1. Completely empty inputs
    let r_empty = source_ramp(&[], &[], &[]);
    assert_eq!(r_empty.n, 0);
    assert!(r_empty.dp.is_empty());
    assert!(r_empty.ratio.is_empty());
    assert_eq!(r_empty.warnings, vec!["of.warn.dp3"]);
    assert!(r_empty.spearman.is_none());
    assert!(r_empty.slope_per_v.is_none());
    assert!(r_empty.dp_dependence.is_none());
    assert!(r_empty.sigmoid.is_none());

    // 2. Mismatched slice lengths (never panics, trims to min)
    let r_mismatch = source_ramp(&[10.0, 20.0, 30.0, 40.0, 50.0], &[1.0, 2.0], &[9.0, 8.0, 7.0]);
    assert_eq!(r_mismatch.n, 2);
    assert_eq!(r_mismatch.dp.len(), 2);
    assert_eq!(r_mismatch.ratio.len(), 2);
    assert_eq!(r_mismatch.warnings, vec!["of.warn.dp3"]);

    // 3. Degenerate span (all DP identical): span == 0, never panics and returns None for sigmoid
    let r_const_dp = source_ramp(&[30.0, 30.0, 30.0, 30.0], &[10.0, 20.0, 30.0, 40.0], &[40.0, 30.0, 20.0, 10.0]);
    assert_eq!(r_const_dp.n, 4);
    assert!(r_const_dp.sigmoid.is_none());

    // 4. Negative / zero denominators: all zero intensities yield NaN ratio
    let r_all_zeros = source_ramp(&[10.0, 20.0, 30.0, 40.0], &[0.0, 0.0, 0.0, 0.0], &[0.0, 0.0, 0.0, 0.0]);
    assert_eq!(r_all_zeros.n, 0);
    assert_eq!(r_all_zeros.warnings, vec!["of.warn.dp3"]);
    for &val in &r_all_zeros.ratio {
        assert!(val.is_nan());
    }

    // 5. Negative intensities yielding negative/zero denominator
    let r_neg = source_ramp(&[10.0, 20.0, 30.0], &[-5.0, 10.0, 20.0], &[5.0, -10.0, -30.0]);
    assert_eq!(r_neg.n, 0);
    assert_eq!(r_neg.warnings, vec!["of.warn.dp3"]);

    // 6. Systematic mutation fuzzing on a valid sigmoid baseline:
    let base_dp = [10.0, 20.0, 30.0, 40.0, 50.0, 60.0];
    let base_f = [10.0, 30.0, 80.0, 180.0, 300.0, 450.0];
    let base_p = [490.0, 470.0, 420.0, 320.0, 200.0, 50.0];

    let baseline = source_ramp(&base_dp, &base_f, &base_p);
    assert_eq!(baseline.n, 6);
    assert!(baseline.sigmoid.is_some());

    let corruptions = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0.0, -100.0, 1e30, -1e30];
    for &mut_val in &corruptions {
        for idx in 0..6 {
            // mutate dp
            let mut dp_mut = base_dp;
            dp_mut[idx] = mut_val;
            let r_dp = source_ramp(&dp_mut, &base_f, &base_p);
            assert!(r_dp.n <= 6);

            // mutate f_int
            let mut f_mut = base_f;
            f_mut[idx] = mut_val;
            let r_f = source_ramp(&base_dp, &f_mut, &base_p);
            assert!(r_f.n <= 6);

            // mutate p_int
            let mut p_mut = base_p;
            p_mut[idx] = mut_val;
            let r_p = source_ramp(&base_dp, &base_f, &p_mut);
            assert!(r_p.n <= 6);
        }
    }
}
