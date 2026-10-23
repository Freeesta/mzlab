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

/// The recipe, the `.mzfeatf` bytes and the weak candidates of each file.
pub(crate) type Detected = (mzlab_feat::recipe::Recipe, Vec<Vec<u8>>, Vec<Vec<u8>>);

/// Index and detection of each file are independent: the pool takes the indexes largest first and starts a detection as soon as its index and the
/// recipe (made from the first index) exist, so no thread waits for the slowest index of the series. Results are in file order. Returns the
/// recipe, the `.mzfeatf` bytes and the weak candidates of each file.
pub(crate) fn index_and_detect(
    files: &[PathBuf],
    idxs: &[PathBuf],
    params: &str,
) -> Result<Detected> {
    #[cfg(not(target_family = "wasm"))]
    if let Some(r) = pooled(files, idxs, params) {
        return r;
    }
    par_map(files.len(), |k| index_one(files, idxs, k))
        .into_iter()
        .collect::<Result<Vec<()>>>()?;
    let recipe = run::recipe_for(params, &mut open(&idxs[0])?)?;
    let detected = par_map(files.len(), |k| detect_one(files, idxs, k, &recipe))
        .into_iter()
        .collect::<Result<Vec<_>>>()?;
    let (peaks, weak) = detected.into_iter().unzip();
    mzlab_feat::prof::report("features");
    Ok((recipe, peaks, weak))
}

fn index_one(files: &[PathBuf], idxs: &[PathBuf], k: usize) -> Result<()> {
    eprintln!("index {}", files[k].display());
    mzlab_feat::prof::time("index", || build_index(&files[k], &idxs[k], DEFAULT_BUDGET))
}

fn detect_one(
    files: &[PathBuf],
    idxs: &[PathBuf],
    k: usize,
    recipe: &mzlab_feat::recipe::Recipe,
) -> Result<(Vec<u8>, Vec<u8>)> {
    eprintln!("detect {}", files[k].display());
    let name = files[k]
        .file_name()
        .map_or(String::new(), |n| n.to_string_lossy().into_owned());
    let mut index = open(&idxs[k])?;
    let peaks = run::detect(&mut index, &name, recipe)?;
    // The weak candidates are made here, beside the file, so the alignment does not detect it again (the Flow does the same).
    let weak = if recipe.weak_files > 0 {
        run::detect_weak(&mut index, recipe)?
    } else {
        Vec::new()
    };
    Ok((peaks, weak))
}

#[cfg(not(target_family = "wasm"))]
enum Job {
    Index(usize),
    Detect(usize),
}

/// What the threads of `pooled` share: the next job, the finished indexes, the recipe and the detections.
#[cfg(not(target_family = "wasm"))]
struct Pool {
    next: usize,
    indexed: Vec<Option<Result<()>>>,
    /// 0 = not yet, 1 = made, 2 = the first index or the recipe failed (the error is in `indexed[0]` or `recipe_err`).
    recipe_state: u8,
    recipe: Option<std::sync::Arc<mzlab_feat::recipe::Recipe>>,
    recipe_err: Option<mzlab_core::error::Error>,
    detected: Vec<Option<Result<(Vec<u8>, Vec<u8>)>>>,
}

/// Stores the recipe made from the first index (or why there is none) and lets the detections start.
#[cfg(not(target_family = "wasm"))]
fn set_recipe(st: &mut Pool, made: Option<Result<mzlab_feat::recipe::Recipe>>) {
    match made {
        Some(Ok(r)) => {
            st.recipe = Some(std::sync::Arc::new(r));
            st.recipe_state = 1;
        }
        Some(Err(e)) => {
            st.recipe_err = Some(e);
            st.recipe_state = 2;
        }
        None => st.recipe_state = 2,
    }
}

