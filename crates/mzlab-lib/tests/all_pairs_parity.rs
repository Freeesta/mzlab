//! Parity and property tests for `all_pairs` module against Python reference implementation.

use std::path::Path;
use std::process::Command;

use mzlab_lib::all_pairs::{bin_matrices, pairs_above, plain_cosine, upper_bound};
use serde_json::Value;

fn run_python_reference(out_path: &Path) {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = manifest_dir.join("../..");
    let script = root.join("tools/all_pairs_riferimento.py");
    let status = Command::new("python3")
        .current_dir(&root)
        .arg(&script)
        .arg(out_path)
        .status()
        .expect("Failed to execute python3 tools/all_pairs_riferimento.py");
    assert!(status.success(), "Python reference generator failed");
}

fn assert_close_f32(got: &[Vec<f32>], want: &[Vec<f32>], tol: f32, ctx: &str) {
    assert_eq!(got.len(), want.len(), "row count mismatch in {ctx}");
    for (r, (g_row, w_row)) in got.iter().zip(want.iter()).enumerate() {
        assert_eq!(g_row.len(), w_row.len(), "col count mismatch at row {r} in {ctx}");
        for (c, (&g, &w)) in g_row.iter().zip(w_row.iter()).enumerate() {
            assert!(
                (g - w).abs() <= tol,
                "mismatch at [{r}][{c}] in {ctx}: got {g}, want {w} (diff {})",
                (g - w).abs()
            );
        }
    }
}

fn assert_close_f64(got: &[f64], want: &[f64], tol: f64, ctx: &str) {
    assert_eq!(got.len(), want.len(), "length mismatch in {ctx}");
    for (i, (&g, &w)) in got.iter().zip(want.iter()).enumerate() {
        assert!(
            (g - w).abs() <= tol,
            "mismatch at index {i} in {ctx}: got {g}, want {w} (diff {})",
            (g - w).abs()
        );
    }
}

fn json_to_vec_vec_f64(val: &Value) -> Vec<Vec<f64>> {
    val.as_array()
        .map(|arr| {
            arr.iter()
                .map(|row| {
                    row.as_array()
                        .map(|r| r.iter().filter_map(|x| x.as_f64()).collect())
                        .unwrap_or_default()
                })
                .collect()
        })
        .unwrap_or_default()
}

fn json_to_vec_vec_f32(val: &Value) -> Vec<Vec<f32>> {
    val.as_array()
        .map(|arr| {
            arr.iter()
                .map(|row| {
                    row.as_array()
                        .map(|r| r.iter().filter_map(|x| x.as_f64().map(|v| v as f32)).collect())
                        .unwrap_or_default()
                })
                .collect()
        })
        .unwrap_or_default()
}

fn json_to_vec_f64(val: &Value) -> Vec<f64> {
    val.as_array()
        .map(|arr| arr.iter().filter_map(|x| x.as_f64()).collect())
        .unwrap_or_default()
}

fn json_to_vec_usize(val: &Value) -> Vec<usize> {
    val.as_array()
        .map(|arr| arr.iter().filter_map(|x| x.as_u64().map(|v| v as usize)).collect())
        .unwrap_or_default()
}

