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

use crate::conv_stamp::stamp;
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
    /// never reused: a running worker finds its job by this, not by a position that other changes shift
    id: u64,
    root: PathBuf,
    out: PathBuf,
    /// size and date of all the files of the dataset when it was queued: a ready mzML is only good for the same bytes
    stamp: Option<(u64, u64)>,
    names: Vec<(String, u32)>,
    phase: Phase,
    files: Vec<PathBuf>,
}

#[derive(Default)]
pub(crate) struct Inner {
    jobs: Vec<Job>,
    threads: usize,
    next_id: u64,
}

/// A job a worker has taken: what it needs to convert and to report back.
struct Started {
    id: u64,
    root: PathBuf,
    out: PathBuf,
    stamp: Option<(u64, u64)>,
}

#[cfg(test)]
impl Inner {
    pub(crate) fn take_for_test(&mut self) -> u64 {
        self.take().expect("a queued job").id
    }

    pub(crate) fn can_take_for_test(&self) -> bool {
        self.jobs.iter().any(|j| {
            j.phase == Phase::Queued
                && !self
                    .jobs
                    .iter()
                    .any(|r| r.phase == Phase::Running && r.root == j.root)
        })
    }

    pub(crate) fn finish_for_test(
        &mut self,
        id: u64,
        r: std::result::Result<Vec<PathBuf>, String>,
    ) {
        self.finish(id, r);
    }
}

impl Inner {
    /// Marks the first queued job as running; a dataset is never converted by two workers at once (they would share one output folder).
    fn take(&mut self) -> Option<Started> {
        let k = self.jobs.iter().position(|j| {
            j.phase == Phase::Queued
                && !self
                    .jobs
                    .iter()
                    .any(|r| r.phase == Phase::Running && r.root == j.root)
        })?;
        let j = &mut self.jobs[k];
        j.phase = Phase::Running;
        Some(Started {
            id: j.id,
            root: j.root.clone(),
            out: j.out.clone(),
            stamp: j.stamp,
        })
    }

    /// Records how job `id` ended; a job that was replaced meanwhile (its dataset changed) is dropped with its result.
    fn finish(&mut self, id: u64, r: std::result::Result<Vec<PathBuf>, String>) {
        let Some(k) = self.jobs.iter().position(|j| j.id == id) else {
            return;
        };
        if self.jobs[k].names.is_empty() {
            self.jobs.remove(k);
            return;
        }
        match r {
            Ok(files) => (self.jobs[k].phase, self.jobs[k].files) = (Phase::Done, files),
            Err(e) => self.jobs[k].phase = Phase::Failed(e),
        }
    }
}

#[derive(Default)]
pub(crate) struct Coda {
    pub(crate) inner: Arc<Mutex<Inner>>,
    folder: Mutex<Option<PathBuf>>,
    /// threads at most; 0 = cores - 1
    workers: usize,
    /// the re-check pause of `wait_for_indexing` in ms; 0 = `RECHECK_MS`
    recheck_ms: u64,
}

impl Coda {
    #[cfg(test)]
    pub(crate) fn with_workers(n: usize) -> Coda {
        Coda {
            workers: n,
            ..Coda::default()
        }
    }

    /// A one-worker queue whose job waits `ms` before it re-checks for an index that started after it was queued.
    #[cfg(test)]
    pub(crate) fn with_recheck(ms: u64) -> Coda {
        Coda {
            workers: 1,
            recheck_ms: ms,
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
}

fn lock(m: &Mutex<Inner>) -> std::sync::MutexGuard<'_, Inner> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl Coda {
    /// Queues the dataset `root` for the page file `name` (sample `id`); a dataset already queued, running or ready only learns the name again.
    pub(crate) fn add(&self, name: &str, id: u32, root: &Path) {
        if self.enqueue(name, id, root, stamp(root)) {
            let inner = self.inner.clone();
            let recheck = Duration::from_millis(if self.recheck_ms > 0 {
                self.recheck_ms
            } else {
                RECHECK_MS
            });
            std::thread::spawn(move || work(inner, recheck));
        }
    }

    /// The queue part of `add`; true when a new worker thread is due.
    pub(crate) fn enqueue(&self, name: &str, id: u32, root: &Path, st: Option<(u64, u64)>) -> bool {
        let key = Sha256::digest(root.to_string_lossy().as_bytes());
        let out = self.base().join(
            key[..8]
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>(),
        );
        {
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
                    // a running conversion of the old bytes stays (its worker holds the output folder) but loses its names:
                    // it is dropped when it ends and the new job waits for it
                    g.jobs
                        .retain(|j| j.root != root || j.phase == Phase::Running);
                    for j in g.jobs.iter_mut().filter(|j| j.root == root) {
                        j.names.clear();
                    }
                    g.next_id += 1;
                    let job_id = g.next_id;
                    g.jobs.push(Job {
                        id: job_id,
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
        if let Some(nat) = nat
            && let Some(root) = dataset_root(&nat.members)
        {
            self.coda.add(name, nat.id, &root);
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

/// Default pause of the re-check in `wait_for_indexing`.
const RECHECK_MS: u64 = 50;

/// Holds back until no index of Data is being built; the short re-check catches an index that starts just after the job was queued.
fn wait_for_indexing(recheck: Duration) {
    loop {
        while INDEXING.load(Ordering::SeqCst) > 0 {
            std::thread::sleep(Duration::from_millis(100));
        }
        std::thread::sleep(recheck);
        if INDEXING.load(Ordering::SeqCst) == 0 {
            return;
        }
    }
}

/// The marker in the output folder that says which bytes its mzML files come from.
fn marker(out: &Path) -> PathBuf {
    out.join(".stamp")
}

/// `convert` for a dataset whose output folder may hold files of other bytes: they are removed only when the dataset changed,
/// so an unchanged one keeps the cache that makes a second run free.
pub(crate) fn convert_fresh(
    root: &Path,
    out: &Path,
    stamp: Option<(u64, u64)>,
) -> std::result::Result<Vec<PathBuf>, String> {
    let now = stamp.map(|(a, b)| format!("{a} {b}"));
    let same = now.is_some() && std::fs::read_to_string(marker(out)).ok() == now;
    if !same {
        for e in std::fs::read_dir(out).into_iter().flatten().flatten() {
            if e.path().extension().is_some_and(|x| x == "mzML") {
                let _ = std::fs::remove_file(e.path());
            }
        }
        let _ = std::fs::remove_file(marker(out));
    }
    let files = convert(root, out)?;
    if let Some(now) = now {
        let _ = std::fs::write(marker(out), now);
    }
    Ok(files)
}

fn work(inner: Arc<Mutex<Inner>>, recheck: Duration) {
    loop {
        wait_for_indexing(recheck);
        let s = {
            let mut g = lock(&inner);
            let Some(s) = g.take() else {
                g.threads -= 1;
                return;
            };
            s
        };
        let r = std::panic::catch_unwind(|| convert_fresh(&s.root, &s.out, s.stamp))
            .unwrap_or_else(|_| Err("the conversion stopped unexpectedly".into()));
        lock(&inner).finish(s.id, r);
    }
}

/// The file of a finished job that holds sample `id`.
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

impl Coda {
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
            .then(|| file_of(j, id))
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
                    Phase::Done if file_of(j, *id).is_some() => ("done", String::new()),
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
