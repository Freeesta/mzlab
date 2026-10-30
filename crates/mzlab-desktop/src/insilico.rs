//! The in-silico candidate package in the desktop app: the same package as the site (`static/insilico/`, written by `mzlab-cli insilico pack`), kept as `insilico/NNN.bin` and
//! `insilico/catalogo.json` under the folder of the libraries (`librerie.rs`; the package is a folder of its own, written here with checked names, not through the page's `lib/*` routes). First use fetches it from the site (each piece checked with its size and SHA-256, and inflated once
//! to check the rows too); after that it is read from disk, offline: `spectra.insilico` takes `insilico/catalogo.json` as its `suspects` file and reads the pieces next to it.
//! Routes `insilico/*` in `protocol.rs`. Every error is a key `normative.dl.err.*` (the keys of the other package) of `lang/`.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::librerie::Folder;
use crate::update::Source;

const SITE: &str = "static/insilico/";
pub const FORMAT: &str = "insilico/1";
/// The folder of the package inside the folder of the libraries, and its catalogue.
pub const DIR: &str = "insilico";
pub const CATALOG: &str = "catalogo.json";
/// A piece inflated above this many bytes is refused.
const MAX_RAW: u64 = 64 << 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InsilicoError {
    pub key: &'static str,
    pub detail: String,
}

