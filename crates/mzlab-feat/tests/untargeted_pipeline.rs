//! The whole pipeline of `untargeted::pipeline` (ADAP -> resolver -> join -> gap filling -> table) on the four synthetic files of the gold data
//! (`tests/data/untargeted/mzmine39/`) against `gapfilled.csv`, the final list of mzmine 3.9.0: the rows and the cells (feature or filled cell, with
//! height and area) agree for at least 0.99 of the gold cells and the table has no cell that mzmine leaves empty. The mzML files are written by
//! `tools/bench/oro_mzmine.py --genera` (python3 with numpy; skipped without). The gold CSV keeps four significant digits: heights and areas agree
//! within 1.2e-3 relative. Also the recipe JSON and the round trip of the store.
#![cfg(feature = "untargeted")]

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use mzlab_core::idxread::Index;
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::upload::prepare;
use mzlab_feat::table::{Aligned, CELL_ABSENT, CELL_FILLED};
use mzlab_feat::untargeted::pipeline::{
    from_file_features, run_file, run_series, to_file_features, BaselineParams, UntParams,
};

type Rec = BTreeMap<String, String>;

fn gold() -> Vec<Rec> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/data/untargeted/mzmine39/gapfilled.csv");
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

fn index_of(path: &Path) -> Index<Vec<u8>> {
    let mzml = std::fs::read(path).unwrap();
    let mut src = mzml.clone();
    let p = prepare(&mut src, mzml.len() as u64, "x.mzML").unwrap();
    let bytes = build_mzml_to_vec(&p.table, &mzml, 64 << 20).unwrap().0;
    let n = bytes.len() as u64;
    Index::open(bytes, n).unwrap()
}

fn num(r: &Rec, k: &str) -> f64 {
    r[k].parse().unwrap()
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1.2e-3 * b.abs()
}

fn synthetic() -> Option<Vec<Index<Vec<u8>>>> {
    // one folder per call: the tests run in parallel and each removes its own
    static CALLS: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "mzlab_untargeted_pipe_{}_{}",
        std::process::id(),
        CALLS.fetch_add(1, Ordering::SeqCst)
    ));
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let ok = Command::new("python3")
        .arg(root.join("tools/bench/oro_mzmine.py"))
        .arg("--genera")
        .arg(&dir)
        .current_dir(&root)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !ok {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 or numpy missing"
        );
        eprintln!("SKIP untargeted_pipeline: python3/numpy not available");
        return None;
    }
    let ixs = (1..=4)
        .map(|i| index_of(&dir.join(format!("oro_{i}.mzML"))))
        .collect();
    let _ = std::fs::remove_dir_all(&dir);
    Some(ixs)
}

/// Rows of the table that hold a cell of file `k` near (mz, rt): (row, cell).
fn find(t: &Aligned, mz: f64, rt: f64) -> Option<usize> {
    (0..t.len()).find(|&r| (t.mz[r] - mz).abs() < 2e-4 && (t.rt[r] - rt).abs() < 0.02)
}

#[test]
fn pipeline_matches_the_final_list_of_mzmine() {
    let Some(mut ixs) = synthetic() else { return };
    let mut p = UntParams {
        polarity: Some(1),
        ..UntParams::default()
    };
    // the gold data are those of mzmine 3.9.0: no zero next to the edges (the default of the product follows 4.10)
    p.resolve.adjust_edges = false;
    let lists: Vec<_> = ixs.iter_mut().map(|ix| run_file(ix, &p).unwrap()).collect();
    let files: Vec<String> = (1..=4).map(|i| format!("oro_{i}")).collect();
    let t = run_series(&lists, &mut ixs, &files, &p).unwrap();
    let g = gold();
    let nrows = g
        .iter()
        .map(|r| r["row"].clone())
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    eprintln!("pipeline: {} rows against {nrows} of mzmine", t.len());
    let (mut agree, mut gold_cells) = (0usize, 0usize);
    let mut seen = vec![false; t.len() * files.len()];
    for r in &g {
        gold_cells += 1;
        let k = files.iter().position(|f| *f == r["file"]).unwrap();
        let Some(row) = find(&t, num(r, "mz"), num(r, "rt")) else {
            continue;
        };
        let c = row * files.len() + k;
        seen[c] = true;
        let filled = t.state[c] == CELL_FILLED;
        if t.state[c] != CELL_ABSENT
            && filled == (r["state"] == "ESTIMATED")
            && close(t.height[c] as f64, num(r, "height"))
            && close(t.area[c], num(r, "area") * 60.0)
        {
            agree += 1;
        } else {
            eprintln!(
                "cell off: row {} {} -> {:?}",
                r["row"], r["file"], t.state[c]
            );
        }
    }
    let spurious = (0..t.len() * files.len())
        .filter(|&c| t.state[c] != CELL_ABSENT && !seen[c])
        .count();
    eprintln!("cells: {agree}/{gold_cells} like mzmine, {spurious} spurious");
    assert_eq!(t.len(), nrows);
    assert!(
        agree as f64 >= 0.99 * gold_cells as f64,
        "{agree}/{gold_cells}"
    );
    assert_eq!(spurious, 0);
}

