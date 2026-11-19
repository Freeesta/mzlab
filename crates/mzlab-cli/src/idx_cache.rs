//! The opt-in index cache (`--cache DIR`, `MZLAB_CACHE`): a `.mzidx` built once is kept under the SHA-256 of its file, the sample, the index format
//! and the engine version, and handed to the next call that asks the same question. Nothing is written where it was not asked. Native only: the site
//! keeps its indexes in OPFS.

use std::cell::RefCell;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use mzlab_core::budget::sha256_hex;
use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::Head;

/// The cache of the running command and the stderr lines it has to report (the outcome carries them: `index reused` is evidence, not a log).
#[derive(Clone)]
pub(crate) struct Ctx {
    dir: PathBuf,
    notes: Arc<Mutex<Vec<String>>>,
}

thread_local! {
    /// The context of the command running on this thread; the pools of `par.rs` pass it on to their workers.
    static CTX: RefCell<Option<Ctx>> = const { RefCell::new(None) };
}

pub(crate) fn set(ctx: Option<Ctx>) {
    CTX.with(|c| *c.borrow_mut() = ctx);
}

pub(crate) fn ctx() -> Option<Ctx> {
    CTX.with(|c| c.borrow().clone())
}

pub(crate) fn dir() -> Option<PathBuf> {
    ctx().map(|c| c.dir)
}

/// `f` for a thread of a pool: it runs there with the cache of the thread that called `carry`.
pub(crate) fn carry<T>(f: impl FnOnce() -> T) -> impl FnOnce() -> T {
    let c = ctx();
    move || {
        set(c);
        f()
    }
}

/// The index of (`path`, `label`) at `idx`, through the cache when there is one; a cache that cannot be used is reported and skipped.
pub(crate) fn through(path: &Path, label: &str, idx: &Path, budget: usize) -> Result<()> {
    let build = || {
        crate::flow_pace::build_index_with(path, label, idx, budget, &mut |_, _| {}, &None)
            .map(|_| ())
    };
    match dir() {
        Some(d) => make(&d, path, label, idx, false, build).map(|_| ()),
        None => build(),
    }
}

fn note(line: String) {
    match ctx() {
        Some(c) => c.notes.lock().unwrap_or_else(|e| e.into_inner()).push(line),
        None => eprintln!("{line}"),
    }
}

/// The lines noted since the cache was set, for the stderr of the outcome.
pub(crate) fn take_notes() -> String {
    ctx().map_or(String::new(), |c| {
        let mut n = c.notes.lock().unwrap_or_else(|e| e.into_inner());
        n.drain(..).map(|l| l + "\n").collect()
    })
}

/// Takes `--cache DIR` out of `args` and sets the cache of this thread: the flag, else `MZLAB_CACHE`, else none.
pub(crate) fn take_flag(args: &mut Vec<String>) -> std::result::Result<(), String> {
    let mut found = None;
    while let Some(i) = args.iter().position(|a| a == "--cache") {
        let v = args
            .get(i + 1)
            .filter(|v| !v.starts_with("--"))
            .ok_or("--cache needs a folder")?
            .clone();
        args.drain(i..i + 2);
        found = Some(PathBuf::from(v));
    }
    let env = std::env::var_os("MZLAB_CACHE")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from);
    set(found.or(env).map(|dir| Ctx {
        dir,
        notes: Arc::default(),
    }));
    Ok(())
}

/// What the cache did for one index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Entry {
    pub reused: bool,
    /// The entry's file name in the cache folder.
    pub key: String,
    pub sha256: String,
}

fn cache_error(what: &str, dir: &Path, e: impl std::fmt::Display) -> Error {
    Error::new("err.run.output").with("detail", format!("cache {}: {what}: {e}", dir.display()))
}

/// The SHA-256 of the dataset at `path`, always read from the file: an entry is only ever named by a hash that was taken now.
fn sha_of(path: &Path) -> Result<String> {
    crate::flow_set::digest_for(path, false).map(|(_, sha)| sha)
}

static TMP_N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// A scratch name in `dir` that no other thread or process uses.
fn tmp_in(dir: &Path) -> PathBuf {
    let k = TMP_N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    dir.join(format!(".tmp-{}-{k}", std::process::id()))
}

/// Removes the `.tmp-*` files a crashed call left (older than an hour: a live writer's file is younger).
fn sweep(dir: &Path) {
    let old = std::time::Duration::from_secs(3600);
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let stale = e.file_name().to_string_lossy().starts_with(".tmp-")
            && e.metadata()
                .and_then(|m| m.modified())
                .is_ok_and(|t| t.elapsed().is_ok_and(|age| age > old));
        if stale {
            let _ = std::fs::remove_file(e.path());
        }
    }
}

