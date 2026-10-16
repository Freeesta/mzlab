//! Parity tests for `mzlab_core::abbina` against Python reference (`tools/abbina_riferimento.py`).
//!
//! Verifies 100% agreement on synthetic test cases (empty lists, boundary values, exact ties,
//! greedy contention, and seeded pseudo-random distributions) and includes mutational tests
//! that fail if tie-breaking criteria are inverted.

#![allow(clippy::disallowed_methods)]

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::abbina::abbina;
use serde_json::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn to_f64_vec(val: &Value) -> Vec<f64> {
    val.as_array()
        .map(|arr| arr.iter().filter_map(Value::as_f64).collect())
        .unwrap_or_default()
}

fn to_usize_vec(val: &Value) -> Vec<usize> {
    val.as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_u64)
                .map(|v| v as usize)
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn parity_against_python_reference() {
    let out = std::env::temp_dir().join(format!("mzlab_abbina_{}.json", std::process::id()));
    let script = root().join("tools/abbina_riferimento.py");
    let run = Command::new("python3").arg(&script).arg(&out).output();

    let ok = matches!(run, Ok(ref o) if o.status.success());

    if !ok {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 tools/abbina_riferimento.py failed to run"
        );
        eprintln!("python3 or numpy not available: skipping python reference check");
        return;
    }

    let bytes = std::fs::read(&out).expect("failed to read generated reference json");
    let _ = std::fs::remove_file(&out);

    let doc: Value = serde_json::from_slice(&bytes).expect("valid json reference");
    let cases = doc["cases"].as_array().expect("cases array");
    assert!(
        cases.len() >= 50,
        "expected at least 50 test cases, found {}",
        cases.len()
    );

    for (idx, c) in cases.iter().enumerate() {
        let name = c["name"].as_str().unwrap_or("unknown");
        let mz_a = to_f64_vec(&c["mz_a"]);
        let rt_a = to_f64_vec(&c["rt_a"]);
        let mz_b = to_f64_vec(&c["mz_b"]);
        let rt_b = to_f64_vec(&c["rt_b"]);
        let ppm = c["ppm"].as_f64().unwrap_or(5.0);
        let rt_tol = c["rt_tol"].as_f64().unwrap_or(0.15);
        let want_a = to_usize_vec(&c["out_a"]);
        let want_b = to_usize_vec(&c["out_b"]);

        let (got_a, got_b) = abbina(&mz_a, &rt_a, &mz_b, &rt_b, ppm, rt_tol);

        assert_eq!(
            got_a, want_a,
            "case #{idx} '{name}': out_a mismatch (got {:?}, want {:?})",
            got_a, want_a
        );
        assert_eq!(
            got_b, want_b,
            "case #{idx} '{name}': out_b mismatch (got {:?}, want {:?})",
            got_b, want_b
        );
    }
}

/// A mutated implementation where the secondary tie-break criterion (ia) is inverted (descending).
fn abbina_mutated_ia_descending(
    mz_a: &[f64],
    rt_a: &[f64],
    mz_b: &[f64],
    rt_b: &[f64],
    ppm: f64,
    rt_tol: f64,
) -> (Vec<usize>, Vec<usize>) {
    let na = mz_a.len().min(rt_a.len());
    let nb = mz_b.len().min(rt_b.len());
    if na == 0 || nb == 0 || ppm <= 0.0 || rt_tol <= 0.0 {
        return (Vec::new(), Vec::new());
    }

    let mut candidates: Vec<(f64, usize, usize)> = Vec::new();
    for ia in 0..na {
        let (mza, rta) = (mz_a[ia], rt_a[ia]);
        let lo = mza * (1.0 - ppm * 1e-6);
        let hi = mza * (1.0 + ppm * 1e-6);
        for ib in 0..nb {
            let (mzb, rtb) = (mz_b[ib], rt_b[ib]);
            if mzb >= lo && mzb <= hi && (rtb - rta).abs() <= rt_tol {
                let d = ((mzb - mza) / mza * 1e6 / ppm).hypot((rtb - rta) / rt_tol);
                candidates.push((d, ia, ib));
            }
        }
    }

    // Mutated sort: ia is compared in REVERSE (descending)
    candidates.sort_by(|a, b| {
        a.0.total_cmp(&b.0)
            .then_with(|| b.1.cmp(&a.1))
            .then_with(|| a.2.cmp(&b.2))
    });

    let mut used_a = vec![false; na];
    let mut used_b = vec![false; nb];
    let mut accepted = Vec::new();
    for (_d, ia, ib) in candidates {
        if !used_a[ia] && !used_b[ib] {
            used_a[ia] = true;
            used_b[ib] = true;
            accepted.push((ia, ib));
        }
    }
    accepted.sort_by_key(|a| a.0);
    (
        accepted.iter().map(|p| p.0).collect(),
        accepted.iter().map(|p| p.1).collect(),
    )
}

