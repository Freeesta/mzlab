//! Statistics steps: parity of the PCA with `tests/golden/statistica_pca.json`, of the KMD with `tests/golden/kmd.json`, the null simulation of the
//! comparison (BH), the refusal with fewer than three replicates and the words the output never uses.

use mzlab_core::pca::PcaOpts;
use mzlab_feat::stat::{
    analyse, benjamini_hochberg, compare, kmd_json, pca_json, stats_json, Ion, KmdOpts, Layout,
    StatsOpts,
};
use serde_json::Value;
use std::path::PathBuf;

fn golden(name: &str) -> Value {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/golden")
        .join(name);
    serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
}

fn f(v: &Value) -> f64 {
    v.as_f64().unwrap()
}

fn grid(v: &Value) -> Vec<Vec<f64>> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|r| r.as_array().unwrap().iter().map(f).collect())
        .collect()
}

fn strs(v: &Value) -> Vec<String> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap().to_string())
        .collect()
}

fn close(a: f64, b: f64, tol: f64) {
    assert!((a - b).abs() <= tol, "{a} vs {b}");
}

#[test]
fn the_pca_matches_the_golden_of_the_python_reference() {
    let g = golden("statistica_pca.json");
    let v: Value =
        serde_json::from_str(&pca_json(g["input"].to_string().as_bytes()).unwrap()).unwrap();
    for (a, b) in v["explained"]
        .as_array()
        .unwrap()
        .iter()
        .zip(g["explained"].as_array().unwrap())
    {
        close(f(a), f(b), 1e-9);
    }
    for key in ["scores", "loadings"] {
        for (row, want) in grid(&v[key]).iter().zip(grid(&g[key])) {
            row.iter()
                .zip(&want)
                .for_each(|(a, b)| close(a.abs(), b.abs(), 1e-8));
        }
    }
    for (a, b) in v["stability"]
        .as_array()
        .unwrap()
        .iter()
        .zip(g["stability"].as_array().unwrap())
    {
        close(f(a), f(b), 1e-8);
    }
    for (a, b) in v["steps"]
        .as_array()
        .unwrap()
        .iter()
        .zip(g["steps"].as_array().unwrap())
    {
        assert_eq!((&a["step"], &a["n"]), (&b["step"], &b["n"]));
    }
    for k in ["imputed", "features", "samples"] {
        assert_eq!(v[k], g[k], "{k}");
    }
    for k in [
        "stability_pc",
        "ellipses",
        "transform",
        "scaling",
        "n_samples",
        "n_features",
    ] {
        assert!(v.get(k).is_some(), "{k}");
    }
}

