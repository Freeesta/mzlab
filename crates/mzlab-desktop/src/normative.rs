//! The regulatory-list package in the desktop app: the same package as the site (`static/normative/`, written by `mzlab-cli normative pack`), kept as
//! `normative.mzsus` in the folder of the libraries (`librerie.rs`) with its catalogue in `.mzlab/normative.catalogo.json`. First use fetches it from the site
//! (pieces checked one by one with their SHA-256, the inflated file checked as a whole) or takes a package folder or `.mzsus` the user picked; after that
//! it is read from disk, offline. Routes `normative/*` in `protocol.rs`. Every error is a key `normative.dl.err.*` of `lang/`.

use std::io::Read;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use mzlab_feat::suspects::Suspects;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::librerie::Folder;
use crate::update::Source;

pub const CATALOG: &str = "static/normative/catalogo.json";
const PIECES: &str = "static/normative/";
pub const FORMAT: &str = "normative/1";
/// The package inside the folder of the libraries, and the catalogue it was installed from.
pub const FILE: &str = "normative.mzsus";
pub const META_FILE: &str = ".mzlab/normative.catalogo.json";
/// An inflated package above this many bytes is refused (the real one is under 1 MB).
const MAX_RAW: u64 = 256 << 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormError {
    pub key: &'static str,
    pub detail: String,
}

