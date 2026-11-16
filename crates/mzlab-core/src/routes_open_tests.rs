//! Tests of `routes_open`: the retry after the cache is emptied and the refusal that names the files held.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use super::*;
use crate::idxread::Index;

/// One small index held open, as the engine holds it.
fn held() -> HashMap<u32, Routes<Vec<u8>>> {
    use crate::mzml::Spectrum;
    use crate::mzmlwrite::{Header, MzmlWriter};
    let mut w = MzmlWriter::new(
        Vec::new(),
        &Header {
            run: "r".into(),
            ..Default::default()
        },
    )
    .unwrap();
    w.open_spectra(3).unwrap();
    for i in 0..3 {
        let s = Spectrum {
            index: i,
            id: format!("scan={}", i + 1),
            level: 1,
            rt: i as f64 * 0.1,
            polarity: 1,
            mz: vec![100.0, 150.0],
            intensity: vec![1.0, 2.0],
            ..Default::default()
        };
        w.spectrum(&s).unwrap();
    }
    w.close_spectra().unwrap();
    let b = w.finish().unwrap();
    let mut src = b.clone();
    let p = crate::upload::prepare(&mut src, b.len() as u64, "r.mzML").unwrap();
    let idx = crate::mzidx::build_mzml_to_vec(&p.table, &b, 64 << 20)
        .unwrap()
        .0;
    let n = idx.len() as u64;
    HashMap::from([(1, Routes::new(Index::open(idx, n).unwrap()))])
}

#[test]
fn second_try_after_the_cache_is_emptied() {
    let mut files = held();
    let mut calls = 0;
    let r = open_beside(&mut files, || {
        calls += 1;
        if calls == 1 {
            Err(Error::memory_budget())
        } else {
            Ok(7)
        }
    });
    assert_eq!((r.unwrap(), calls), (7, 2));
    assert_eq!(crate::routes::cache_bytes(&files), 0);
}

#[test]
fn refusal_names_the_files_held() {
    let mut files = held();
    let scans = files[&1].idx.scans.len();
    let e = open_beside(&mut files, || Err::<(), _>(Error::memory_budget())).unwrap_err();
    assert_eq!(e.to_json(), format!("{{\"error_key\":\"error.memoryBudget\",\"params\":{{\"fase\":\"openFiles\",\"aperti\":\"1\",\"scansioni\":\"{scans}\"}}}}"));
}

#[test]
fn slots_sharing_one_table_count_as_one_file() {
    let mut files = held();
    let shared = Routes::new(files[&1].idx.over(Vec::new()));
    files.insert(2, shared);
    let scans = files[&1].idx.scans.len();
    let e = open_beside(&mut files, || Err::<(), _>(Error::memory_budget())).unwrap_err();
    assert!(
        e.to_json()
            .contains(&format!("\"aperti\":\"1\",\"scansioni\":\"{scans}\"")),
        "{}",
        e.to_json()
    );
}

#[test]
fn other_errors_and_planned_refusals_pass_unchanged() {
    let mut files = held();
    let mut calls = 0;
    let planned = crate::budget::refusal("index", 1 << 30, 1 << 20);
    let e = open_beside(&mut files, || {
        calls += 1;
        Err::<(), _>(planned.clone())
    })
    .unwrap_err();
    assert_eq!((e, calls), (planned, 1));
    assert_eq!(
        open_beside(&mut files, || Err::<(), _>(Error::new("error.badIndex")))
            .unwrap_err()
            .key,
        "error.badIndex"
    );
}