#[test]
fn test_all_pairs_parity_with_python() {
    let tmp_dir = std::env::temp_dir();
    let out_file = tmp_dir.join(format!("mzlab_all_pairs_parity_{}.json", std::process::id()));
    run_python_reference(&out_file);

    let content = std::fs::read_to_string(&out_file).expect("Failed to read reference JSON");
    let _ = std::fs::remove_file(&out_file);

    let cases: Vec<Value> = serde_json::from_str(&content).expect("Failed to parse reference JSON");
    assert!(cases.len() >= 40, "Expected at least 40 cases, got {}", cases.len());

    for case in &cases {
        let id = case["id"].as_i64().unwrap_or(-1);
        let desc = case["desc"].as_str().unwrap_or("unknown");
        let ctx = format!("case {id} ({desc})");

        let m = json_to_vec_vec_f64(&case["m"]);
        let w_f32 = json_to_vec_vec_f32(&case["w"]);
        let w_f64 = json_to_vec_vec_f64(&case["w"]);
        let p = json_to_vec_f64(&case["p"]);
        let binw = case["binw"].as_f64().unwrap_or(1.0);
        let mmax = case["mmax"].as_f64().unwrap_or(1000.0);
        let threshold = case["threshold"].as_f64().unwrap_or(0.3) as f32;
        let upper_triangle = case["upper_triangle"].as_bool().unwrap_or(true);
        let tol = case["tol"].as_f64().unwrap_or(0.005);

        let rows_opt = if case["rows"].is_null() {
            None
        } else {
            Some(json_to_vec_usize(&case["rows"]))
        };
        let cols_opt = if case["cols"].is_null() {
            None
        } else {
            Some(json_to_vec_usize(&case["cols"]))
        };

        let exp = &case["expected"];
        let exp_nbins = exp["n_bins"].as_u64().unwrap_or(0) as usize;
        let exp_f = json_to_vec_vec_f32(&exp["f"]);
        let exp_l = json_to_vec_vec_f32(&exp["l"]);
        let exp_fs = json_to_vec_vec_f32(&exp["fs"]);
        let exp_ls = json_to_vec_vec_f32(&exp["ls"]);
        let exp_u = json_to_vec_vec_f32(&exp["u"]);
        let exp_ia = json_to_vec_usize(&exp["pairs_ia"]);
        let exp_ib = json_to_vec_usize(&exp["pairs_ib"]);
        let exp_scores = json_to_vec_f64(&exp["cosine_scores"]);

        // 1. bin_matrices
        let bm = bin_matrices(&m, &w_f32, &p, binw, mmax);
        assert_eq!(bm.n_bins, exp_nbins, "n_bins mismatch in {ctx}");
        assert_close_f32(&bm.f, &exp_f, 1e-5, &format!("{ctx} - F"));
        assert_close_f32(&bm.l, &exp_l, 1e-5, &format!("{ctx} - L"));
        assert_close_f32(&bm.fs, &exp_fs, 1e-5, &format!("{ctx} - Fs"));
        assert_close_f32(&bm.ls, &exp_ls, 1e-5, &format!("{ctx} - Ls"));

        // 2. upper_bound
        let u = upper_bound(&bm, rows_opt.as_deref(), cols_opt.as_deref());
        assert_close_f32(&u, &exp_u, 1e-5, &format!("{ctx} - U"));

        // 3. pairs_above
        let (ia, ib) = pairs_above(&u, threshold, upper_triangle);
        assert_eq!(ia, exp_ia, "ia pairs mismatch in {ctx}");
        assert_eq!(ib, exp_ib, "ib pairs mismatch in {ctx}");

        // 4. plain_cosine
        let pc_ia = if case["pc_ia"].is_null() {
            ia.clone()
        } else {
            json_to_vec_usize(&case["pc_ia"])
        };
        let pc_ib = if case["pc_ib"].is_null() {
            ib.clone()
        } else {
            json_to_vec_usize(&case["pc_ib"])
        };
        let scores = plain_cosine(&m, &w_f64, &pc_ia, &pc_ib, tol);
        assert_close_f64(&scores, &exp_scores, 1e-7, &format!("{ctx} - plain_cosine"));
    }
}

