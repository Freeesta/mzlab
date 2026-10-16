//! Parity tests for ROI windowed correlation and average linkage clustering
//! against `mzlab/ionfamily.py` via `tools/roi_cluster_riferimento.py`.

#![allow(clippy::disallowed_methods)]

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::roi_cluster::{average_linkage, cluster_rois, pairwise_windowed_corr};
use serde_json::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn close_1e12(a: f64, b: f64) -> bool {
    (a.is_nan() && b.is_nan()) || a == b || (a - b).abs() <= 1e-12 * a.abs().max(b.abs()).max(1e-3)
}

fn nums_2d(v: &Value) -> Vec<Vec<f64>> {
    v.as_array()
        .map(|rows| {
            rows.iter()
                .map(|row| {
                    row.as_array()
                        .map(|vals| {
                            vals.iter()
                                .map(|x| x.as_f64().unwrap_or(f64::NAN))
                                .collect()
                        })
                        .unwrap_or_default()
                })
                .collect()
        })
        .unwrap_or_default()
}

fn nums_1d(v: &Value) -> Vec<f64> {
    v.as_array()
        .map(|vals| {
            vals.iter()
                .map(|x| x.as_f64().unwrap_or(f64::NAN))
                .collect()
        })
        .unwrap_or_default()
}

fn usizes_1d(v: &Value) -> Vec<usize> {
    v.as_array()
        .map(|vals| {
            vals.iter()
                .map(|x| x.as_u64().unwrap_or(0) as usize)
                .collect()
        })
        .unwrap_or_default()
}

fn verify_trace_case(case: &Value) {
    let case_id = case["id"].as_str().unwrap_or("");
    let traces = nums_2d(&case["traces"]);
    let thr = case["thr"].as_f64().unwrap_or(0.05);
    let min_overlap = case["min_overlap"].as_u64().unwrap_or(4) as usize;
    let cut = case["cut"].as_f64().unwrap_or(0.3);

    let want_corr = nums_2d(&case["corr"]);
    let want_labels = usizes_1d(&case["labels"]);
    let want_heights = nums_1d(&case["heights"]);
    let want_cluster_labels = usizes_1d(&case["cluster_labels"]);
    let want_cluster_heights = nums_1d(&case["cluster_heights"]);

    let got_corr = pairwise_windowed_corr(&traces, thr, min_overlap);
    assert_eq!(
        got_corr.len(),
        want_corr.len(),
        "case {case_id}: corr row count"
    );
    for (i, (got_row, want_row)) in got_corr.iter().zip(&want_corr).enumerate() {
        assert_eq!(
            got_row.len(),
            want_row.len(),
            "case {case_id} row {i}: col count"
        );
        for (j, (&g, &w)) in got_row.iter().zip(want_row).enumerate() {
            assert!(
                close_1e12(g, w),
                "case {case_id} corr[{i}][{j}]: got {g} want {w}"
            );
        }
    }

    let dist = nums_2d(&case["dist"]);
    let (got_labels, got_heights) = average_linkage(&dist, cut);
    assert_eq!(got_labels, want_labels, "case {case_id}: labels mismatch");
    assert_eq!(
        got_heights.len(),
        want_heights.len(),
        "case {case_id}: heights count"
    );
    for (idx, (&g, &w)) in got_heights.iter().zip(&want_heights).enumerate() {
        assert!(
            close_1e12(g, w),
            "case {case_id} heights[{idx}]: got {g} want {w}"
        );
    }

    let (got_cl_labels, got_cl_heights) = cluster_rois(&traces, thr, min_overlap, cut);
    assert_eq!(
        got_cl_labels, want_cluster_labels,
        "case {case_id}: cluster labels"
    );
    assert_eq!(
        got_cl_heights.len(),
        want_cluster_heights.len(),
        "case {case_id}: cluster heights"
    );
    for (idx, (&g, &w)) in got_cl_heights.iter().zip(&want_cluster_heights).enumerate() {
        assert!(
            close_1e12(g, w),
            "case {case_id} cluster heights[{idx}]: got {g} want {w}"
        );
    }
}

fn verify_linkage_case(case: &Value) {
    let case_id = case["id"].as_str().unwrap_or("");
    let dist = nums_2d(&case["dist"]);
    let cut = case["cut"].as_f64().unwrap_or(0.3);
    let want_labels = usizes_1d(&case["labels"]);
    let want_heights = nums_1d(&case["heights"]);

    let (got_labels, got_heights) = average_linkage(&dist, cut);
    assert_eq!(
        got_labels, want_labels,
        "linkage case {case_id}: labels mismatch"
    );
    assert_eq!(
        got_heights.len(),
        want_heights.len(),
        "linkage case {case_id}: heights count"
    );
    for (idx, (&g, &w)) in got_heights.iter().zip(&want_heights).enumerate() {
        assert!(
            close_1e12(g, w),
            "linkage case {case_id} heights[{idx}]: got {g} want {w}"
        );
    }
}