#[test]
fn the_pca_is_refused_under_five_samples_and_rejects_unknown_scaling() {
    let few = pca_json(br#"{"matrix":[[1,2,3,4],[4,3,2,1],[1,5,2,6]],"roles":["sample","sample","sample","sample"]}"#).unwrap_err();
    assert!(few.contains("err.stat.pca.few"), "{few}");
    let bad = pca_json(br#"{"matrix":[[1,2,3,4,5,6]],"roles":["sample","sample","sample","sample","sample","sample"],"scaling":"range"}"#).unwrap_err();
    assert!(bad.contains("error"), "{bad}");
}

#[test]
fn the_kmd_matches_the_golden_of_the_javascript_reference() {
    let g = golden("kmd.json");
    for (name, case) in g.as_object().unwrap() {
        let inp = &case["input"];
        let ions: Vec<Ion> = inp["features"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| Ion {
                mz: f(&x["mz"]),
                rt: x["rt"].as_f64(),
            })
            .collect();
        let mut o = KmdOpts::default();
        o.unit = ["CF2", "CF2O", "C2F4", "CH2"]
            .into_iter()
            .find(|u| Some(*u) == inp["unit"].as_str())
            .unwrap_or("CF2");
        o.tol_mda = inp["tolMDa"].as_f64().unwrap_or(o.tol_mda);
        o.min_members = inp["minMembers"]
            .as_u64()
            .map_or(o.min_members, |n| n as usize);
        let r = analyse(&ions, &o);
        let rows = case["rows"].as_array().unwrap();
        assert_eq!(r.rows.len(), rows.len(), "{name}");
        for (a, b) in r.rows.iter().zip(rows) {
            close(a.km, f(&b["km"]), 1e-9);
            assert_eq!(a.nkm, f(&b["nkm"]), "{name}");
            close(a.kmd, f(&b["kmd"]), 1e-9);
            assert_eq!(a.series.map(|s| s as u64), b["series"].as_u64(), "{name}");
        }
        let series = case["series"].as_array().unwrap();
        assert_eq!(r.series.len(), series.len(), "{name}");
        for (a, b) in r.series.iter().zip(series) {
            let members: Vec<u64> = b["members"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_u64().unwrap())
                .collect();
            assert_eq!(
                a.members.iter().map(|m| *m as u64).collect::<Vec<_>>(),
                members,
                "{name}"
            );
            let steps: Vec<f64> = b["steps"].as_array().unwrap().iter().map(f).collect();
            assert_eq!(a.steps, steps, "{name}");
            close(a.kmd, f(&b["kmd"]), 1e-9);
            close(a.span_mda, f(&b["spanMDa"]), 1e-6);
            close(a.consecutive, f(&b["consecutive"]), 1e-12);
            assert_eq!(a.distinct as u64, b["distinct"].as_u64().unwrap());
            assert_eq!(a.rho.is_some(), b["rho"].as_f64().is_some(), "{name}");
            if let (Some(x), Some(y)) = (a.rho, b["rho"].as_f64()) {
                close(x, y, 1e-12);
            }
        }
    }
}

#[test]
fn the_kmd_json_lists_the_units_and_the_series() {
    let text = kmd_json(br#"{"features":[{"mz":212.97918,"rt":3},{"mz":262.97599,"rt":4},{"mz":312.97279,"rt":5}],"unit":"CF2"}"#).unwrap();
    let v: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v["series"].as_array().unwrap().len(), 1);
    assert_eq!(v["unit"]["nominal"], 50.0);
}

/// Deterministic uniform numbers (xorshift) so the simulation is the same everywhere.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
    fn normal(&mut self) -> f64 {
        (-2.0 * self.next().max(1e-300).ln()).sqrt() * (std::f64::consts::TAU * self.next()).cos()
    }
}

fn layout(groups: &[Option<usize>], names: &[&str]) -> Layout {
    Layout {
        names: names.iter().map(|s| s.to_string()).collect(),
        group_of: groups.to_vec(),
        blank: vec![false; groups.len()],
        reference: Some(0),
    }
}

#[test]
fn null_data_with_shuffled_labels_rarely_gives_a_small_q() {
    for seed in [1u64, 7, 99, 12345] {
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15 ^ seed);
        let n = 8;
        let m: Vec<Vec<f64>> = (0..1000)
            .map(|_| {
                let level = 10.0 + 2.0 * rng.normal();
                (0..n)
                    .map(|_| (level + 0.5 * rng.normal()).exp2())
                    .collect()
            })
            .collect();
        // Labels shuffled (Fisher-Yates) over the same files.
        let mut labels: Vec<usize> = (0..n).map(|i| i % 2).collect();
        for i in (1..n).rev() {
            labels.swap(i, (rng.next() * (i + 1) as f64) as usize % (i + 1));
        }
        let lay = layout(
            &labels.iter().map(|&g| Some(g)).collect::<Vec<_>>(),
            &["a", "b"],
        );
        let out = compare(&m, &lay, &[(0, 1)], &StatsOpts::default()).unwrap();
        let rows = &out.comparisons[0].rows;
        let hits = rows.iter().filter(|r| r.q < 0.05).count();
        assert!(
            hits as f64 <= 0.05 * rows.len() as f64,
            "seed {seed}: {hits} features with q < 0.05"
        );
        let raw = rows.iter().filter(|r| r.p < 0.05).count();
        assert!(raw > 10, "the raw p-values are not degenerate ({raw})");
    }
}

#[test]
fn a_real_difference_is_found_and_the_anova_agrees() {
    let mut rng = Rng(42);
    let m: Vec<Vec<f64>> = (0..200)
        .map(|r| {
            (0..9)
                .map(|c| {
                    let shift = if r < 20 && c >= 6 { 7.0 } else { 0.0 };
                    (12.0 + shift + 0.3 * rng.normal()).exp2()
                })
                .collect()
        })
        .collect();
    let groups: Vec<Option<usize>> = (0..9).map(|c| Some(c / 3)).collect();
    let out = compare(
        &m,
        &layout(&groups, &["ref", "x", "y"]),
        &[(0, 2)],
        &StatsOpts::default(),
    )
    .unwrap();
    let rows = &out.comparisons[0].rows;
    assert!(rows[..20].iter().all(|r| r.q < 0.05 && r.log2fc > 2.0));
    assert!(rows[20..].iter().filter(|r| r.q < 0.05).count() <= 2);
    let an = &out.anova.rows;
    assert!(an[..20].iter().all(|r| r.q < 0.05));
    assert_eq!(out.heat_groups, vec![0, 1, 2]);
}

