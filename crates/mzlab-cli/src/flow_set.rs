//! The datasets of vendor files on disk as `convert.mzml` and `mzlab-cli convert` take them: a file, a `.wiff` with its `.wiff.scan`, or the files of a
//! `.D`/`.d`/`.raw` folder (the `FileSet` of a reader of the registry), their SHA-256, and the sample a plan input names (`NAME@ID`).

use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

use mzlab_core::budget::sha256_hex;
use mzlab_core::hashio::hash_read;
use mzlab_core::lettori::{FileSet, Membro};
use mzlab_core::{Error, Result};

use crate::flow::io;

fn lower(p: &Path) -> String {
    p.file_name()
        .map_or(String::new(), |n| n.to_string_lossy().to_lowercase())
}

/// Is the folder `p` one dataset of a maker (`.D` of Agilent, `.d` of Bruker, `.raw` of Waters) and not a folder to look through?
pub(crate) fn is_set_folder(p: &Path) -> bool {
    let n = lower(p);
    p.is_dir() && [".d", ".raw"].iter().any(|e| n.ends_with(e))
}

fn walk(dir: &Path, prefix: &str, out: &mut Vec<(String, PathBuf)>) -> Result<()> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(io)?
        .flatten()
        .map(|e| e.path())
        .collect();
    entries.sort();
    for p in entries {
        let name = format!(
            "{prefix}/{}",
            p.file_name().unwrap_or_default().to_string_lossy()
        );
        if p.is_dir() {
            walk(&p, &name, out)?;
        } else {
            out.push((name, p));
        }
    }
    Ok(())
}

/// The members of the dataset at `path` as (logical name, file): the folder name leads the names of a folder, a `.wiff` brings the `.wiff.scan` beside it.
pub(crate) fn members(path: &Path) -> Result<Vec<(String, PathBuf)>> {
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let mut out = Vec::new();
    if path.is_dir() {
        walk(path, &name, &mut out)?;
    } else {
        out.push((name.clone(), path.to_path_buf()));
        if lower(path).ends_with(".wiff") {
            let scan = path.with_file_name(format!("{name}.scan"));
            if scan.is_file() {
                out.push((format!("{name}.scan"), scan));
            }
        }
    }
    Ok(out)
}

/// The `FileSet` of the dataset at `path`, its members opened on the disk.
pub(crate) fn file_set(path: &Path) -> Result<FileSet> {
    let mut membri = Vec::new();
    for (nome, p) in members(path)? {
        let f = File::open(&p).map_err(io)?;
        membri.push(Membro {
            nome,
            dim: f.metadata().map_err(io)?.len(),
            dati: Box::new(f),
            sha1: known_sha1(&p),
        });
    }
    Ok(FileSet { membri })
}

/// Size and SHA-256 of the file `p`; its SHA-1 comes with them in the same read and is kept for `file_set`, so the reader that writes the header of an mzML
/// does not read the file again for it.
fn hash_file(p: &Path) -> Result<(u64, String)> {
    let before = stamp(p);
    let (bytes, sha256, sha1) = hash_read(&mut File::open(p).map_err(io)?).map_err(io)?;
    if let (Some((len, time)), Ok(mut seen)) = (before.filter(|s| s.0 == bytes), SHA1.lock()) {
        seen.retain(|e| e.path != p);
        seen.push(Hashed {
            path: p.to_path_buf(),
            len,
            time,
            sha: sha1,
        });
    }
    Ok((bytes, sha256))
}

/// SHA-256 of the lone files `digest` has hashed, by path, size and modification time: the `.mzidx` build takes its source's hash from here instead of reading the file again.
static HASHED: Mutex<Vec<Hashed>> = Mutex::new(Vec::new());

/// SHA-1 of the files `hash_file` has read, in the same way.
static SHA1: Mutex<Vec<Hashed>> = Mutex::new(Vec::new());

