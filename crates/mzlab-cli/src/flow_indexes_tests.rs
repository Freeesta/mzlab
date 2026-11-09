//! Each index is opened once per step of the alignment (`flow_indexes`): the base chain counts the full opens (`flow_base::open`) of every `.mzidx`, on a
//! fresh run and on a run that reuses both tables; the tables are those of the page either way, and a changed index file is opened again, not copied.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::flow_run_tests::{fixture, flow_run, p};
use crate::tests::page_table;

/// How many times each index path was opened in full, over every test of the crate (each test has its own folder).
static OPENS: Mutex<Option<HashMap<PathBuf, usize>>> = Mutex::new(None);

pub(crate) fn opened(path: &Path) {
    let mut m = OPENS.lock().unwrap_or_else(|e| e.into_inner());
    *m.get_or_insert_with(HashMap::new)
        .entry(path.to_path_buf())
        .or_insert(0) += 1;
}

/// The full opens of the indexes of `out`, by file name, since the last call for that folder.
fn take(out: &Path) -> Vec<(String, usize)> {
    let mut m = OPENS.lock().unwrap_or_else(|e| e.into_inner());
    let m = m.get_or_insert_with(HashMap::new);
    let mine: Vec<PathBuf> = m.keys().filter(|k| k.starts_with(out)).cloned().collect();
    let mut v: Vec<(String, usize)> = mine
        .into_iter()
        .map(|k| {
            let n = m.remove(&k).unwrap_or(0);
            (k.file_name().unwrap().to_string_lossy().into_owned(), n)
        })
        .collect();
    v.sort();
    v
}

#[test]
fn each_index_is_opened_once_per_step_of_the_alignment() {
    let (dir, files) = fixture("opens");
    let out = dir.join("out");
    let run = || {
        let o = flow_run(&[
            p(&dir.join("plan.json")),
            "--in",
            p(&dir.join("data")),
            "--out",
            p(&out),
        ]);
        assert_eq!(o.code, 0, "{}", o.stderr);
    };
    run();
    let idx = |n: usize| -> Vec<(String, usize)> {
        ["idx-s_t000.mzidx", "idx-s_t010.mzidx", "idx-s_t030.mzidx"]
            .iter()
            .map(|f| (f.to_string(), n))
            .collect()
    };
    // `features.align` and `features.gapfill`: one open each (the detection reads its index through its own gated open)
    assert_eq!(take(&out), idx(2));
    let page = page_table(&files, "").0;
    assert_eq!(std::fs::read(out.join("gap.mzfeat")).unwrap(), page);

    // both tables reused: the memory of each step needs only the recipe, from the first index of the series
    run();
    assert_eq!(take(&out), vec![("idx-s_t000.mzidx".to_string(), 2)]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_changed_index_is_opened_again_not_copied() {
    let (dir, _) = fixture("opens_changed");
    let out = dir.join("out");
    let o = flow_run(&[
        p(&dir.join("plan.json")),
        "--in",
        p(&dir.join("data")),
        "--out",
        p(&out),
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let paths: Vec<PathBuf> = ["idx-s_t000.mzidx", "idx-s_t010.mzidx"]
        .iter()
        .map(|f| out.join(f))
        .collect();
    let ix = crate::flow_base::flow_indexes::Indexes::open(&paths).unwrap();
    let _ = take(&out);
    let copy = ix.get(1).unwrap();
    assert_eq!(take(&out), vec![], "an unchanged file is copied");
    // another index written over the file: the copy would hold the tables of the old one
    std::fs::copy(&paths[0], &paths[1]).unwrap();
    let again = ix.get(1).unwrap();
    assert_eq!(take(&out), vec![("idx-s_t010.mzidx".to_string(), 1)]);
    let name = |i: &mzlab_core::idxread::Index<std::fs::File>| format!("{:?}", i.source);
    assert_ne!(name(&copy), name(&again));
    assert_eq!(name(&again), name(&ix.get(0).unwrap()));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn past_the_cap_an_index_is_opened_on_demand() {
    use crate::flow_base::flow_indexes::{weight, Indexes};
    let (dir, _) = fixture("opens_cap");
    let out = dir.join("out");
    let o = flow_run(&[
        p(&dir.join("plan.json")),
        "--in",
        p(&dir.join("data")),
        "--out",
        p(&out),
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let names = ["idx-s_t000.mzidx", "idx-s_t010.mzidx", "idx-s_t030.mzidx"];
    let paths: Vec<PathBuf> = names.iter().map(|f| out.join(f)).collect();
    let mut ix = Indexes::open(&paths).unwrap();
    assert_eq!(ix.all().len(), 3);
    // room for the first index only: the other two are opened in full at each use, the first is copied
    ix.keep(weight(&ix.all()[0]) + weight(&ix.all()[1]) - 1);
    assert_eq!(ix.all().len(), 1);
    let _ = take(&out);
    let seen =
        |i: &mzlab_core::idxread::Index<std::fs::File>| (i.scans.len(), format!("{:?}", i.source));
    for (k, path) in paths.iter().enumerate() {
        let opened = crate::flow_base::open(path).unwrap();
        assert_eq!(seen(&ix.get(k).unwrap()), seen(&opened));
    }
    let n = |f: &str, c: usize| (f.to_string(), c);
    assert_eq!(
        take(&out),
        vec![n(names[0], 1), n(names[1], 2), n(names[2], 2)]
    );
    ix.keep(0);
    assert!(ix.all().is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}