#[test]
fn roi_cluster_matches_python() {
    let out = std::env::temp_dir().join(format!("mzlab_roi_cluster_{}.json", std::process::id()));
    let ok = Command::new("python3")
        .arg(root().join("tools/roi_cluster_riferimento.py"))
        .arg(&out)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !ok {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 missing or roi_cluster_riferimento.py failed"
        );
        eprintln!("skipped: python3 not available");
        return;
    }

    let data_bytes = std::fs::read(&out).expect("failed to read generated reference json");
    let _ = std::fs::remove_file(&out);
    let all: Value = serde_json::from_slice(&data_bytes).expect("invalid json");

    let trace_cases = all["trace_cases"].as_array().expect("trace_cases missing");
    assert!(trace_cases.len() >= 40, "expected at least 40 trace cases");
    for case in trace_cases {
        verify_trace_case(case);
    }

    let linkage_cases = all["linkage_cases"]
        .as_array()
        .expect("linkage_cases missing");
    for case in linkage_cases {
        verify_linkage_case(case);
    }
}

#[test]
fn mutational_and_negative_tests() {
    // 1. Empty traces
    let empty_corr = pairwise_windowed_corr(&[], 0.05, 4);
    assert!(empty_corr.is_empty());
    let (empty_lbl, empty_ht) = average_linkage(&[], 0.3);
    assert!(empty_lbl.is_empty() && empty_ht.is_empty());
    let (cl_lbl, cl_ht) = cluster_rois(&[], 0.05, 4, 0.3);
    assert!(cl_lbl.is_empty() && cl_ht.is_empty());

    // 2. Single trace
    let single_trace = vec![vec![0.0, 1.0, 5.0, 10.0, 5.0, 1.0, 0.0]];
    let (s_lbl, s_ht) = cluster_rois(&single_trace, 0.05, 4, 0.3);
    assert_eq!(s_lbl, vec![0]);
    assert!(s_ht.is_empty());

    // 3. Impossible threshold (> 1.0) -> all NaN
    let traces = vec![
        vec![0.0, 1.0, 5.0, 10.0, 5.0, 1.0, 0.0],
        vec![0.0, 1.0, 5.0, 10.0, 5.0, 1.0, 0.0],
    ];
    let r_high_thr = pairwise_windowed_corr(&traces, 1.5, 3);
    assert!(r_high_thr[0][0].is_nan());
    assert!(r_high_thr[0][1].is_nan());
    assert!(r_high_thr[1][0].is_nan());
    assert!(r_high_thr[1][1].is_nan());

    // 4. Impossible min_overlap -> all NaN
    let r_high_overlap = pairwise_windowed_corr(&traces, 0.05, 100);
    assert!(r_high_overlap[0][1].is_nan());

    // 5. Negative cut -> no merges performed
    let dist = vec![
        vec![0.0, 0.1, 0.5],
        vec![0.1, 0.0, 0.4],
        vec![0.5, 0.4, 0.0],
    ];
    let (neg_lbl, neg_ht) = average_linkage(&dist, -0.01);
    assert_eq!(neg_lbl, vec![0, 1, 2]);
    assert!(neg_ht.is_empty());

    // 6. Cut = 0.0 -> no merges if distances > 0
    let (zero_lbl, zero_ht) = average_linkage(&dist, 0.0);
    assert_eq!(zero_lbl, vec![0, 1, 2]);
    assert!(zero_ht.is_empty());

    // 7. All infinity matrix -> halts immediately
    let inf_dist = vec![
        vec![f64::INFINITY, f64::INFINITY],
        vec![f64::INFINITY, f64::INFINITY],
    ];
    let (inf_lbl, inf_ht) = average_linkage(&inf_dist, 0.5);
    assert_eq!(inf_lbl, vec![0, 1]);
    assert!(inf_ht.is_empty());

    // 8. Scale invariance: scaling a trace by 10x should not change pairwise_windowed_corr
    let t1 = vec![0.0, 1.0, 4.0, 8.0, 4.0, 1.0, 0.0];
    let t2 = vec![0.0, 2.0, 8.0, 16.0, 8.0, 2.0, 0.0];
    let r1 = pairwise_windowed_corr(&[t1.clone(), t1.clone()], 0.05, 4);
    let r2 = pairwise_windowed_corr(&[t1, t2], 0.05, 4);
    assert!(close_1e12(r1[0][1], r2[0][1]));

    // 9. Cut monotonicity: larger cut leads to <= number of clusters (more or equal merges)
    let (lbl_strict, _) = average_linkage(&dist, 0.05);
    let (lbl_mid, _) = average_linkage(&dist, 0.2);
    let (lbl_wide, _) = average_linkage(&dist, 0.6);
    let count_clusters = |lbls: &[usize]| {
        let mut s = lbls.to_vec();
        s.sort_unstable();
        s.dedup();
        s.len()
    };
    assert!(count_clusters(&lbl_strict) >= count_clusters(&lbl_mid));
    assert!(count_clusters(&lbl_mid) >= count_clusters(&lbl_wide));
}