impl NormError {
    fn new(key: &'static str, detail: impl Into<String>) -> Self {
        NormError {
            key,
            detail: detail.into(),
        }
    }
    fn io(e: impl std::fmt::Display) -> Self {
        NormError::new("normative.dl.err.io", e.to_string())
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
}

/// The fields of `catalogo.json` (`normative/1`) the app needs; the rest (lists, attributions) is kept as text next to the file.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Catalog {
    pub format: String,
    pub size: u64,
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
    pub size: u64,
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

pub fn parse_catalog(bytes: &[u8]) -> Result<Catalog, NormError> {
    let bad = |d: &str| NormError::new("normative.dl.err.catalog", d);
    let c: Catalog = serde_json::from_slice(bytes).map_err(|e| bad(&e.to_string()))?;
    let plain =
        |f: &str| f.len() == 7 && f.ends_with(".bin") && f[..3].bytes().all(|b| b.is_ascii_digit());
    if c.format != FORMAT || c.sha256.len() != 64 || c.size > MAX_RAW {
        return Err(bad(&c.format));
    }
    if c.pieces
        .iter()
        .any(|p| !plain(&p.file) || p.sha256.len() != 64)
    {
        return Err(bad("piece name"));
    }
    if c.pieces.iter().map(|p| p.size).sum::<u64>() != c.size_compressed {
        return Err(bad("size"));
    }
    Ok(c)
}

fn fetch_catalog(src: &dyn Source) -> Result<(Catalog, Vec<u8>), NormError> {
    let b = src.get(CATALOG).map_err(|e| {
        NormError::new(
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

/// A raw `.mzsus` that carries the `listings` extension.
fn check_raw(raw: &[u8]) -> Result<(), NormError> {
    let s = Suspects::open(raw)
        .map_err(|e| NormError::new("normative.dl.err.hash", format!("{e:?}")))?;
    s.listings()
        .map(|_| ())
        .ok_or_else(|| NormError::new("normative.dl.err.hash", "no listings extension"))
}

fn write(folder: &Folder, raw: &[u8], catalog: &[u8]) -> Result<(), NormError> {
    folder
        .put(FILE, 0, raw.len() as u64, raw)
        .map_err(NormError::io)?;
    folder
        .put(META_FILE, 0, catalog.len() as u64, catalog)
        .map_err(NormError::io)?;
    Ok(())
}

/// What is installed, read from the catalogue stored next to the file.
pub fn installed(folder: &Folder) -> Option<Catalog> {
    let st = folder.stat(META_FILE).ok()?;
    let b = folder.read(META_FILE, 0, st.size).ok()?;
    parse_catalog(&b).ok()
}

/// The installed catalogue as the site serves it (the same bytes), for the «Regulatory lists» section of the Library window.
pub fn catalog_text(folder: &Folder) -> Result<String, NormError> {
    installed(folder).ok_or_else(|| NormError::new("normative.dl.err.missing", ""))?;
    let st = folder.stat(META_FILE).map_err(NormError::io)?;
    let b = folder.read(META_FILE, 0, st.size).map_err(NormError::io)?;
    String::from_utf8(b).map_err(NormError::io)
}

pub fn status(folder: &Folder) -> Status {
    match installed(folder).zip(folder.stat(FILE).ok()) {
        Some((c, f)) => Status {
            installed: true,
            sha256: c.sha256,
            size: f.size,
        },
        None => Status {
            installed: false,
            sha256: String::new(),
            size: 0,
        },
    }
}

/// «Check for updates»: the catalogue of the site against the installed one; nothing is downloaded.
pub fn check(folder: &Folder, src: &dyn Source) -> Result<Check, NormError> {
    let (live, _) = fetch_catalog(src)?;
    Ok(match installed(folder) {
        None => Check::None,
        Some(c) if c.sha256 == live.sha256 => Check::Current,
        Some(_) => Check::Update,
    })
}

/// Downloads the package of the site into the folder: every piece checked (size and SHA-256), the inflated file checked as a whole; the old file stays until
/// the new one is complete. `progress(piece, pieces)` after each piece.
pub fn install(
    folder: &Folder,
    src: &dyn Source,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(usize, usize),
) -> Result<Status, NormError> {
    let (cat, text) = fetch_catalog(src)?;
    let mut z = Vec::with_capacity(cat.size_compressed as usize);
    for (i, p) in cat.pieces.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            return Err(NormError::new("normative.dl.err.cancelled", ""));
        }
        let mut got = None;
        for _ in 0..2 {
            let b = src
                .get(&format!("{PIECES}{}", p.file))
                .map_err(|e| NormError::new("normative.dl.err.offline", e.detail))?;
            if b.len() as u64 == p.size && hex(&b) == p.sha256 {
                got = Some(b);
                break;
            }
        }
        z.extend(got.ok_or_else(|| NormError::new("normative.dl.err.hash", p.file.clone()))?);
        progress(i + 1, cat.pieces.len());
    }
    let mut raw = Vec::new();
    flate2::read::ZlibDecoder::new(&z[..])
        .take(MAX_RAW + 1)
        .read_to_end(&mut raw)
        .map_err(|e| NormError::new("normative.dl.err.hash", e.to_string()))?;
    if raw.len() as u64 != cat.size || hex(&raw) != cat.sha256 {
        return Err(NormError::new("normative.dl.err.hash", FILE));
    }
    check_raw(&raw)?;
    write(folder, &raw, &text)?;
    Ok(status(folder))
}

/// The package the user picked: a `.mzsus`, or a package folder (`normative.mzsus` and `catalogo.json` as `mzlab-cli normative pack` writes them).
pub fn import(folder: &Folder, from: &Path) -> Result<Status, NormError> {
    let (file, cat) = if from.is_dir() {
        (from.join(FILE), Some(from.join("catalogo.json")))
    } else {
        (from.to_path_buf(), None)
    };
    let raw = std::fs::read(&file).map_err(NormError::io)?;
    check_raw(&raw)?;
    let text = match cat.filter(|p| p.is_file()) {
        Some(p) => {
            let t = std::fs::read(p).map_err(NormError::io)?;
            let c = parse_catalog(&t)?;
            if c.sha256 != hex(&raw) {
                return Err(NormError::new("normative.dl.err.hash", FILE));
            }
            t
        }
        // a bare `.mzsus`: the catalogue is written from the file itself
        None => serde_json::to_vec(&Catalog {
            format: FORMAT.into(),
            size: raw.len() as u64,
            size_compressed: 0,
            sha256: hex(&raw),
            rows: 0,
            pieces: vec![],
        })
        .map_err(NormError::io)?,
    };
    write(folder, &raw, &text)?;
    Ok(status(folder))
}

/// The `listings` JSON of the installed package (the text of `Suspects::listings_json`, as in the `.wasm` and `mzlab-cli normative listings`).
pub fn listings_json(folder: &Folder) -> Result<String, NormError> {
    let st = folder
        .stat(FILE)
        .map_err(|_| NormError::new("normative.dl.err.missing", FILE))?;
    let raw = folder.read(FILE, 0, st.size).map_err(NormError::io)?;
    let s = Suspects::open(&raw)
        .map_err(|e| NormError::new("normative.dl.err.hash", format!("{e:?}")))?;
    s.listings_json()
        .ok_or_else(|| NormError::new("normative.dl.err.hash", "no listings extension"))
}

#[cfg(test)]
#[path = "normative_tests.rs"]
mod tests;