struct Hashed {
    path: PathBuf,
    len: u64,
    time: Option<SystemTime>,
    sha: String,
}

fn stamp(p: &Path) -> Option<(u64, Option<SystemTime>)> {
    let m = std::fs::metadata(p).ok()?;
    Some((m.len(), m.modified().ok()))
}

/// Notes the SHA-256 of the lone file `path` of `bytes` bytes, which a caller has just hashed (or hashed while it wrote the file), for `known_sha`.
pub(crate) fn remember_sha(path: &Path, bytes: u64, sha: &str) {
    remember(path, stamp(path), bytes, sha);
}

/// `remember_sha` with the stamp of the file taken before it was hashed: a file that changed meanwhile is not noted.
fn remember(path: &Path, before: Option<(u64, Option<SystemTime>)>, bytes: u64, sha: &str) {
    if let (Some((len, time)), Ok(mut seen)) = (before.filter(|s| s.0 == bytes), HASHED.lock()) {
        seen.retain(|e| e.path != path);
        seen.push(Hashed {
            path: path.to_path_buf(),
            len,
            time,
            sha: sha.to_string(),
        });
    }
}

/// The SHA-1 `hash_file` took of `path`, if it is still the same file.
fn known_sha1(path: &Path) -> Option<String> {
    let (len, time) = stamp(path)?;
    let seen = SHA1.lock().ok()?;
    seen.iter()
        .find(|e| e.path == path && e.len == len && e.time == time)
        .map(|e| e.sha.clone())
}

/// Size and SHA-256 of `path`; the hash is not recomputed when the writer already took `known`.
pub(crate) fn digest_known(path: &Path, known: Option<String>) -> Result<(u64, String)> {
    match known {
        Some(sha) => Ok((std::fs::metadata(path).map_err(io)?.len(), sha)),
        None => digest(path),
    }
}

/// The SHA-256 `digest` took of the lone file `path`, if it is still the same file (size and modification time).
pub(crate) fn known_sha(path: &Path) -> Option<String> {
    let (len, time) = stamp(path)?;
    let seen = HASHED.lock().ok()?;
    seen.iter()
        .find(|e| e.path == path && e.len == len && e.time == time)
        .map(|e| e.sha.clone())
}

/// Bytes and SHA-256 of the dataset at `path`: a lone file hashes as its own bytes, a dataset of several members as the list of their names and hashes.
pub(crate) fn digest(path: &Path) -> Result<(u64, String)> {
    let list = members(path)?;
    if !path.is_dir() && list.len() == 1 {
        let before = stamp(path);
        let (bytes, hex) = hash_file(path)?;
        remember(path, before, bytes, &hex);
        return Ok((bytes, hex));
    }
    let (mut bytes, mut text) = (0, String::new());
    for (name, p) in &list {
        let (n, hex) = hash_file(p)?;
        bytes += n;
        text += &format!("{name}\0{hex}\n");
    }
    Ok((bytes, sha256_hex(text.as_bytes())))
}

/// The sample a plan input names: `NAME@ID` is the sample `ID` of the dataset `NAME`, a name without that ending is the first sample.
pub(crate) fn split_sample(label: &str) -> (&str, Option<u32>) {
    label
        .rsplit_once('@')
        .and_then(|(n, id)| id.parse().ok().map(|id| (n, Some(id))))
        .unwrap_or((label, None))
}

/// The dataset a plan input names: `NAME@ID` is the sample `ID` of `NAME`, anything else is its own name.
pub(crate) fn dataset_name(label: &str) -> &str {
    split_sample(label).0
}

/// Index of the sample the label `label` names among the `ids` of the reader.
pub(crate) fn sample_index(label: &str, ids: &[u32]) -> Result<usize> {
    match split_sample(label).1 {
        None => Ok(0),
        Some(id) => ids
            .iter()
            .position(|&s| s == id)
            .ok_or_else(|| Error::new("error.badIndex").with("c", id)),
    }
}
