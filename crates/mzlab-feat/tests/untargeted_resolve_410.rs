//! Parity of `untargeted::resolve` with the local-minimum resolver of mzmine 4.10 on the gold data of `tests/data/untargeted/mzmine410/`: the chromatograms of `adap.csv`
//! (their points per scan are in `adap_scansioni.csv`, written by `tools/bench/oro_scansioni.py`) go through `resolve` and the features are compared with `resolved.csv`.
//! The CSV of mzmine keeps four significant digits for height and area (`5.087E5`), so those two agree within 1.1e-3 relative, not 1e-6.
#![cfg(feature = "untargeted")]

use std::collections::BTreeMap;
use std::path::PathBuf;

use mzlab_feat::untargeted::resolve::{resolve, Chrom, Feature, MinParams};

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/data/untargeted/mzmine410")
}

fn rows(name: &str) -> Vec<BTreeMap<String, String>> {
    let text = std::fs::read_to_string(dir().join(name)).unwrap();
    let mut lines = text.lines();
    let head: Vec<&str> = lines.next().unwrap().split(',').collect();
    lines
        .map(|l| {
            head.iter()
                .zip(l.split(','))
                .map(|(h, v)| (h.to_string(), v.to_string()))
                .collect()
        })
        .collect()
}

fn num(r: &BTreeMap<String, String>, k: &str) -> f64 {
    r[k].parse().unwrap()
}

/// RT axis as mzmine holds it: a float (32 bit), so the comparisons with the search range see the same numbers.
fn axis() -> Vec<f64> {
    rows("adap_asse.csv")
        .iter()
        .map(|r| num(r, "rt") as f32 as f64)
        .collect()
}

/// (file, adap id) -> chromatogram on the whole run.
fn chromatograms(n: usize) -> BTreeMap<(String, String), Chrom> {
    let mut out: BTreeMap<(String, String), Chrom> = BTreeMap::new();
    for r in rows("adap_scansioni.csv") {
        let c = out
            .entry((r["file"].clone(), r["id"].clone()))
            .or_insert_with(|| Chrom {
                mz: vec![0.0; n],
                y: vec![0.0; n],
            });
        let i = num(&r, "scan") as usize;
        c.mz[i] = num(&r, "mz");
        c.y[i] = num(&r, "intensity");
    }
    out
}

fn close(a: f64, b: f64, rel: f64) -> bool {
    (a - b).abs() <= rel * b.abs()
}

fn same(f: &Feature, r: &BTreeMap<String, String>) -> bool {
    (f.mz - num(r, "mz")).abs() < 1e-4
        && (f.rt_apex - num(r, "rt")).abs() < 1e-4
        && (f.rt_start - num(r, "rt_start")).abs() < 1e-4
        && (f.rt_end - num(r, "rt_end")).abs() < 1e-4
        && close(f.height, num(r, "height"), 1.1e-3)
        && close(f.area, num(r, "area"), 1.1e-3)
        && close(f.area_s, num(r, "area") * 60.0, 1.1e-3)
}

/// `Some(())` when the 4.10 gold data are there; otherwise says why the test is skipped.
fn need_gold(test: &str) -> Option<()> {
    let dir = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/data/untargeted/mzmine410"
    );
    if std::path::Path::new(dir).exists() {
        return Some(());
    }
    eprintln!("SKIP {test}: tests/data/untargeted/mzmine410 is missing (the 4.10 gold data need an mzmine account: tools/bench/oro_mzmine410.py)");
    None
}

#[test]
fn resolver_matches_mzmine_410() {
    let Some(()) = need_gold("resolver_matches_mzmine_410") else {
        return;
    };
    let rt = axis();
    // mzmine 4.10 takes the zero next to an edge in (`adjustStartAndEnd`)
    let p = MinParams {
        adjust_edges: true,
        ..MinParams::default()
    };
    let expected = rows("resolved.csv");
    let adap = rows("adap.csv");
    let mut found: Vec<(String, String, Feature)> = Vec::new();
    for ((file, id), c) in chromatograms(rt.len()) {
        for f in resolve(&c, &rt, &p) {
            found.push((file.clone(), id.clone(), f));
        }
    }
    // each row of mzmine belongs to the chromatogram of the same file with the nearest m/z
    let owner = |r: &BTreeMap<String, String>| -> String {
        adap.iter()
            .filter(|a| a["file"] == r["file"])
            .min_by(|a, b| {
                (num(a, "mz") - num(r, "mz"))
                    .abs()
                    .total_cmp(&(num(b, "mz") - num(r, "mz")).abs())
            })
            .map(|a| a["id"].clone())
            .unwrap()
    };
    let mut identical = 0usize;
    let mut used = vec![false; found.len()];
    let mut count_diff = Vec::new();
    for r in &expected {
        let hit = found
            .iter()
            .enumerate()
            .find(|(k, (file, _, f))| !used[*k] && *file == r["file"] && same(f, r))
            .map(|(k, _)| k);
        match hit {
            Some(k) => {
                used[k] = true;
                identical += 1;
            }
            None => eprintln!("MISSING {:?}", r),
        }
    }
    for (k, (file, id, f)) in found.iter().enumerate() {
        if !used[k] {
            eprintln!("EXTRA {file} chrom {id}: {f:?}");
        }
    }
    for a in &adap {
        let want = expected
            .iter()
            .filter(|r| r["file"] == a["file"] && owner(r) == a["id"])
            .count();
        let got = found
            .iter()
            .filter(|(file, id, _)| *file == a["file"] && *id == a["id"])
            .count();
        if want != got {
            count_diff.push(format!(
                "{} chrom {}: mzmine {want}, ours {got}",
                a["file"], a["id"]
            ));
        }
    }
    let share = identical as f64 / expected.len().max(found.len()) as f64;
    eprintln!(
        "identical {identical} of {} (ours {}): {share:.4}",
        expected.len(),
        found.len()
    );
    assert!(
        count_diff.is_empty(),
        "different number of features per chromatogram: {count_diff:?}"
    );
    assert!(share >= 0.99, "share of identical features {share}");
}