/// A mutated implementation where the tertiary tie-break criterion (ib) is inverted (descending).
fn abbina_mutated_ib_descending(
    mz_a: &[f64],
    rt_a: &[f64],
    mz_b: &[f64],
    rt_b: &[f64],
    ppm: f64,
    rt_tol: f64,
) -> (Vec<usize>, Vec<usize>) {
    let na = mz_a.len().min(rt_a.len());
    let nb = mz_b.len().min(rt_b.len());
    if na == 0 || nb == 0 || ppm <= 0.0 || rt_tol <= 0.0 {
        return (Vec::new(), Vec::new());
    }

    let mut candidates: Vec<(f64, usize, usize)> = Vec::new();
    for ia in 0..na {
        let (mza, rta) = (mz_a[ia], rt_a[ia]);
        let lo = mza * (1.0 - ppm * 1e-6);
        let hi = mza * (1.0 + ppm * 1e-6);
        for ib in 0..nb {
            let (mzb, rtb) = (mz_b[ib], rt_b[ib]);
            if mzb >= lo && mzb <= hi && (rtb - rta).abs() <= rt_tol {
                let d = ((mzb - mza) / mza * 1e6 / ppm).hypot((rtb - rta) / rt_tol);
                candidates.push((d, ia, ib));
            }
        }
    }

    // Mutated sort: ib is compared in REVERSE (descending)
    candidates.sort_by(|a, b| {
        a.0.total_cmp(&b.0)
            .then_with(|| a.1.cmp(&b.1))
            .then_with(|| b.2.cmp(&a.2))
    });

    let mut used_a = vec![false; na];
    let mut used_b = vec![false; nb];
    let mut accepted = Vec::new();
    for (_d, ia, ib) in candidates {
        if !used_a[ia] && !used_b[ib] {
            used_a[ia] = true;
            used_b[ib] = true;
            accepted.push((ia, ib));
        }
    }
    accepted.sort_by_key(|a| a.0);
    (
        accepted.iter().map(|p| p.0).collect(),
        accepted.iter().map(|p| p.1).collect(),
    )
}

#[test]
fn mutational_test_inverted_ia_tie_break_fails() {
    // Two identical points in A competing for one point in B:
    // Both have identical distance (d = 0.0).
    // Correct tie-break: ia=0 wins.
    // Inverted ia tie-break: ia=1 wins.
    let mz_a = [300.0, 300.0];
    let rt_a = [2.0, 2.0];
    let mz_b = [300.0];
    let rt_b = [2.0];

    let correct = abbina(&mz_a, &rt_a, &mz_b, &rt_b, 5.0, 0.15);
    let mutated = abbina_mutated_ia_descending(&mz_a, &rt_a, &mz_b, &rt_b, 5.0, 0.15);

    assert_eq!(
        correct,
        (vec![0], vec![0]),
        "correct tie-break selects ia=0"
    );
    assert_eq!(
        mutated,
        (vec![1], vec![0]),
        "mutated tie-break selects ia=1"
    );
    assert_ne!(
        correct, mutated,
        "mutated ia order must differ from correct specification"
    );
}

#[test]
fn mutational_test_inverted_ib_tie_break_fails() {
    // One point in A competing for two identical points in B:
    // Both have identical distance (d = 0.0) and identical ia=0.
    // Correct tie-break: ib=0 wins.
    // Inverted ib tie-break: ib=1 wins.
    let mz_a = [300.0];
    let rt_a = [2.0];
    let mz_b = [300.0, 300.0];
    let rt_b = [2.0, 2.0];

    let correct = abbina(&mz_a, &rt_a, &mz_b, &rt_b, 5.0, 0.15);
    let mutated = abbina_mutated_ib_descending(&mz_a, &rt_a, &mz_b, &rt_b, 5.0, 0.15);

    assert_eq!(
        correct,
        (vec![0], vec![0]),
        "correct tie-break selects ib=0"
    );
    assert_eq!(
        mutated,
        (vec![0], vec![1]),
        "mutated tie-break selects ib=1"
    );
    assert_ne!(
        correct, mutated,
        "mutated ib order must differ from correct specification"
    );
}

#[test]
fn edge_cases_direct() {
    // Empty inputs
    assert_eq!(abbina(&[], &[], &[], &[], 5.0, 0.15), (vec![], vec![]));
    assert_eq!(
        abbina(&[100.0], &[1.0], &[], &[], 5.0, 0.15),
        (vec![], vec![])
    );
    assert_eq!(
        abbina(&[], &[], &[100.0], &[1.0], 5.0, 0.15),
        (vec![], vec![])
    );

    // Invalid parameters
    assert_eq!(
        abbina(&[100.0], &[1.0], &[100.0], &[1.0], 0.0, 0.15),
        (vec![], vec![])
    );
    assert_eq!(
        abbina(&[100.0], &[1.0], &[100.0], &[1.0], 5.0, 0.0),
        (vec![], vec![])
    );
    assert_eq!(
        abbina(&[100.0], &[1.0], &[100.0], &[1.0], -5.0, 0.15),
        (vec![], vec![])
    );
    assert_eq!(
        abbina(&[100.0], &[1.0], &[100.0], &[1.0], f64::NAN, 0.15),
        (vec![], vec![])
    );

    // Greedy contention test
    // A0 (100.0, 5.00), A1 (100.0, 5.08)
    // B0 (100.0, 5.06), B1 (100.0, 5.12)
    // d(A1, B0) = |5.06 - 5.08|/0.15 = 0.02/0.15 = 0.1333
    // d(A0, B0) = |5.06 - 5.00|/0.15 = 0.06/0.15 = 0.4000
    // d(A1, B1) = |5.12 - 5.08|/0.15 = 0.04/0.15 = 0.2667
    // Smallest distance is (A1, B0). A1 gets B0!
    // A0 cannot get B0, but B1 is at distance |5.12 - 5.00|/0.15 = 0.12/0.15 = 0.80 <= 1.0.
    // So A0 gets B1!
    // Final sorted by ia: A0->B1, A1->B0.
    let (out_a, out_b) = abbina(
        &[100.0, 100.0],
        &[5.0, 5.08],
        &[100.0, 100.0],
        &[5.06, 5.12],
        5.0,
        0.15,
    );
    assert_eq!(out_a, vec![0, 1]);
    assert_eq!(out_b, vec![1, 0]);
}
