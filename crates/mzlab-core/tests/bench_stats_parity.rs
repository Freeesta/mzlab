//! Parity tests for `bench_stats`: McNemar exact against Python `math.comb`,
//! Holm step-down against Python, non-inferiority, superiority and bootstrap.

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::bench_stats::*;
use serde_json::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a.is_nan() && b.is_nan()) || a == b || (a - b).abs() <= tol
}

fn load_reference_data() -> &'static Value {
    static DATA: std::sync::OnceLock<Value> = std::sync::OnceLock::new();
    DATA.get_or_init(|| {
        let script = root().join("tools/bench_stats_riferimento.py");
        let output = Command::new("python3")
            .arg(&script)
            .arg("-")
            .output()
            .expect("Failed to execute python3 tools/bench_stats_riferimento.py");
        assert!(
            output.status.success(),
            "Reference generation script failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).expect("Failed to parse reference json")
    })
}

#[test]
fn parity_mcnemar_exact_grid_and_vectors() {
    let data = load_reference_data();
    let cases = data["mcnemar_cases"]
        .as_array()
        .expect("mcnemar_cases array");
    assert!(
        cases.len() >= 500,
        "Expected >= 500 McNemar cases, got {}",
        cases.len()
    );

    for (i, c) in cases.iter().enumerate() {
        let n10 = c["n10"].as_u64().expect("n10") as usize;
        let n01 = c["n01"].as_u64().expect("n01") as usize;
        let want_p = c["p"].as_f64().expect("p");
        let got_p = mcnemar_p_value(n10, n01);
        assert!(
            close(got_p, want_p, 1e-12),
            "mcnemar case {i} (n10={n10}, n01={n01}): got {got_p}, want {want_p}"
        );
    }

    let vec_cases = data["mcnemar_vectors"].as_array().expect("mcnemar_vectors");
    for (i, c) in vec_cases.iter().enumerate() {
        let a: Vec<i32> = c["a"]
            .as_array()
            .expect("a")
            .iter()
            .map(|v| v.as_i64().expect("i") as i32)
            .collect();
        let b: Vec<i32> = c["b"]
            .as_array()
            .expect("b")
            .iter()
            .map(|v| v.as_i64().expect("i") as i32)
            .collect();
        let want_n10 = c["n10"].as_u64().expect("n10") as usize;
        let want_n01 = c["n01"].as_u64().expect("n01") as usize;
        let want_p = c["p"].as_f64().expect("p");

        let (got_n10, got_n01, got_p) = mcnemar_esatto(&a, &b);
        assert_eq!(got_n10, want_n10, "mcnemar vector case {i} n10");
        assert_eq!(got_n01, want_n01, "mcnemar vector case {i} n01");
        assert!(
            close(got_p, want_p, 1e-12),
            "mcnemar vector case {i} p: got {got_p}, want {want_p}"
        );
    }
}

#[test]
fn parity_holm_step_down() {
    let data = load_reference_data();
    let cases = data["holm_cases"].as_array().expect("holm_cases array");
    assert!(cases.len() >= 20, "Expected >= 20 holm cases");

    for (i, c) in cases.iter().enumerate() {
        let p: Vec<f64> = c["p"]
            .as_array()
            .expect("p")
            .iter()
            .map(|v| v.as_f64().expect("f"))
            .collect();
        let alfa = c["alfa"].as_f64().expect("alfa");
        let want_adj: Vec<f64> = c["adj"]
            .as_array()
            .expect("adj")
            .iter()
            .map(|v| v.as_f64().expect("f"))
            .collect();
        let want_rej: Vec<bool> = c["rifiutata"]
            .as_array()
            .expect("rej")
            .iter()
            .map(|v| v.as_bool().expect("b"))
            .collect();

        let (got_adj, got_rej) = holm(&p, alfa);
        assert_eq!(got_adj.len(), want_adj.len(), "case {i} length");
        for (j, (&got, &want)) in got_adj.iter().zip(&want_adj).enumerate() {
            assert!(
                close(got, want, 1e-12),
                "case {i} index {j}: got adj {got}, want {want}"
            );
        }
        assert_eq!(got_rej, want_rej, "case {i} rejection flags");
    }
}

