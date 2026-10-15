//! Gap filler of mzmine 3.9.0 rewritten in Rust (`untargeted::gapfill`): the cells that `gapfilled.csv` fills and `aligned.csv` leaves empty are the
//! cells that the Rust filler fills from the `.mzidx` of the four synthetic files, with the same m/z, RT, height and area. The mzML files are not in
//! the repository: `tools/bench/oro_mzmine.py --genera` writes them (python3 with numpy; the test is skipped without). The gold CSV keeps four significant
//! digits, so the areas agree within 6e-4 relative (half of the last digit), not within 1e-6.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use mzlab_core::idxread::Index;
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::upload::prepare;
use mzlab_feat::untargeted::gapfill::{fill, GapParams};
use mzlab_feat::untargeted::join::{Row, Rows};
use mzlab_feat::untargeted::table::to_aligned;
use mzlab_feat::untargeted::Feature;

type Rec = BTreeMap<String, String>;

fn csv(name: &str) -> Vec<Rec> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/data/untargeted/mzmine39")
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

fn num(r: &Rec, k: &str) -> f64 {
    r[k].parse().unwrap()
}

fn index_of(path: &Path) -> Index<Vec<u8>> {
    let mzml = std::fs::read(path).unwrap();
    let mut src = mzml.clone();
    let p = prepare(&mut src, mzml.len() as u64, "x.mzML").unwrap();
    let bytes = build_mzml_to_vec(&p.table, &mzml, 64 << 20).unwrap().0;
    let n = bytes.len() as u64;
    Index::open(bytes, n).unwrap()
}

/// The rows of `aligned.csv` with the cells that the join left (a detected feature with the numbers of the CSV).
fn rows_of(recs: &[Rec], files: &[String]) -> (Vec<String>, Rows) {
    let mut ids: Vec<String> = Vec::new();
    let mut rows: Vec<Row> = Vec::new();
    for r in recs {
        let at = ids.iter().position(|i| *i == r["row"]).unwrap_or_else(|| {
            ids.push(r["row"].clone());
            rows.push(Row {
                mz: num(r, "mz"),
                rt: num(r, "rt"),
                cells: vec![None; files.len()],
            });
            ids.len() - 1
        });
        let k = files.iter().position(|f| *f == r["file"]).unwrap();
        rows[at].cells[k] = Some(Feature {
            mz: num(r, "mz_file"),
            rt: num(r, "rt_file"),
            height: num(r, "height"),
            area: num(r, "area"),
            ..Feature::default()
        });
    }
    (
        ids,
        Rows {
            n_files: files.len(),
            rows,
        },
    )
}

#[test]
fn fills_the_cells_like_mzmine() {
    let dir = std::env::temp_dir().join(format!("mzlab_untargeted_gap_{}", std::process::id()));
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
        eprintln!("SKIP untargeted_gapfill: python3/numpy not available");
        return;
    }
    let files: Vec<String> = (1..=4).map(|i| format!("oro_{i}")).collect();
    let (ids, mut rows) = rows_of(&csv("aligned.csv"), &files);
    let before: Vec<Vec<bool>> = rows
        .rows
        .iter()
        .map(|r| r.cells.iter().map(Option::is_some).collect())
        .collect();
    let mut ixs: Vec<_> = files
        .iter()
        .map(|f| index_of(&dir.join(format!("{f}.mzML"))))
        .collect();
    // The mass detector of the batch keeps the peaks from 1e4 up; the scans are the positive MS1 ones.
    let p = GapParams {
        noise: 1e4,
        polarity: Some(1),
        ..GapParams::default()
    };
    let n = fill(&mut rows, &mut ixs, &p).unwrap();
    let _ = std::fs::remove_dir_all(&dir);

    // gold: the cells of gapfilled.csv that are not in aligned.csv
    let gold: Vec<Rec> = csv("gapfilled.csv");
    let mut want: BTreeMap<(String, String), &Rec> = BTreeMap::new();
    for r in &gold {
        let k = files.iter().position(|f| *f == r["file"]).unwrap();
        let at = ids.iter().position(|i| *i == r["row"]).unwrap();
        if !before[at][k] {
            assert_eq!(r["state"], "ESTIMATED");
            want.insert((r["row"].clone(), r["file"].clone()), r);
        }
    }
    assert!(
        want.len() >= 3,
        "the gold data has too few gaps: {}",
        want.len()
    );
    let (mut agree, mut spurious) = (0usize, 0usize);
    for (at, row) in rows.rows.iter().enumerate() {
        for (k, c) in row.cells.iter().enumerate() {
            let Some(f) = c.as_ref().filter(|f| f.filled) else {
                continue;
            };
            match want.get(&(ids[at].clone(), files[k].clone())) {
                Some(g) => {
                    let ok = (f.area - num(g, "area")).abs() <= 6e-4 * num(g, "area")
                        && (f.height - num(g, "height")).abs() <= 6e-4 * num(g, "height")
                        && (f.rt - num(g, "rt_file")).abs() <= 1e-4
                        && (f.mz - num(g, "mz_file")).abs() <= 2e-5;
                    assert!(ok, "row {} file {}: {f:?} against {g:?}", ids[at], files[k]);
                    agree += 1;
                }
                None => spurious += 1,
            }
        }
    }
    assert_eq!(n, agree + spurious);
    assert_eq!(spurious, 0, "cells filled that mzmine leaves empty");
    assert!(
        agree as f64 >= 0.99 * want.len() as f64,
        "cells filled like mzmine: {agree} of {}",
        want.len()
    );
    // the row RT is the mean again after the filling (row 2: 1.4975 in the gold file)
    for g in gold.iter().filter(|r| r["row"] == "2") {
        let at = ids.iter().position(|i| *i == g["row"]).unwrap();
        assert!((rows.rows[at].rt - num(g, "rt")).abs() <= 1e-4);
    }
    // the table: filled cells are `CELL_FILLED`, areas are in counts x seconds
    let t = to_aligned(&rows, &files);
    let filled = t
        .state
        .iter()
        .filter(|&&s| s == mzlab_feat::table::CELL_FILLED)
        .count();
    assert_eq!(filled, n);
    assert_eq!(t.len(), rows.rows.len());
}