#[test]
fn fewer_than_three_replicates_refuse_with_a_reason_and_no_number() {
    let m: Vec<Vec<f64>> = (0..5)
        .map(|r| {
            (0..7)
                .map(|c| 1000.0 * (1.0 + r as f64 + c as f64))
                .collect()
        })
        .collect();
    let groups = vec![
        Some(0),
        Some(0),
        Some(0),
        Some(1),
        Some(1),
        Some(2),
        Some(2),
    ];
    let out = compare(
        &m,
        &layout(&groups, &["a", "b", "c"]),
        &[(0, 1), (0, 2)],
        &StatsOpts::default(),
    )
    .unwrap();
    assert!(out
        .comparisons
        .iter()
        .all(|c| c.refused == Some("fewReplicates") && c.rows.is_empty()));
    assert_eq!(out.anova.refused, Some("fewReplicates"));
    assert!(out.anova.rows.is_empty() && out.heat.is_empty());
    assert_eq!(out.warnings.len(), 2);
    let json = stats_json(
        br#"{"matrix":[[1,2,3,4,5]],"groups":["a","a","a","b","b"],"comparisons":[["a","b"]]}"#,
    )
    .unwrap();
    let v: Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v["comparisons"][0]["refused"], "fewReplicates");
    assert_eq!(v["comparisons"][0]["rows"].as_array().unwrap().len(), 0);
    assert_eq!(v["warnings"][0]["code"], "fewReplicates");
}

#[test]
fn benjamini_hochberg_follows_the_definition() {
    let q = benjamini_hochberg(&[0.01, 0.04, 0.03, 0.5, f64::NAN]);
    close(q[0], 0.04, 1e-12);
    close(q[1], 0.0533333333333333, 1e-12);
    close(q[2], 0.0533333333333333, 1e-12);
    close(q[3], 0.5, 1e-12);
    assert!(q[4].is_nan());
}

#[test]
fn the_pseudocount_comes_from_the_blanks_then_the_reference_then_the_floor() {
    let m = vec![vec![0.0, 5000.0, 7000.0, 1.0e5, 2.0e5, 3.0e5]];
    let mut lay = layout(
        &[None, Some(0), Some(0), Some(1), Some(1), Some(1)],
        &["ref", "x"],
    );
    lay.blank = vec![true, false, false, false, false, false];
    let o = StatsOpts::default();
    assert_eq!(
        mzlab_feat::stat::pseudocount(&m, &lay, &o).unwrap().1,
        "reference"
    );
    lay.reference = None;
    assert_eq!(
        mzlab_feat::stat::pseudocount(&m, &lay, &o).unwrap(),
        (1000.0, "floor")
    );
    let given = StatsOpts {
        pseudo: Some(42.0),
        ..o
    };
    assert_eq!(
        mzlab_feat::stat::pseudocount(&m, &lay, &given).unwrap(),
        (42.0, "given")
    );
}

/// The output and the sources of the module never say a result is "significant": the word is joined here so this file is not a hit itself.
#[test]
fn the_word_significant_is_never_used() {
    let word = ["signif", "icant"].concat();
    let sources = [
        include_str!("../src/stat/mod.rs"),
        include_str!("../src/stat/compare.rs"),
        include_str!("../src/stat/json.rs"),
        include_str!("../src/stat/kmd.rs"),
        include_str!("../src/stat/special.rs"),
    ];
    assert!(sources.iter().all(|s| !s.to_lowercase().contains(&word)));
    let g = golden("statistica_pca.json");
    let out = [
        pca_json(g["input"].to_string().as_bytes()).unwrap(),
        stats_json(br#"{"matrix":[[1,2,3,4,5,6]],"groups":["a","a","a","b","b","b"],"comparisons":[["a","b"]]}"#).unwrap(),
        kmd_json(br#"{"features":[{"mz":212.97918}]}"#).unwrap(),
    ];
    assert!(out.iter().all(|s| !s.to_lowercase().contains(&word)));
    let steps = mzlab_core::step::json::registry_json(&mzlab_feat::steps::registry());
    assert!(!steps.to_lowercase().contains(&word));
}

#[test]
fn the_descriptors_carry_the_defaults_of_the_engine() {
    use mzlab_core::step::find;
    let reg = mzlab_feat::steps::registry();
    let pcs = find(&reg, "table.pca").unwrap();
    let d = PcaOpts::default();
    let num = |name: &str| pcs.param(name).unwrap().default_f64().unwrap();
    assert_eq!(
        (num("factor"), num("min_intensity")),
        (d.factor, d.min_intensity)
    );
    assert_eq!(
        (
            num("min_detect") as i64,
            num("min_samples") as i64,
            num("ncomp") as usize
        ),
        (d.min_detect, d.min_samples, d.ncomp)
    );
    let k = find(&reg, "table.kmd").unwrap();
    let kd = KmdOpts::default();
    assert_eq!(
        (
            k.param("tol_mda").unwrap().default_f64().unwrap(),
            k.param("min_members").unwrap().default_f64().unwrap() as usize
        ),
        (kd.tol_mda, kd.min_members)
    );
    let st = find(&reg, "table.stats").unwrap();
    assert_eq!(
        st.param("floor").unwrap().default_f64().unwrap(),
        StatsOpts::default().floor
    );
    assert!(st.param("pseudocount").unwrap().default.is_none());
    for id in ["table.stats", "table.pca", "table.kmd"] {
        let s = find(&reg, id).unwrap();
        assert!(
            s.runs_in.wasm.is_none() && s.runs_in.native,
            "{id} stays out of the .wasm table"
        );
    }
}
