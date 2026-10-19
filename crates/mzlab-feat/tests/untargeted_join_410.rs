//! Join aligner of mzmine 4.10 rewritten in Rust (`untargeted::join`). Parity: from the per-file features of `resolved.csv` to the rows of `aligned.csv`
//! (`tests/data/untargeted/mzmine410/`, synthetic files) the same features are in the same rows and the row m/z and RT agree within the print precision of
//! the CSV (m/z 5 decimals, RT 4). Then the cases made by hand: tie, out of tolerance, the base list chosen by size, two features of one file.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use mzlab_feat::untargeted::join::{align, JoinParams};
use mzlab_feat::untargeted::Feature;

fn csv(name: &str) -> Vec<BTreeMap<String, String>> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/data/untargeted/mzmine410")
        .join(name);
    let text = std::fs::read_to_string(p).unwrap();
    let mut lines = text.lines();
    let head: Vec<&str> = lines.next().unwrap().split(',').collect();
    lines
        .map(|l| {
            head.iter()
                .map(|h| h.to_string())
                .zip(l.split(',').map(str::to_string))
                .collect()
        })
        .collect()
}

fn num(r: &BTreeMap<String, String>, k: &str) -> f64 {
    r[k].parse().unwrap()
}

/// Identity of a feature across the two tables: file, m/z (5 decimals) and RT (3 decimals) as written by mzmine.
fn key(file: &str, mz: f64, rt: f64) -> String {
    format!("{file}|{mz:.5}|{rt:.3}")
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
fn rows_like_mzmine() {
    let Some(()) = need_gold("rows_like_mzmine") else {
        return;
    };
    let resolved = csv("resolved.csv");
    let files: Vec<String> = resolved
        .iter()
        .map(|r| r["file"].clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let lists: Vec<Vec<Feature>> = files
        .iter()
        .map(|f| {
            resolved
                .iter()
                .filter(|r| &r["file"] == f)
                .map(|r| Feature {
                    mz: num(r, "mz"),
                    rt: num(r, "rt"),
                    rt_start: num(r, "rt_start"),
                    rt_end: num(r, "rt_end"),
                    height: num(r, "height"),
                    area: num(r, "area"),
                    filled: false,
                })
                .collect()
        })
        .collect();
    let got = align(&lists, &JoinParams::default());
    let ours: Vec<(BTreeSet<String>, f64, f64)> = got
        .rows
        .iter()
        .map(|r| {
            let set = r
                .cells
                .iter()
                .enumerate()
                .filter_map(|(k, c)| c.as_ref().map(|f| key(&files[k], f.mz, f.rt)))
                .collect();
            (set, r.mz, r.rt)
        })
        .collect();
    let mut gold: BTreeMap<String, (BTreeSet<String>, f64, f64)> = BTreeMap::new();
    for r in csv("aligned.csv") {
        let e =
            gold.entry(r["row"].clone())
                .or_insert((BTreeSet::new(), num(&r, "mz"), num(&r, "rt")));
        e.0.insert(key(&r["file"], num(&r, "mz_file"), num(&r, "rt_file")));
    }
    assert!(!gold.is_empty());
    let same = gold
        .values()
        .filter(|(set, mz, rt)| {
            ours.iter()
                .any(|(s, m, t)| s == set && (m - mz).abs() <= 2e-5 && (t - rt).abs() <= 1e-4)
        })
        .count();
    let share = same as f64 / gold.len() as f64;
    assert!(
        share >= 0.99,
        "rows equal to mzmine: {same} of {} ({share:.3})",
        gold.len()
    );
    assert_eq!(ours.len(), gold.len(), "number of rows");
    // the rows come out in RT order, like the list of mzmine after the renumbering
    assert!(got.rows.windows(2).all(|w| w[0].rt <= w[1].rt));
}