/// The first file first (the recipe waits for it), the others and then the detections largest first.
#[cfg(not(target_family = "wasm"))]
fn plan_jobs(files: &[PathBuf]) -> Vec<Job> {
    let size = |k: usize| std::fs::metadata(&files[k]).map_or(0, |m| m.len());
    let mut rest: Vec<usize> = (1..files.len()).collect();
    rest.sort_by_key(|&k| std::cmp::Reverse(size(k)));
    let mut jobs = vec![Job::Index(0)];
    jobs.extend(rest.iter().map(|&k| Job::Index(k)));
    let mut all: Vec<usize> = (0..files.len()).collect();
    all.sort_by_key(|&k| std::cmp::Reverse(size(k)));
    jobs.extend(all.iter().map(|&k| Job::Detect(k)));
    jobs
}

/// The result of a finished pool: the first error in file order, else the recipe and the detections.
#[cfg(not(target_family = "wasm"))]
fn collect(mut st: Pool) -> Result<Detected> {
    for r in st.indexed.iter_mut() {
        if let Some(Err(e)) = r.take() {
            return Err(e);
        }
    }
    if let Some(e) = st.recipe_err.take() {
        return Err(e);
    }
    let Some(recipe) = st.recipe.take() else {
        return Err(mzlab_core::error::Error::memory_budget());
    };
    let (mut peaks, mut weak) = (Vec::new(), Vec::new());
    for r in st.detected.iter_mut() {
        let (p, w) = r
            .take()
            .unwrap_or_else(|| Err(mzlab_core::error::Error::memory_budget()))?;
        peaks.push(p);
        weak.push(w);
    }
    let recipe = std::sync::Arc::try_unwrap(recipe).unwrap_or_else(|a| (*a).clone());
    Ok((recipe, peaks, weak))
}

/// `index_and_detect` on a pool of threads; `None` when there is only one thread or one file (then the plain passes run).
#[cfg(not(target_family = "wasm"))]
fn pooled(files: &[PathBuf], idxs: &[PathBuf], params: &str) -> Option<Result<Detected>> {
    use std::sync::{Condvar, Mutex};
    let n = files.len();
    let workers = std::thread::available_parallelism().map_or(1, |w| w.get());
    if workers < 2 || n < 2 {
        return None;
    }
    let jobs = plan_jobs(files);
    let pool = Mutex::new(Pool {
        next: 0,
        indexed: (0..n).map(|_| None).collect(),
        recipe_state: 0,
        recipe: None,
        recipe_err: None,
        detected: (0..n).map(|_| None).collect(),
    });
    let ready = Condvar::new();
    let lock = || pool.lock().unwrap_or_else(|e| e.into_inner());
    std::thread::scope(|s| {
        for _ in 0..workers.min(jobs.len()) {
            s.spawn(|| loop {
                let j = {
                    let mut st = lock();
                    let j = st.next;
                    st.next += 1;
                    j
                };
                let Some(job) = jobs.get(j) else { break };
                match *job {
                    Job::Index(k) => {
                        let r = index_one(files, idxs, k);
                        let ok = r.is_ok();
                        lock().indexed[k] = Some(r);
                        if k == 0 {
                            let made = ok.then(|| {
                                open(&idxs[0]).and_then(|mut ix| run::recipe_for(params, &mut ix))
                            });
                            set_recipe(&mut lock(), made);
                        }
                        ready.notify_all();
                    }
                    Job::Detect(k) => {
                        let mut st = lock();
                        while st.recipe_state == 0 || st.indexed[k].is_none() {
                            st = ready.wait(st).unwrap_or_else(|e| e.into_inner());
                        }
                        let recipe = st.recipe.clone();
                        let indexed = matches!(st.indexed[k], Some(Ok(())));
                        drop(st);
                        if let (Some(recipe), true) = (recipe, indexed) {
                            let r = detect_one(files, idxs, k, &recipe);
                            lock().detected[k] = Some(r);
                        }
                    }
                }
            });
        }
    });
    let out = collect(pool.into_inner().unwrap_or_else(|e| e.into_inner()));
    mzlab_feat::prof::report("features");
    Some(out)
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