impl InsilicoError {
    fn new(key: &'static str, detail: impl Into<String>) -> Self {
        InsilicoError {
            key,
            detail: detail.into(),
        }
    }
    fn io(e: impl std::fmt::Display) -> Self {
        InsilicoError::new("normative.dl.err.io", e.to_string())
    }
    pub fn to_json(&self) -> String {
        serde_json::json!({"status": "error", "error_key": self.key, "detail": self.detail})
            .to_string()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Piece {
    pub file: String,
    pub size: u64,
    pub sha256: String,
    pub size_raw: u64,
    pub sha256_raw: String,
}

/// The fields of `catalogo.json` (`insilico/1`) the app needs.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Catalog {
    pub format: String,
    pub size_compressed: u64,
    pub sha256: String,
    #[serde(default)]
    pub rows: u64,
    pub pieces: Vec<Piece>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Status {
    pub installed: bool,
    pub sha256: String,
    pub rows: u64,
    pub size_compressed: u64,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Check {
    None,
    Current,
    Update,
}

fn hex(b: &[u8]) -> String {
    Sha256::digest(b)
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect()
}

pub fn parse_catalog(bytes: &[u8]) -> Result<Catalog, InsilicoError> {
    let bad = |d: &str| InsilicoError::new("normative.dl.err.catalog", d);
    let c: Catalog = serde_json::from_slice(bytes).map_err(|e| bad(&e.to_string()))?;
    let plain =
        |f: &str| f.len() == 7 && f.ends_with(".bin") && f[..3].bytes().all(|b| b.is_ascii_digit());
    if c.format != FORMAT || c.sha256.len() != 64 || c.pieces.is_empty() {
        return Err(bad(&c.format));
    }
    let hash = |h: &str| h.len() == 64;
    if c.pieces.iter().any(|p| {
        !plain(&p.file) || !hash(&p.sha256) || !hash(&p.sha256_raw) || p.size_raw > MAX_RAW
    }) {
        return Err(bad("piece"));
    }
    if c.pieces.iter().map(|p| p.size).sum::<u64>() != c.size_compressed {
        return Err(bad("size"));
    }
    Ok(c)
}

fn fetch_catalog(src: &dyn Source) -> Result<(Catalog, Vec<u8>), InsilicoError> {
    let b = src.get(&format!("{SITE}catalogo.json")).map_err(|e| {
        InsilicoError::new(
            if e.key == "update.error.offline" {
                "normative.dl.err.offline"
            } else {
                "normative.dl.err.http"
            },
            e.detail,
        )
    })?;
    Ok((parse_catalog(&b)?, b))
}

/// The folder of the package; `flow run` takes its `catalogo.json` as the `suspects` file and reads the pieces next to it.
pub fn dir(folder: &Folder) -> PathBuf {
    folder.root().join(DIR)
}

/// A file of the package written whole: to `name.part` first, then renamed over the old one.
fn put(folder: &Folder, name: &str, bytes: &[u8]) -> Result<(), InsilicoError> {
    let d = dir(folder);
    std::fs::create_dir_all(&d).map_err(InsilicoError::io)?;
    let part = d.join(format!("{name}.part"));
    std::fs::File::create(&part)
        .and_then(|mut f| f.write_all(bytes))
        .map_err(InsilicoError::io)?;
    std::fs::rename(&part, d.join(name)).map_err(InsilicoError::io)
}

/// What is installed, read from the catalogue stored in the folder.
pub fn installed(folder: &Folder) -> Option<Catalog> {
    parse_catalog(&std::fs::read(dir(folder).join(CATALOG)).ok()?).ok()
}

pub fn status(folder: &Folder) -> Status {
    match installed(folder) {
        Some(c) => Status {
            installed: true,
            sha256: c.sha256,
            rows: c.rows,
            size_compressed: c.size_compressed,
        },
        None => Status {
            installed: false,
            sha256: String::new(),
            rows: 0,
            size_compressed: 0,
        },
    }
}

/// «Check for updates»: the catalogue of the site against the installed one; nothing is downloaded.
pub fn check(folder: &Folder, src: &dyn Source) -> Result<Check, InsilicoError> {
    let (live, _) = fetch_catalog(src)?;
    Ok(match installed(folder) {
        None => Check::None,
        Some(c) if c.sha256 == live.sha256 => Check::Current,
        Some(_) => Check::Update,
    })
}

/// A piece as it came, checked: compressed size and SHA-256, then the inflated rows (size and SHA-256).
fn check_piece(p: &Piece, z: &[u8]) -> Result<(), InsilicoError> {
    let bad = || InsilicoError::new("normative.dl.err.hash", p.file.clone());
    if z.len() as u64 != p.size || hex(z) != p.sha256 {
        return Err(bad());
    }
    let mut raw = Vec::new();
    flate2::read::ZlibDecoder::new(z)
        .take(MAX_RAW + 1)
        .read_to_end(&mut raw)
        .map_err(|_| bad())?;
    if raw.len() as u64 != p.size_raw || hex(&raw) != p.sha256_raw {
        return Err(bad());
    }
    Ok(())
}

/// Downloads the package of the site into the folder: every piece checked and written, the catalogue written last (so an install that stops leaves the previous package
/// whole), the pieces of the previous catalogue that the new one does not name removed. `progress(piece, pieces)` after each piece.
pub fn install(
    folder: &Folder,
    src: &dyn Source,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(usize, usize),
) -> Result<Status, InsilicoError> {
    let (cat, text) = fetch_catalog(src)?;
    let before = installed(folder);
    for (i, p) in cat.pieces.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            return Err(InsilicoError::new("normative.dl.err.cancelled", ""));
        }
        let mut got = None;
        for _ in 0..2 {
            let b = src
                .get(&format!("{SITE}{}", p.file))
                .map_err(|e| InsilicoError::new("normative.dl.err.offline", e.detail))?;
            if check_piece(p, &b).is_ok() {
                got = Some(b);
                break;
            }
        }
        let b = got.ok_or_else(|| InsilicoError::new("normative.dl.err.hash", p.file.clone()))?;
        put(folder, &p.file, &b)?;
        progress(i + 1, cat.pieces.len());
    }
    put(folder, CATALOG, &text)?;
    if let Some(old) = before {
        for p in old
            .pieces
            .iter()
            .filter(|p| !cat.pieces.iter().any(|q| q.file == p.file))
        {
            let _ = std::fs::remove_file(dir(folder).join(&p.file));
        }
    }
    Ok(status(folder))
}

#[cfg(test)]
#[path = "insilico_tests.rs"]
mod tests;