#[test]
fn parity_decisions_non_inferiore_e_superiore() {
    let data = load_reference_data();
    let cases = data["decision_cases"]
        .as_array()
        .expect("decision_cases array");
    assert!(cases.len() >= 30, "Expected >= 30 decision cases");

    for (i, c) in cases.iter().enumerate() {
        let lo = c["ic_basso"].as_f64().expect("lo");
        let hi = c["ic_alto"].as_f64().expect("hi");
        let dir_alto = c["direzione_alto"].as_bool().expect("dir");
        let margine = c["margine"].as_f64().expect("margine");
        let rej = c["rifiutata"].as_bool().expect("rej");
        let want_ni = c["non_inferiore"].as_bool().expect("ni");
        let want_sup = c["superiore"].as_bool().expect("sup");

        let got_ni = non_inferiore(lo, hi, dir_alto, margine);
        let got_sup = superiore(lo, hi, dir_alto, rej);
        assert_eq!(got_ni, want_ni, "decision case {i} non_inferiore");
        assert_eq!(got_sup, want_sup, "decision case {i} superiore");
    }
}

#[test]
fn parity_bootstrap_confidence_interval() {
    let data = load_reference_data();
    let cases = data["bootstrap_cases"]
        .as_array()
        .expect("bootstrap_cases array");
    for (i, c) in cases.iter().enumerate() {
        let x: Vec<f64> = c["x"]
            .as_array()
            .expect("x")
            .iter()
            .map(|v| v.as_f64().unwrap_or(f64::NAN))
            .collect();
        let want_pt = c["valore"].as_f64().expect("pt");
        let want_lo = c["ic_basso"].as_f64().expect("lo");
        let want_hi = c["ic_alto"].as_f64().expect("hi");

        let tipo_str = c["tipo"].as_str().unwrap_or("media");
        let tipo = if tipo_str == "mediana" {
            StatTipo::Mediana
        } else {
            StatTipo::Media
        };

        let res = intervallo(&x, tipo, 200, 20261012);
        assert_eq!(res.n, x.len());
        assert!(
            close(res.valore.expect("val"), want_pt, 1e-9),
            "boot case {i} value"
        );
        assert!(
            close(res.ic_basso.expect("lo"), want_lo, 1e-9),
            "boot case {i} lo"
        );
        assert!(
            close(res.ic_alto.expect("hi"), want_hi, 1e-9),
            "boot case {i} hi"
        );
    }
}

#[test]
fn edge_cases_and_robustness() {
    // Empty inputs
    assert_eq!(mcnemar_esatto(&[], &[]), (0, 0, 1.0));
    assert_eq!(holm(&[], 0.05), (Vec::new(), Vec::new()));
    let empty_int = intervallo(&[], StatTipo::Media, 100, 1);
    assert_eq!(empty_int.n, 0);
    assert!(empty_int.valore.is_none());

    let empty_diff = differenza(&[], &[], StatTipo::Media, 100, 1);
    assert!(empty_diff.is_some());
    assert_eq!(empty_diff.as_ref().map(|d| d.n), Some(0));

    // Mismatched lengths
    assert!(differenza(&[1.0, 2.0], &[1.0], StatTipo::Media, 100, 1).is_none());

    // Single elements
    let (h_adj, h_rej) = holm(&[0.02], 0.05);
    assert_eq!(h_adj, vec![0.02]);
    assert_eq!(h_rej, vec![true]);

    let (h_adj2, h_rej2) = holm(&[0.06], 0.05);
    assert_eq!(h_adj2, vec![0.06]);
    assert_eq!(h_rej2, vec![false]);
}