#[test]
fn test_all_pairs_negative_and_edge_cases() {
    // 1. Degenerate binw and mmax
    let m = vec![vec![100.0, 200.0]];
    let w32 = vec![vec![0.5f32, 0.8f32]];
    let p = vec![300.0];
    let bm_neg_binw = bin_matrices(&m, &w32, &p, -1.0, 500.0);
    assert!(bm_neg_binw.n_bins >= 2);
    let bm_zero_binw = bin_matrices(&m, &w32, &p, 0.0, 500.0);
    assert!(bm_zero_binw.n_bins >= 2);
    let bm_nan_binw = bin_matrices(&m, &w32, &p, f64::NAN, 500.0);
    assert!(bm_nan_binw.n_bins >= 2);

    // 2. Out of bounds row/col in upper_bound
    let bm = bin_matrices(&m, &w32, &p, 1.0, 500.0);
    let u_oob = upper_bound(&bm, Some(&[999]), Some(&[888]));
    assert_eq!(u_oob.len(), 1);
    assert_eq!(u_oob[0].len(), 1);
    assert_eq!(u_oob[0][0], 0.0);

    // 3. Out of bounds index in plain_cosine
    let w64 = vec![vec![0.5, 0.8]];
    let scores_oob = plain_cosine(&m, &w64, &[999], &[0], 0.01);
    assert_eq!(scores_oob, vec![0.0]);

    // 4. Mismatched length of ia and ib
    let scores_mismatched = plain_cosine(&m, &w64, &[0, 0], &[0], 0.01);
    assert_eq!(scores_mismatched.len(), 1);

    // 5. Negative tolerance
    let scores_neg_tol = plain_cosine(&m, &w64, &[0], &[0], -0.01);
    assert_eq!(scores_neg_tol, vec![0.0]);

    // 6. Inf and NaN handling in peaks
    let m_nan = vec![vec![f64::NAN, f64::INFINITY, -10.0, 100.0]];
    let w_nan = vec![vec![0.5f32, 0.5f32, 0.5f32, 0.5f32]];
    let bm_nan = bin_matrices(&m_nan, &w_nan, &p, 1.0, 500.0);
    // Only the peak at 100.0 should be registered
    let mut non_zero_count = 0;
    if let Some(row) = bm_nan.f.first() {
        for &val in row {
            if val > 0.0 {
                non_zero_count += 1;
            }
        }
    }
    assert_eq!(non_zero_count, 1);

    // 7. Exact boundary equality tests
    let u_exact = vec![vec![0.3f32]];
    let (ia_exact, ib_exact) = pairs_above(&u_exact, 0.3, false);
    assert_eq!(ia_exact, vec![0]);
    assert_eq!(ib_exact, vec![0]);

    let m_both = vec![vec![100.0], vec![100.5]];
    let w_both = vec![vec![1.0], vec![1.0]];
    let sc_both = plain_cosine(&m_both, &w_both, &[0], &[1], 0.5);
    assert!((sc_both[0] - 1.0).abs() < 1e-9);
}

#[test]
fn test_all_pairs_mutational_properties() {
    let m = vec![
        vec![100.000, 200.000, 300.000],
        vec![100.002, 200.003, 300.001],
    ];
    let w32 = vec![
        vec![0.577f32, 0.577f32, 0.577f32],
        vec![0.577f32, 0.577f32, 0.577f32],
    ];
    let w64 = vec![
        vec![0.57735, 0.57735, 0.57735],
        vec![0.57735, 0.57735, 0.57735],
    ];
    let p = vec![400.0, 400.0];

    // Property 1: Smear dominance: Xs[i][b] >= X[i][b]
    let bm = bin_matrices(&m, &w32, &p, 1.0, 500.0);
    for row in 0..bm.f.len() {
        for b in 0..bm.n_bins {
            assert!(bm.fs[row][b] >= bm.f[row][b]);
            assert!(bm.ls[row][b] >= bm.l[row][b]);
        }
    }

    // Property 2: Monotonicity of pairs_above with threshold
    let u = upper_bound(&bm, None, None);
    let (ia_low, _) = pairs_above(&u, 0.1, true);
    let (ia_mid, _) = pairs_above(&u, 0.5, true);
    let (ia_high, _) = pairs_above(&u, 1.5, true);
    assert!(ia_low.len() >= ia_mid.len());
    assert!(ia_mid.len() >= ia_high.len());

    // Property 3: Shifting peaks beyond tolerance drops cosine score to 0
    let sc_exact = plain_cosine(&m, &w64, &[0], &[1], 0.005)[0];
    assert!(sc_exact > 0.9);

    let m_shifted = vec![
        vec![100.000, 200.000, 300.000],
        vec![105.000, 205.000, 305.000],
    ];
    let sc_shifted = plain_cosine(&m_shifted, &w64, &[0], &[1], 0.005)[0];
    assert_eq!(sc_shifted, 0.0);

    // Property 4: Weight scaling: doubling weights quadruples candidate products
    let w64_doubled = vec![
        vec![0.57735 * 2.0, 0.57735 * 2.0, 0.57735 * 2.0],
        vec![0.57735 * 2.0, 0.57735 * 2.0, 0.57735 * 2.0],
    ];
    let sc_doubled = plain_cosine(&m, &w64_doubled, &[0], &[1], 0.005)[0];
    assert!((sc_doubled - sc_exact * 4.0).abs() < 1e-6);
}
