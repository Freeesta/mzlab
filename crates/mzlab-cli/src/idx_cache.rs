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

/// The SHA-256 of the dataset at `path`. A stamp (path, member names, sizes, modification times) found in `dir` gives it without reading the
/// file; any change of those makes a new stamp and the file is read again, so an entry is only ever named by a hash that was taken.
fn sha_of(dir: &Path, path: &Path) -> Result<String> {
    let members = crate::flow_set::members(path)?;
    let mut text = std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .display()
        .to_string();
    for (name, p) in &members {
        let m = std::fs::metadata(p).map_err(crate::io)?;
        let ns = m
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_nanos());
        text += &format!("\n{name}\0{}\0{ns}", m.len());
    }
    let stamp = dir.join(format!("stamp-{}", &sha256_hex(text.as_bytes())[..32]));
    if let Ok(s) = std::fs::read_to_string(&stamp)
        && s.len() == 64
        && s.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Ok(s);
    }
    let (_, sha) = crate::flow_set::digest_for(path, false)?;
    // a stamp that cannot be written only costs the next call a hash
    let _ = write_atomic(dir, &stamp, sha.as_bytes());
    Ok(sha)
}

fn write_atomic(dir: &Path, to: &Path, bytes: &[u8]) -> std::io::Result<()> {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let k = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = dir.join(format!(".tmp-{}-{k}", std::process::id()));
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, to).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

/// `idx` copied to `entry` so that no reader ever sees half of it.
fn keep(dir: &Path, idx: &Path, entry: &Path) -> std::io::Result<()> {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let k = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = dir.join(format!(".tmp-{}-{k}", std::process::id()));
    std::fs::copy(idx, &tmp)?;
    std::fs::rename(&tmp, entry).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

/// Is `entry` a whole index of this program: its header reads and the length it declares is the length of the file?
fn is_sound(entry: &Path) -> bool {
    let Ok(mut f) = File::open(entry) else {
        return false;
    };
    let Ok(size) = f.metadata().map(|m| m.len()) else {
        return false;
    };
    Head::read(&mut f, size).is_ok()
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
    let prepared = std::fs::create_dir_all(dir)
        .map_err(|e| cache_error("cannot create", dir, e))
        .and_then(|_| sha_of(dir, path));
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
            // a link costs nothing; a folder on another volume gets a copy
            if std::fs::hard_link(&entry, idx).is_ok() || std::fs::copy(&entry, idx).is_ok() {
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
        Err(e) if strict => return Err(e),
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
