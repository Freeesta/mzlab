//! `stats` answers like the small statistics of `mzlab/ionfamily.py` (`tools/stats_riferimento.py`): ranks, correlations, Theil-Sen, chi-square
//! survival within 1e-9 relative, and the permutation p-values IDENTICAL (same numpy generator). Skipped without python3 and numpy
//! (`MZLAB_GOLDEN_STRICT=1` fails instead).

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::stats::*;
use serde_json::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn nums(v: &Value) -> Vec<f64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect()
}
fn opt(v: &Value) -> f64 {
    v.as_f64().unwrap_or(f64::NAN)
}
fn close(a: f64, b: f64) -> bool {
    (a.is_nan() && b.is_nan()) || a == b || (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1e-3)
}

#[test]
fn statistics_like_python() {
    let golden = root().join("tests/golden/ref/stats.json"); let committed = golden.exists();           // committed answers of Python: no python3 needed
    let out = if committed { golden } else { std::env::temp_dir().join(format!("mzlab_stats_{}.json", std::process::id())) };
    let ok = committed || Command::new("python3")
        .arg(root().join("tools/stats_riferimento.py"))
        .arg(&out)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !ok {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 or numpy missing"
        );
        eprintln!("skipped: python3/numpy not available");
        return;
    }
    let d: Value = serde_json::from_slice(&std::fs::read(&out).unwrap()).unwrap();
    if !committed { let _ = std::fs::remove_file(&out); }
    let mut perms = 0;
    for (i, c) in d["cases"].as_array().unwrap().iter().enumerate() {
        let (x, y, w) = (nums(&c["x"]), nums(&c["y"]), nums(&c["w"]));
        let chk = |what: &str, got: f64, want: f64| {
            assert!(
                close(got, want),
                "case {i} (n={}) {what}: {got} against {want}",
                x.len()
            )
        };
        for (g, wnt) in rankdata(&x).iter().zip(nums(&c["rank"])) {
            chk("rank", *g, wnt);
        }
        chk("pearson", wpearson(&x, &y, None), opt(&c["pearson"]));
        chk(
            "pearson_w",
            wpearson(&x, &y, Some(&w)),
            opt(&c["pearson_w"]),
        );
        chk("spearman", wspearman(&x, &y, None), opt(&c["spearman"]));
        chk(
            "spearman_w",
            wspearman(&x, &y, Some(&w)),
            opt(&c["spearman_w"]),
        );
        chk("kendall", kendall_tau(&x, &y), opt(&c["kendall"]));
        let (s, a) = theil_sen(&x, &y, 80);
        chk("ts slope", s, opt(&c["ts"][0]));
        chk("ts intercept", a, opt(&c["ts"][1]));
        let (s, a) = theil_sen(&x, &y, 30);
        chk("ts30 slope", s, opt(&c["ts30"][0]));
        chk("ts30 intercept", a, opt(&c["ts30"][1]));
        if c.get("perm").is_some() {
            let p = perm_pvalue(Stat::Spearman, &x, &y, 4000, 0);
            assert!(
                p.is_nan() && c["perm"].is_null() || p == c["perm"].as_f64().unwrap(),
                "case {i} (n={}) perm: {p} against {}",
                x.len(),
                c["perm"]
            );
            let p = perm_pvalue(Stat::Kendall, &x, &y, 500, 3);
            assert!(
                p.is_nan() && c["perm_k"].is_null() || p == c["perm_k"].as_f64().unwrap(),
                "case {i} perm_k: {p} against {}",
                c["perm_k"]
            );
            perms += 1;
        }
    }
    assert!(perms > 40);
    for r in d["chi"].as_array().unwrap() {
        let got = chi2_sf(r[0].as_f64().unwrap(), r[1].as_u64().unwrap() as u32);
        assert!(
            close(got, r[2].as_f64().unwrap()),
            "chi2_sf({}, {}): {got} against {}",
            r[0],
            r[1],
            r[2]
        );
    }
}