#[test]
fn store_round_trip_keeps_the_list() {
    let Some(mut ixs) = synthetic() else { return };
    let p = UntParams::default();
    let list = run_file(&mut ixs[0], &p).unwrap();
    assert!(!list.is_empty());
    assert_eq!(from_file_features(&to_file_features(&list)), list);
}

#[test]
fn recipe_json_overrides_only_what_it_names() {
    let d = UntParams::default();
    let p = UntParams::from_json(
        r#"{"engine":"untargeted","polarity":-1,"adap":{"noise":5000,"min_consecutive":3},"resolve":{"duration":[0.1,2]},"join":{"rt_tol":0.3},"gap":{"min_points":2}}"#,
    )
    .unwrap();
    assert_eq!(p.polarity, Some(-1));
    assert_eq!(p.gap.polarity, Some(-1));
    assert_eq!(p.gap.noise, 5000.0);
    assert_eq!((p.adap.noise, p.adap.min_consecutive), (5000.0, 3));
    assert_eq!(p.adap.tol_ppm, d.adap.tol_ppm);
    assert_eq!(p.resolve.duration, (0.1, 2.0));
    assert_eq!(p.join.rt_tol, 0.3);
    assert_eq!(p.join.tol_ppm, d.join.tol_ppm);
    assert_eq!(p.gap.min_points, 2);
    assert_eq!(UntParams::from_json("").unwrap().adap, d.adap);
    for bad in [
        "{",
        r#"{"engine":"mzfinder"}"#,
        r#"{"adap":{"noise":-1}}"#,
        r#"{"resolve":{"duration":[2,1]}}"#,
    ] {
        assert!(UntParams::from_json(bad).is_err(), "{bad}");
    }
}

#[test]
fn adjust_edges_is_on_by_default_and_the_recipe_can_turn_it_off() {
    assert!(UntParams::default().resolve.adjust_edges);
    assert!(UntParams::from_json("").unwrap().resolve.adjust_edges);
    let off = UntParams::from_json(r#"{"resolve":{"adjust_edges":false}}"#).unwrap();
    assert!(!off.resolve.adjust_edges);
    assert!(UntParams::from_json(r#"{"resolve":{"adjust_edges":3}}"#).is_err());
}

#[test]
fn baseline_is_off_by_default_and_the_recipe_turns_it_on() {
    // mzmine 4.10 gold batches have no baseline step, so parity needs it off
    assert!(UntParams::default().baseline.is_none());
    assert!(UntParams::from_json(r#"{"baseline":null}"#)
        .unwrap()
        .baseline
        .is_none());
    assert!(UntParams::from_json(r#"{"baseline":false}"#)
        .unwrap()
        .baseline
        .is_none());
    let d = BaselineParams::default();
    assert_eq!((d.lambda, d.max_iter, d.exclude_peaks), (1e5, 50, false));
    assert_eq!(
        UntParams::from_json(r#"{"baseline":true}"#)
            .unwrap()
            .baseline,
        Some(d)
    );
    let p = UntParams::from_json(r#"{"baseline":{"lambda":1e4,"exclude_peaks":true}}"#).unwrap();
    assert_eq!(
        p.baseline.map(|b| (b.lambda, b.max_iter, b.exclude_peaks)),
        Some((1e4, 50, true))
    );
    for bad in [
        r#"{"baseline":{"lambda":0}}"#,
        r#"{"baseline":{"exclude_peaks":2}}"#,
    ] {
        assert!(UntParams::from_json(bad).is_err(), "{bad}");
    }
}

#[test]
fn baseline_apply_is_the_arpls_correction_of_the_trace() {
    // a flat offset of 100 under a triangular peak: arPLS removes the offset (Baek et al. 2015)
    let x: Vec<f64> = (0..60).map(|i| i as f64 * 0.05).collect();
    let y: Vec<f64> = (0..60)
        .map(|i| 100.0 + (50.0 - 10.0 * (i as f64 - 30.0).abs()).max(0.0))
        .collect();
    let b = BaselineParams::default();
    assert_eq!(
        b.apply(&x, &y),
        mzlab_feat::untargeted::arpls::correct(&y, 1e5, 50)
    );
    let out = b.apply(&x, &y);
    assert!(out[0] < 1.0 && out[59] < 1.0, "offset removed at the edges");
    assert!(out[30] > 40.0, "the peak stays");
}

#[test]
fn baseline_in_the_recipe_changes_the_features_of_the_synthetic_files() {
    let Some(mut ixs) = synthetic() else { return };
    let off = run_file(&mut ixs[0], &UntParams::default()).unwrap();
    let on = UntParams::from_json(r#"{"baseline":true}"#).unwrap();
    let with = run_file(&mut ixs[0], &on).unwrap();
    assert!(!off.is_empty());
    assert_ne!(
        off.iter().map(|f| f.height).collect::<Vec<_>>(),
        with.iter().map(|f| f.height).collect::<Vec<_>>()
    );
}
