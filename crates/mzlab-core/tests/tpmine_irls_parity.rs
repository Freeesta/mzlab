//! Parity tests for regularised L2 logistic regression via IRLS against Python (`tools/tpmine_irls_riferimento.py`).
//! Checks coefficients, intercept, standardization parameters, logit predictions and feature contributions.

use std::path::Path;
use std::process::Command;

use mzlab_core::tpmine_irls::{fit_logistic, predict_logit, LogisticModel};
use serde_json::Value;

fn close(got: f64, want: f64) -> bool {
    if got.is_nan() && want.is_nan() {
        return true;
    }
    let diff = (got - want).abs();
    diff <= 1e-7 || diff <= 1e-7 * got.abs().max(want.abs())
}

fn parse_f64_row(row_val: &Value) -> Vec<f64> {
    match row_val.as_array() {
        Some(arr) => arr
            .iter()
            .map(|item| item.as_f64().unwrap_or(f64::NAN))
            .collect(),
        None => Vec::new(),
    }
}

fn verify_queries(case_idx: usize, queries: &[Value], model: &LogisticModel) {
    for (q_idx, q) in queries.iter().enumerate() {
        let feat = parse_f64_row(q.get("feat").expect("missing feat"));
        let want_logit = q
            .get("logit")
            .and_then(Value::as_f64)
            .expect("missing logit");
        let want_contrib = parse_f64_row(q.get("contrib").expect("missing contrib"));

        let (got_logit, got_contrib) = predict_logit(&feat, model);

        assert!(
            close(got_logit, want_logit),
            "case {case_idx} query {q_idx} logit: got {got_logit} want {want_logit}"
        );

        for (j, (&got_cb, &want_cb)) in got_contrib.iter().zip(&want_contrib).enumerate() {
            assert!(
                close(got_cb, want_cb),
                "case {case_idx} query {q_idx} contrib[{j}]: got {got_cb} want {want_cb}"
            );
        }
    }
}

fn verify_single_case(case_idx: usize, c: &Value) {
    let x_raw = c.get("x").and_then(Value::as_array).expect("missing x");
    let x: Vec<Vec<f64>> = x_raw.iter().map(parse_f64_row).collect();
    let y = parse_f64_row(c.get("y").expect("missing y"));

    let weights: Option<Vec<f64>> = c.get("w").and_then(|w| {
        if w.is_null() {
            None
        } else {
            Some(parse_f64_row(w))
        }
    });

    let lam = c.get("lam").and_then(Value::as_f64).expect("missing lam");
    let n_iter = c
        .get("n_iter")
        .and_then(Value::as_u64)
        .map(|v| v as usize)
        .unwrap_or(60);

    let want_mean = parse_f64_row(c.get("mean").expect("missing mean"));
    let want_std = parse_f64_row(c.get("std").expect("missing std"));
    let want_coef = parse_f64_row(c.get("coef").expect("missing coef"));
    let want_intercept = c
        .get("intercept")
        .and_then(Value::as_f64)
        .expect("missing intercept");

    let model = match fit_logistic(&x, &y, lam, weights.as_deref(), n_iter) {
        Ok(m) => m,
        Err(e) => panic!("case {case_idx} fit_logistic failed: {e}"),
    };

    assert!(
        close(model.intercept, want_intercept),
        "case {case_idx}: intercept got {} want {}",
        model.intercept,
        want_intercept
    );

    for (j, (&got_c, &want_c)) in model.coef.iter().zip(&want_coef).enumerate() {
        assert!(
            close(got_c, want_c),
            "case {case_idx} coef[{j}]: got {got_c} want {want_c}"
        );
    }

    for (j, (&got_m, &want_m)) in model.mean.iter().zip(&want_mean).enumerate() {
        assert!(
            close(got_m, want_m),
            "case {case_idx} mean[{j}]: got {got_m} want {want_m}"
        );
    }

    for (j, (&got_s, &want_s)) in model.std.iter().zip(&want_std).enumerate() {
        assert!(
            close(got_s, want_s),
            "case {case_idx} std[{j}]: got {got_s} want {want_s}"
        );
    }

    if let Some(queries) = c.get("queries").and_then(Value::as_array) {
        verify_queries(case_idx, queries, &model);
    }
}

#[test]
fn tpmine_irls_matches_python_reference() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = std::env::temp_dir().join(format!("mzlab-irls-{}.json", std::process::id()));

    let run = Command::new("python3")
        .current_dir(&root)
        .arg("tools/tpmine_irls_riferimento.py")
        .arg(&out)
        .output();

    let success = match &run {
        Ok(output) => output.status.success(),
        Err(_) => false,
    };

    if !success {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "the reference generator did not run: {run:?}"
        );
        eprintln!("python3 or numpy not available: test skipped");
        return;
    }

    let file_bytes = match std::fs::read(&out) {
        Ok(b) => b,
        Err(e) => panic!("failed to read generated reference file: {e}"),
    };
    let _ = std::fs::remove_file(&out);

    let data: Value = match serde_json::from_slice(&file_bytes) {
        Ok(v) => v,
        Err(e) => panic!("invalid JSON in reference output: {e}"),
    };

    let cases = match data.get("cases").and_then(Value::as_array) {
        Some(c) => c,
        None => panic!("missing 'cases' array in reference data"),
    };

    assert!(
        cases.len() >= 30,
        "expected at least 30 test cases, got {}",
        cases.len()
    );

    for (case_idx, c) in cases.iter().enumerate() {
        verify_single_case(case_idx, c);
    }
}

#[test]
fn validation_and_edge_cases() {
    // Empty inputs
    assert!(fit_logistic(&[], &[], 1.0, None, 60).is_err());
    assert!(fit_logistic(&[vec![]], &[1.0], 1.0, None, 60).is_err());

    // Inconsistent feature row length
    assert!(fit_logistic(&[vec![1.0, 2.0], vec![3.0]], &[1.0, 0.0], 1.0, None, 60).is_err());

    // Dimension mismatches
    assert!(fit_logistic(&[vec![1.0], vec![2.0]], &[1.0], 1.0, None, 60).is_err());
    assert!(fit_logistic(&[vec![1.0]], &[1.0], 1.0, Some(&[1.0, 2.0]), 60).is_err());

    // Predict with varying feature slice lengths
    let model = LogisticModel {
        mean: vec![0.0, 1.0],
        std: vec![1.0, 2.0],
        coef: vec![0.5, -0.5],
        intercept: 0.1,
    };

    // Exactly matching length
    let (logit, contrib) = predict_logit(&[0.0, 3.0], &model);
    assert_eq!(contrib.len(), 2);
    assert!((contrib[0] - 0.0).abs() < 1e-12);
    assert!((contrib[1] - (-0.5)).abs() < 1e-12);
    assert!((logit - (0.1 - 0.5)).abs() < 1e-12);

    // Shorter feature slice (missing features treated as NaN -> contrib 0.0)
    let (logit_short, contrib_short) = predict_logit(&[2.0], &model);
    assert_eq!(contrib_short.len(), 2);
    assert!((contrib_short[0] - 1.0).abs() < 1e-12);
    assert_eq!(contrib_short[1], 0.0);
    assert!((logit_short - (0.1 + 1.0)).abs() < 1e-12);

    // Longer feature slice (extra features ignored)
    let (logit_long, contrib_long) = predict_logit(&[0.0, 3.0, 99.0], &model);
    assert_eq!(contrib_long.len(), 2);
    assert!((logit_long - logit).abs() < 1e-12);
}