/// The checksum file of an entry: the SHA-256 of the entry's bytes, so a body damaged without changing its length is found before use.
fn sum_of(entry: &Path) -> PathBuf {
    entry.with_extension("mzidx.sum")
}

/// `idx` copied to `entry` with its checksum, each renamed into place whole so that no reader sees half of either.
fn keep(dir: &Path, idx: &Path, entry: &Path) -> std::io::Result<()> {
    sweep(dir);
    let (tmp, tmp_sum) = (tmp_in(dir), tmp_in(dir));
    let made = std::fs::copy(idx, &tmp)
        .and_then(|_| mzlab_core::budget::sha256_hex_read(File::open(&tmp)?))
        .and_then(|sum| std::fs::write(&tmp_sum, sum))
        .and_then(|_| std::fs::rename(&tmp_sum, sum_of(entry)))
        .and_then(|_| std::fs::rename(&tmp, entry));
    if made.is_err() {
        let _ = (std::fs::remove_file(&tmp), std::fs::remove_file(&tmp_sum));
    }
    made
}

/// Is `entry` a whole index of this program: its bytes are the ones its checksum file recorded and its header reads?
fn is_sound(entry: &Path) -> bool {
    let Ok(want) = std::fs::read_to_string(sum_of(entry)) else {
        return false;
    };
    let Ok(mut f) = File::open(entry) else {
        return false;
    };
    let Ok(size) = f.metadata().map(|m| m.len()) else {
        return false;
    };
    mzlab_core::budget::sha256_hex_read(&mut f).is_ok_and(|got| got == want.trim())
        && Head::read(&mut f, size).is_ok()
}

/// Puts the index of (`path`, `label`) at `idx`: the entry of `dir` when there is a sound one, else what `build` writes there, which is then kept.
/// With `strict` a cache that cannot be read or written is an error (exit 6); without it the index is built and kept nowhere, and stderr says so.
pub(crate) fn make(
    dir: &Path,
    path: &Path,
    label: &str,
    idx: &Path,
    strict: bool,
    build: impl FnOnce() -> Result<()>,
) -> Result<Entry> {
    let name = path
        .file_name()
        .map_or(String::new(), |n| n.to_string_lossy().into_owned());
    // a file that is not there is the caller's input error, whatever the cache folder is, and no folder is made for it
    std::fs::metadata(path).map_err(|e| {
        Error::new("err.flow.io").with("detail", format!("{}: {e}", path.display()))
    })?;
    let prepared = std::fs::create_dir_all(dir)
        .map_err(|e| cache_error("cannot create", dir, e))
        .and_then(|_| sha_of(path));
    let sha = match prepared {
        Ok(s) => s,
        Err(e) if strict => return Err(e),
        Err(e) => {
            note(format!(
                "warn.cache.unusable: {name}: {}; index built without the cache",
                crate::error_text::human_error(&e)
            ));
            build()?;
            return Ok(Entry {
                reused: false,
                key: String::new(),
                sha256: String::new(),
            });
        }
    };
    let key = format!(
        "{sha}-{}-f{}-e{}.mzidx",
        &sha256_hex(label.as_bytes())[..12],
        mzlab_core::idx_newer::VERSION_MAX,
        env!("CARGO_PKG_VERSION")
    );
    let entry = dir.join(&key);
    if entry.exists() {
        if is_sound(&entry) {
            let _ = std::fs::remove_file(idx);
            // a copy, never a link: whatever writes `idx` must not reach the entry
            if std::fs::copy(&entry, idx).is_ok() {
                note(format!("index reused: {name}"));
                return Ok(Entry {
                    reused: true,
                    key,
                    sha256: sha,
                });
            }
        }
        note(format!(
            "warn.cache.stale: {name}: entry {key} is damaged or cannot be read; index rebuilt"
        ));
    }
    build()?;
    let kept = keep(dir, idx, &entry).map_err(|e| cache_error("cannot keep the index", dir, e));
    match kept {
        Ok(()) => note(format!("index built: {name}")),
        // an entry that cannot be replaced (another process has it open) costs only this call's keeping
        Err(e) if strict && !entry.exists() => return Err(e),
        Err(e) => note(format!(
            "warn.cache.unusable: {name}: {}; index built, not kept",
            crate::error_text::human_error(&e)
        )),
    }
    Ok(Entry {
        reused: false,
        key,
        sha256: sha,
    })
}

#[cfg(test)]
#[path = "idx_cache_tests.rs"]
mod tests;
