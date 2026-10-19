//! The whole pipeline of `untargeted::pipeline` (ADAP -> resolver -> join -> gap filling -> table) on the four synthetic files of the gold data
//! (`tests/data/untargeted/mzmine410/`) against `gapfilled.csv`, the final list of mzmine 4.10: the rows and the cells (feature or filled cell, with
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
use mzlab_feat::untargeted::pipeline::{run_file, run_series, UntParams};

type Rec = BTreeMap<String, String>;

fn gold() -> Vec<Rec> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/data/untargeted/mzmine410/gapfilled.csv");
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
fn pipeline_matches_the_final_list_of_mzmine() {
    let Some(()) = need_gold("pipeline_matches_the_final_list_of_mzmine") else {
        return;
    };
    let Some(mut ixs) = synthetic() else { return };
    // the default of the product: mzmine 4.10 takes the zero next to an edge in (`adjustStartAndEnd`)
    let p = UntParams {
        polarity: Some(1),
        ..UntParams::default()
    };
    assert!(p.resolve.adjust_edges);
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
