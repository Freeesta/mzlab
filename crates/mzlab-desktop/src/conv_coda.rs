//! The background conversion queue of the desktop app: each maker's file the page holds is converted to indexed mzML in the cache folder by the very
//! entry of `mzlab-cli convert` (same bytes), on `cores - 1` threads and never while an index of Data is being built. The page reads it with
//! `conv_list`, stops it with `conv_cancel`, saves the ready files with `conv_save`; `flow/run` takes a ready mzML instead of converting again.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use mzlab_core::Result;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::engine::State;
use crate::rpc::{io_error, text, Answer};

/// How many index builds of Data are running: the queue waits for them to end.
static INDEXING: AtomicUsize = AtomicUsize::new(0);

/// Held while an index is built (`IdxBuilder::scrivi`): the queue leaves the cores to it.
pub(crate) struct Indexing;

impl Indexing {
    pub(crate) fn new() -> Indexing {
        INDEXING.fetch_add(1, Ordering::SeqCst);
        Indexing
    }
}

impl Drop for Indexing {
    fn drop(&mut self) {
        INDEXING.fetch_sub(1, Ordering::SeqCst);
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Phase {
    Queued,
    Running,
    Done,
    Failed(String),
    Cancelled,
}

/// One dataset (a `.raw`, a `.wiff` with its scan file, a `.D` folder) and the page files (samples) that come from it.
struct Job {
    root: PathBuf,
    out: PathBuf,
    /// size and date of the dataset when it was queued: a ready mzML is only good for the same bytes
    stamp: Option<(u64, u64)>,
    names: Vec<(String, u32)>,
    phase: Phase,
    files: Vec<PathBuf>,
}

#[derive(Default)]
struct Inner {
    jobs: Vec<Job>,
    threads: usize,
}

#[derive(Default)]
pub(crate) struct Coda {
    inner: Arc<Mutex<Inner>>,
    folder: Mutex<Option<PathBuf>>,
    /// threads at most; 0 = cores - 1
    workers: usize,
}

fn lock(m: &Mutex<Inner>) -> std::sync::MutexGuard<'_, Inner> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn stamp(p: &Path) -> Option<(u64, u64)> {
    let m = std::fs::metadata(p).ok()?;
    let t = m
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_millis() as u64;
    Some((m.len(), t))
}

/// The dataset a sample belongs to: the first member that is not a `.wiff.scan`, taken back up to the folder of a `.D`.
pub(crate) fn dataset_root(members: &[(String, PathBuf)]) -> Option<PathBuf> {
    let (name, path) = members
        .iter()
        .find(|(n, _)| !n.to_lowercase().ends_with(".wiff.scan"))
        .or_else(|| members.first())?;
    let up = name
        .split(['/', '\\'])
        .filter(|s| !s.is_empty())
        .count()
        .saturating_sub(1);
    let mut root = path.clone();
    for _ in 0..up {
        root = root.parent()?.to_path_buf();
    }
    Some(root)
}

/// What `mzlab-cli convert` does with one dataset: its mzML files in `out` (the cache of its folder makes a second run free).
fn convert(root: &Path, out: &Path) -> std::result::Result<Vec<PathBuf>, String> {
    let args = vec![
        root.to_string_lossy().into_owned(),
        "--out".into(),
        out.to_string_lossy().into_owned(),
        "--workers".into(),
        "1".into(),
    ];
    let o = mzlab_cli::convert::run(args);
    if o.code != 0 {
        // the line of the failed file: «[1/1] name ... failed: CAUSE»
        let why: String = o
            .stderr
            .lines()
            .find_map(|l| l.split_once("... failed: ").map(|p| p.1))
            .or_else(|| o.stderr.lines().next())
            .unwrap_or("conversion failed")
            .chars()
            .take(600)
            .collect();
        return Err(why);
    }
    let mut files: Vec<PathBuf> = std::fs::read_dir(out)
        .map_err(|e| e.to_string())?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "mzML"))
        .collect();
    files.sort();
    Ok(files)
}

fn work(inner: Arc<Mutex<Inner>>) {
    loop {
        while INDEXING.load(Ordering::SeqCst) > 0 {
            std::thread::sleep(Duration::from_millis(100));
        }
        let (k, root, out) = {
            let mut g = lock(&inner);
            let Some(k) = g.jobs.iter().position(|j| j.phase == Phase::Queued) else {
                g.threads -= 1;
                return;
            };
            g.jobs[k].phase = Phase::Running;
            (k, g.jobs[k].root.clone(), g.jobs[k].out.clone())
        };
        let r = std::panic::catch_unwind(|| convert(&root, &out))
            .unwrap_or_else(|_| Err("the conversion stopped unexpectedly".into()));
        let mut g = lock(&inner);
        match r {
            Ok(files) => (g.jobs[k].phase, g.jobs[k].files) = (Phase::Done, files),
            Err(e) => g.jobs[k].phase = Phase::Failed(e),
        }
    }
}

impl Coda {
    #[cfg(test)]
    pub(crate) fn with_workers(n: usize) -> Coda {
        Coda {
            workers: n,
            ..Coda::default()
        }
    }

    fn limit(&self) -> usize {
        if self.workers > 0 {
            self.workers
        } else {
            std::thread::available_parallelism()
                .map_or(1, |n| n.get())
                .saturating_sub(1)
                .max(1)
        }
    }

    pub(crate) fn set_folder(&self, dir: PathBuf) {
        *self.folder.lock().unwrap_or_else(|e| e.into_inner()) = Some(dir);
    }

    fn base(&self) -> PathBuf {
        self.folder
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
            .unwrap_or_else(|| std::env::temp_dir().join("mzlab-mzml"))
    }

