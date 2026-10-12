//! The folder of the spectral libraries of the desktop app: ONE native folder (default `~/Documents/mzLab/Librerie`, `%USERPROFILE%\Documents\mzLab\Librerie`),
//! read memory-mapped (`memmap2`, see `mapped.rs`) and written by the page through the `mzlab://localhost/lib/<route>` protocol (routes in `protocol.rs`).
//! Layout and states: `mzlab/web/libreria-cartella.js`. Only plain names of the folder are reachable (`name` or `.mzlab/name`): no `..`, no absolute path.
//! The path chosen by the user and the switch of the external downloads live in `config.json` of the data folder of the app.

use std::collections::HashMap;
use std::fs;
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::UNIX_EPOCH;

use mzlab_core::Error;
use serde::{Deserialize, Serialize};

use crate::mapped::Mapped;

pub const META: &str = ".mzlab";

pub type Result<T> = std::result::Result<T, Error>;

fn io(e: &std::io::Error) -> Error {
    Error::new("lib.err.io").with("detail", e)
}

/// A file of the folder as the page sees it (`mtime` in milliseconds since 1970, like `File.lastModified`).
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct Entry {
    pub name: String,
    pub size: u64,
    pub mtime: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct Config {
    /// the folder of the libraries chosen by the user (none = the default one)
    #[serde(default)]
    pub cartella: Option<String>,
    /// «Scarica librerie pubbliche da fonti esterne»: off until the user turns it on
    #[serde(default)]
    pub esterne: bool,
    /// the folder of the data pointed at by the user (`cartella.rs`)
    #[serde(default)]
    pub dati: Option<String>,
}

pub fn read_config(data_dir: &Path) -> Config {
    fs::read_to_string(data_dir.join("config.json"))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn write_config(data_dir: &Path, c: &Config) -> Result<()> {
    let tmp = data_dir.join("config.json.part");
    fs::write(&tmp, serde_json::to_vec_pretty(c).unwrap_or_default()).map_err(|e| io(&e))?;
    fs::rename(&tmp, data_dir.join("config.json")).map_err(|e| io(&e))
}

/// `~/Documents/mzLab/Librerie` (macOS, Linux) or `%USERPROFILE%\Documents\mzLab\Librerie` (Windows).
pub fn default_dir() -> PathBuf {
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join("Documents").join("mzLab").join("Librerie")
}

fn mtime_ms(m: &fs::Metadata) -> i64 {
    m.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_millis() as i64)
}

struct Cached {
    size: u64,
    mtime: i64,
    map: Arc<Mapped>,
}

pub struct Folder {
    root: Mutex<PathBuf>,
    maps: Mutex<HashMap<PathBuf, Cached>>,
}

impl Default for Folder {
    fn default() -> Self {
        Folder::new(default_dir())
    }
}

impl Folder {
    pub fn new(root: PathBuf) -> Folder {
        Folder {
            root: Mutex::new(root),
            maps: Mutex::new(HashMap::new()),
        }
    }

    pub fn root(&self) -> PathBuf {
        self.root.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub fn set_root(&self, root: PathBuf) {
        *self.root.lock().unwrap_or_else(|e| e.into_inner()) = root;
        self.maps.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }

    /// The path of `name` or `.mzlab/name` inside the folder, never outside it.
    pub fn resolve(&self, rel: &str) -> Result<PathBuf> {
        let parts: Vec<&str> = rel.split('/').collect();
        let plain =
            |p: &&str| !p.is_empty() && *p != "." && *p != ".." && !p.contains(['\\', ':', '\0']);
        let ok = match parts.len() {
            1 => plain(&parts[0]),
            2 => parts[0] == META && plain(&parts[1]),
            _ => false,
        };
        if !ok {
            return Err(Error::new("lib.err.io").with("detail", "bad path"));
        }
        Ok(parts.iter().fold(self.root(), |p, c| p.join(c)))
    }

    /// The files of the folder (`sub` = "") or of `.mzlab`; a folder that does not exist yet is empty.
    pub fn list(&self, sub: &str) -> Result<Vec<Entry>> {
        let dir = match sub {
            "" => self.root(),
            META => self.root().join(META),
            _ => return Err(Error::new("lib.err.io").with("detail", "bad path")),
        };
        let mut out = Vec::new();
        let rd = match fs::read_dir(&dir) {
            Ok(r) => r,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(out),
            Err(e) => return Err(io(&e)),
        };
        for e in rd.flatten() {
            let Ok(m) = e.metadata() else { continue };
            let name = e.file_name().to_string_lossy().into_owned();
            if m.is_file() && !name.ends_with(".part") {
                out.push(Entry {
                    name,
                    size: m.len(),
                    mtime: mtime_ms(&m),
                });
            }
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    pub fn stat(&self, rel: &str) -> Result<Entry> {
        let p = self.resolve(rel)?;
        let m = fs::metadata(&p).map_err(|e| io(&e))?;
        Ok(Entry {
            name: rel.rsplit('/').next().unwrap_or(rel).to_string(),
            size: m.len(),
            mtime: mtime_ms(&m),
        })
    }

    fn map(&self, p: &Path) -> Result<Arc<Mapped>> {
        let m = fs::metadata(p).map_err(|e| io(&e))?;
        let (size, mtime) = (m.len(), mtime_ms(&m));
        let mut maps = self.maps.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(c) = maps.get(p) {
            if c.size == size && c.mtime == mtime {
                return Ok(c.map.clone());
            }
        }
        let map = Arc::new(Mapped::open(p).map_err(|e| io(&e))?);
        if maps.len() > 16 {
            maps.clear();
        }
        maps.insert(
            p.to_path_buf(),
            Cached {
                size,
                mtime,
                map: map.clone(),
            },
        );
        Ok(map)
    }

    /// `len` bytes from `off` (fewer at the end of the file), read from the memory map.
    pub fn read(&self, rel: &str, off: u64, len: u64) -> Result<Vec<u8>> {
        let map = self.map(&self.resolve(rel)?)?;
        let b = map.bytes();
        let a = (off as usize).min(b.len());
        let z = a.saturating_add(len as usize).min(b.len());
        Ok(b[a..z].to_vec())
    }

    /// A piece of a file the page writes in order: the bytes go to `name.part`, which becomes `name` (replacing it) when the last byte of `total` arrived.
    /// Returns true when the file is complete.
    pub fn put(&self, rel: &str, off: u64, total: u64, data: &[u8]) -> Result<bool> {
        let path = self.resolve(rel)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| io(&e))?;
        }
        let part = PathBuf::from(format!("{}.part", path.display()));
        let mut f = fs::OpenOptions::new()
            .create(true)
            .truncate(off == 0)
            .write(true)
            .open(&part)
            .map_err(|e| io(&e))?;
        f.seek(SeekFrom::Start(off)).map_err(|e| io(&e))?;
        f.write_all(data).map_err(|e| io(&e))?;
        drop(f);
        let done = off + data.len() as u64 >= total;
        if done {
            fs::rename(&part, &path).map_err(|e| io(&e))?;
            self.maps
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .remove(&path);
        }
        Ok(done)
    }

    pub fn remove(&self, rel: &str) -> Result<()> {
        let p = self.resolve(rel)?;
        self.maps
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&p);
        match fs::remove_file(&p) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(io(&e)),
        }
    }

    /// `from` becomes `to`: the old `to` stays until the new file is complete (the rename replaces it in one step).
    pub fn rename(&self, from: &str, to: &str) -> Result<()> {
        let (a, b) = (self.resolve(from)?, self.resolve(to)?);
        {
            let mut m = self.maps.lock().unwrap_or_else(|e| e.into_inner());
            m.remove(&a);
            m.remove(&b);
        }
        fs::rename(&a, &b).map_err(|e| io(&e))
    }
}
