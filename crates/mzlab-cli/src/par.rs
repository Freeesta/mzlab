//! `par_map`: the independent per-file passes of `features` on a pool of threads and the index + detect stage of `features` built on it.

use crate::{build_index, open, DEFAULT_BUDGET};
use mzlab_core::error::Result;
use mzlab_feat_wasm::run;
use std::path::PathBuf;

/// `f(0..n)` on up to one thread per core (the order of the results is the order of the indices); in sequence on WASI, which has no threads.
fn par_map<T: Send>(n: usize, f: impl Fn(usize) -> T + Sync) -> Vec<T> {
    #[cfg(not(target_family = "wasm"))]
    {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Mutex;
        let workers = std::thread::available_parallelism().map_or(1, |w| w.get());
        if workers > 1 && n > 1 {
            let next = AtomicUsize::new(0);
            let slots: Mutex<Vec<Option<T>>> = Mutex::new((0..n).map(|_| None).collect());
            std::thread::scope(|s| {
                for _ in 0..workers.min(n) {
                    s.spawn(|| loop {
                        let k = next.fetch_add(1, Ordering::Relaxed);
                        if k >= n {
                            break;
                        }
                        let r = f(k);
                        slots.lock().unwrap_or_else(|e| e.into_inner())[k] = Some(r);
                    });
                }
            });
            let slots = slots.into_inner().unwrap_or_else(|e| e.into_inner());
            return slots.into_iter().flatten().collect();
        }
    }
    (0..n).map(f).collect()
}

/// Index and detection of each file are independent: one pass per file on the pool, results in file order. Returns the recipe, the `.mzfeatf` bytes and the weak candidates of each file.
pub(crate) fn index_and_detect(
    files: &[PathBuf],
    idxs: &[PathBuf],
    params: &str,
) -> Result<(mzlab_feat::recipe::Recipe, Vec<Vec<u8>>, Vec<Vec<u8>>)> {
    par_map(files.len(), |k| {
        eprintln!("index {}", files[k].display());
        build_index(&files[k], &idxs[k], DEFAULT_BUDGET)
    })
    .into_iter()
    .collect::<Result<Vec<()>>>()?;
    let recipe = run::recipe_for(params, &mut open(&idxs[0])?)?;
    let detected = par_map(files.len(), |k| {
        eprintln!("detect {}", files[k].display());
        let name = files[k]
            .file_name()
            .map_or(String::new(), |n| n.to_string_lossy().into_owned());
        let mut index = open(&idxs[k])?;
        let peaks = run::detect(&mut index, &name, &recipe)?;
        // The weak candidates are made here, beside the file, so the alignment does not detect it again (the Flow does the same).
        let weak = if recipe.weak_files > 0 {
            run::detect_weak(&mut index, &recipe)?
        } else {
            Vec::new()
        };
        Ok((peaks, weak))
    })
    .into_iter()
    .collect::<Result<Vec<_>>>()?;
    let (peaks, weak) = detected.into_iter().unzip();
    Ok((recipe, peaks, weak))
}

/// The detections of a series in memory, with their weak candidates, as the source of the alignment.
pub(crate) struct Series(pub(crate) Vec<Vec<u8>>, pub(crate) Vec<Vec<u8>>);

impl mzlab_feat::slabs::Stored for Series {
    fn count(&self) -> usize {
        self.0.len()
    }
    fn bytes(&mut self, k: usize) -> Result<Vec<u8>> {
        Ok(self.0[k].clone())
    }
    fn weak(&mut self, k: usize) -> Result<Option<Vec<u8>>> {
        Ok(Some(self.1[k].clone()).filter(|w| !w.is_empty()))
    }
}