    /// Queues the dataset `root` for the page file `name` (sample `id`); a dataset already queued, running or ready only learns the name again.
    pub(crate) fn add(&self, name: &str, id: u32, root: &Path) {
        let key = Sha256::digest(root.to_string_lossy().as_bytes());
        let out = self.base().join(
            key[..8]
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>(),
        );
        let st = stamp(root);
        let spawn = {
            let mut g = lock(&self.inner);
            let again =
                |j: &Job| j.root == root && j.stamp == st && !matches!(j.phase, Phase::Cancelled);
            match g.jobs.iter().position(again) {
                Some(k) => {
                    if !g.jobs[k].names.iter().any(|(n, _)| n == name) {
                        g.jobs[k].names.push((name.to_string(), id));
                    }
                    false
                }
                None => {
                    g.jobs.retain(|j| j.root != root);
                    g.jobs.push(Job {
                        root: root.to_path_buf(),
                        out,
                        stamp: st,
                        names: vec![(name.to_string(), id)],
                        phase: Phase::Queued,
                        files: Vec::new(),
                    });
                    g.threads < self.limit() && {
                        g.threads += 1;
                        true
                    }
                }
            }
        };
        if spawn {
            let inner = self.inner.clone();
            std::thread::spawn(move || work(inner));
        }
    }

    /// Stops what has not started (one page file's dataset with `name`, else everything queued); the running conversion ends and keeps its files.
    pub(crate) fn cancel(&self, name: Option<&str>) {
        for j in lock(&self.inner).jobs.iter_mut().filter(|j| {
            j.phase == Phase::Queued && name.is_none_or(|n| j.names.iter().any(|(m, _)| m == n))
        }) {
            j.phase = Phase::Cancelled;
        }
    }

    fn file_of(job: &Job, id: u32) -> Option<PathBuf> {
        match job.files.as_slice() {
            [one] => Some(one.clone()),
            many => many
                .iter()
                .find(|f| {
                    f.file_name()
                        .is_some_and(|n| n.to_string_lossy().ends_with(&format!("-{id}.mzML")))
                })
                .cloned(),
        }
    }

    /// The ready mzML of the page file `name`, if its dataset is unchanged since it was converted.
    pub(crate) fn ready(&self, name: &str) -> Option<PathBuf> {
        let g = lock(&self.inner);
        let (j, id) = g.jobs.iter().find_map(|j| {
            j.names
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, id)| (j, *id))
        })?;
        (j.phase == Phase::Done && stamp(&j.root) == j.stamp)
            .then(|| Self::file_of(j, id))
            .flatten()
            .filter(|f| f.is_file())
    }

    pub(crate) fn list(&self) -> Value {
        let g = lock(&self.inner);
        let mut rows = Vec::new();
        for j in &g.jobs {
            for (name, id) in &j.names {
                let (state, error) = match &j.phase {
                    Phase::Queued => ("queued", String::new()),
                    Phase::Running => ("run", String::new()),
                    Phase::Done if Self::file_of(j, *id).is_some() => ("done", String::new()),
                    Phase::Done => ("failed", "no mzML for this sample".into()),
                    Phase::Failed(e) => ("failed", e.clone()),
                    Phase::Cancelled => ("cancelled", String::new()),
                };
                rows.push(json!({ "name": name, "state": state, "error": error }));
            }
        }
        json!({ "jobs": rows })
    }

    /// Puts every ready mzML in the folder `dest` (a hard link where the system allows, else a copy); how many.
    pub(crate) fn save(&self, dest: &Path) -> Result<usize> {
        std::fs::create_dir_all(dest).map_err(|e| io_error(&e))?;
        let files: Vec<PathBuf> = lock(&self.inner)
            .jobs
            .iter()
            .filter(|j| j.phase == Phase::Done)
            .flat_map(|j| j.files.clone())
            .collect();
        let mut n = 0;
        for f in files {
            let Some(name) = f.file_name() else { continue };
            let to = dest.join(name);
            let _ = std::fs::remove_file(&to);
            std::fs::hard_link(&f, &to)
                .or_else(|_| std::fs::copy(&f, &to).map(|_| ()))
                .map_err(|e| io_error(&e))?;
            n += 1;
        }
        Ok(n)
    }
}

impl State {
    pub fn set_convert_folder(&self, dir: PathBuf) {
        self.coda.set_folder(dir);
    }

    /// `conv_add {name}`: queues the maker's file `name` of the list (nothing for a file that is already an mzML).
    pub(crate) fn conv_add(&self, p: &Value) -> Result<Answer> {
        let name = p["name"].as_str().unwrap_or("");
        let nat = self
            .store
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(name)
            .and_then(|e| e.nat);
        if let Some(nat) = nat {
            if let Some(root) = dataset_root(&nat.members) {
                self.coda.add(name, nat.id, &root);
            }
        }
        text(self.coda.list().to_string())
    }

    /// `conv_list {}`: `{"jobs": [{name, state: queued|run|done|failed|cancelled, error}]}`.
    pub(crate) fn conv_list(&self, _p: &Value) -> Result<Answer> {
        text(self.coda.list().to_string())
    }

    /// `conv_cancel {name?}`: stops the files that have not started.
    pub(crate) fn conv_cancel(&self, p: &Value) -> Result<Answer> {
        self.coda.cancel(p["name"].as_str());
        text(self.coda.list().to_string())
    }

    /// `conv_save {dest}`: the ready mzML files in the folder `dest`; `{"saved": n}`.
    pub fn conv_save(&self, dest: &Path) -> Result<usize> {
        self.coda.save(dest)
    }

    /// The ready mzML of the page file `name`, for `flow/run`.
    pub(crate) fn conv_ready(&self, name: &str) -> Option<PathBuf> {
        self.coda.ready(name)
    }
}
