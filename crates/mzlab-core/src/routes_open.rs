//! Opening one more index beside the files the engine already holds (site engine `open_index`, desktop `open_lr`: the same function).
//! Every open file keeps its scan table (about 1 KB per DDA scan) outside the budget of the cache: when the memory runs out, the cache of the
//! other files goes first (read again from their indexes, same answers), then the refusal names the files held, so the page says «close some files»
//! and never «the file is too big» or «damaged index».

use std::collections::HashMap;

use crate::error::{Error, Result};
use crate::idxread::ReadAt;
use crate::routes::{trim_cache, Routes};

/// A reservation that failed (`error.memoryBudget` without a phase): the browser or the computer gave no more memory.
fn failed_reservation(e: &Error) -> bool {
    e.key == "error.memoryBudget" && !e.params.iter().any(|(k, _)| *k == "fase")
}

/// `open()` once; after a failed reservation the cache of `files` is emptied and `open()` runs once more. A second failure is `error.memoryBudget`
/// with `fase` `openFiles`, `aperti` (files held) and `scansioni` (their scans); any other error passes as it is.
pub fn open_beside<R: ReadAt, T>(
    files: &mut HashMap<u32, Routes<R>>,
    mut open: impl FnMut() -> Result<T>,
) -> Result<T> {
    match open() {
        Err(e) if failed_reservation(&e) => {}
        done => return done,
    }
    trim_cache(files, 0);
    open().map_err(|e| {
        if !failed_reservation(&e) {
            return e;
        }
        // the slots of one file share its scan table (`Index::over`): counted once, so `aperti` is files, not slots
        let mut tables: Vec<&std::sync::Arc<Vec<crate::idxread::Scan>>> = Vec::new();
        for r in files.values() {
            if !tables
                .iter()
                .any(|t| std::sync::Arc::ptr_eq(t, &r.idx.scans))
            {
                tables.push(&r.idx.scans);
            }
        }
        let scans: usize = tables.iter().map(|t| t.len()).sum();
        let held = tables.len();
        Error::memory_budget()
            .with("fase", "openFiles")
            .with("aperti", held)
            .with("scansioni", scans)
    })
}

#[cfg(test)]
mod tests {
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
}
