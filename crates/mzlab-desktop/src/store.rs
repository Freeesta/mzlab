//! The files of the student and the notebook between two runs of the app: the page's archive (`archivio.js` in the browser, OPFS and IndexedDB) for the
//! native engine. A file is KNOWN BY NAME and stays where it is on the disk (no copy of a 2 GB mzML); only what the app makes itself (the mzML converted
//! from a native sample, a file sent as bytes) is copied into `files/`. `files.json` keeps name → path, size and date; `kv.json` the notebook; the indexes
//! live in the index folder of the app (`idxbuild.rs`), and an index is dropped when its file changes. A sample of a maker's file (`.raw`, `.wiff` +
//! `.wiff.scan`, a `.D` folder) is an entry with `nat`: its members stay where they are, the size and the date are the ones of all of them.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use mzlab_core::Error;
use serde_json::{json, Value};

pub type Result<T> = std::result::Result<T, Error>;

impl Store {
    /// Where `files.json`, `kv.json` and the files made by the app are. Reads what a previous run left.
    pub fn set_dir(&mut self, dir: PathBuf) {
        self.dir = Some(dir);
        self.loaded = false;
        self.files.clear();
        self.kv.clear();
    }

    fn dir(&mut self) -> PathBuf {
        self.dir.get_or_insert_with(|| std::env::temp_dir().join("mzlab-data")).clone()
    }

    /// The folder of the files made by the app (created when needed).
    pub fn own_dir(&mut self) -> Result<PathBuf> {
        let d = self.dir().join("files");
        std::fs::create_dir_all(&d).map_err(|e| io_error(&e))?;
        Ok(d)
    }

    fn load(&mut self) {
        if self.loaded {
            return;
        }
        self.loaded = true;
        let dir = self.dir();
        if let Some(Value::Object(m)) = std::fs::read(dir.join("files.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()) {
            for (name, e) in m {
                let path = PathBuf::from(e["path"].as_str().unwrap_or(""));
                if path.as_os_str().is_empty() {
                    continue;
                }
                self.seq += 1;
                let nat = Nat::from_json(&e["nat"]);
                self.files.insert(
                    name,
                    Entry { path, size: e["size"].as_u64().unwrap_or(0), mtime: e["mtime"].as_u64().unwrap_or(0), seq: self.seq, nat },
                );
            }
        }
        if let Some(Value::Object(m)) = std::fs::read(dir.join("kv.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()) {
            for (k, v) in m {
                if let Some(s) = v.as_str() {
                    self.kv.insert(k, s.to_string());
                }
            }
        }
    }

    fn save_files(&mut self) {
        let dir = self.dir();
        let m: serde_json::Map<String, Value> = self
            .files
            .iter()
            .map(|(n, e)| {
                let mut v = json!({ "path": e.path.to_string_lossy(), "size": e.size, "mtime": e.mtime });
                if let Some(nat) = &e.nat {
                    v["nat"] = nat.to_json();
                }
                (n.clone(), v)
            })
            .collect();
        write_atomic(&dir, "files.json", &Value::Object(m).to_string());
    }

    fn save_kv(&mut self) {
        let dir = self.dir();
        let m: serde_json::Map<String, Value> = self.kv.iter().map(|(k, v)| (k.clone(), Value::String(v.clone()))).collect();
        write_atomic(&dir, "kv.json", &Value::Object(m).to_string());
    }

    /// The files whose path still exists, with the size and date they have now. The second value of each pair is true when the file changed since
    /// it was registered (its index, if any, no longer belongs to it).
    pub fn list(&mut self) -> Vec<(String, Entry, bool)> {
        self.load();
        let mut gone = Vec::new();
        let mut out = Vec::new();
        for (name, e) in self.files.iter_mut() {
            match e.stamp() {
                None => gone.push(name.clone()),
                Some((size, mtime)) => {
                    let changed = size != e.size || mtime != e.mtime;
                    e.size = size;
                    e.mtime = mtime;
                    out.push((name.clone(), e.clone(), changed));
                }
            }
        }
        if !gone.is_empty() {
            for n in &gone {
                self.files.remove(n);
            }
            self.save_files();
        } else if out.iter().any(|(_, _, c)| *c) {
            self.save_files();
        }
        out
    }

    pub fn get(&mut self, name: &str) -> Option<Entry> {
        self.load();
        self.files.get(name).cloned()
    }

    /// Registers (or replaces) the file `name` that lives at `path`.
    pub fn put(&mut self, name: &str, path: PathBuf) -> Result<Entry> {
        self.load();
        let name = clean(name)?;
        let (size, mtime) = stamp(&path).ok_or_else(|| Error::new("err.file.unknown").with("detail", "unknown file"))?;
        self.seq += 1;
        let e = Entry { path, size, mtime, seq: self.seq, nat: None };
        self.files.insert(name.to_string(), e.clone());
        self.save_files();
        Ok(e)
    }

    /// Registers (or replaces) the sample `name` of a maker's file: the members stay where they are.
    pub fn put_nat(&mut self, name: &str, nat: Nat) -> Result<Entry> {
        self.load();
        let name = clean(name)?;
        let path = nat.members.first().map(|m| m.1.clone()).ok_or_else(|| Error::new("err.file.notAccepted").with("name", name))?;
        self.seq += 1;
        let mut e = Entry { path, size: 0, mtime: 0, seq: self.seq, nat: Some(nat) };
        (e.size, e.mtime) = e.stamp().ok_or_else(|| Error::new("err.file.unknown").with("detail", "unknown file"))?;
        self.files.insert(name.to_string(), e.clone());
        self.save_files();
        Ok(e)
    }

    pub fn remove(&mut self, name: &str) {
        self.load();
        if let Some(e) = self.files.remove(name) {
            if let Ok(own) = self.own_dir() {
                let paths = e.nat.iter().flat_map(|n| n.members.iter().map(|m| m.1.clone())).chain([e.path.clone()]);
                for p in paths.filter(|p| p.starts_with(&own)) {
                    let _ = std::fs::remove_file(p);
                }
            }
            self.save_files();
        }
    }

    /// «New session» with fresh: every file forgotten (the ones made by the app go; the ones of the student stay where they are), the notebook empty.
    pub fn clear(&mut self) {
        self.load();
        if let Ok(own) = self.own_dir() {
            let _ = std::fs::remove_dir_all(own);
        }
        self.files.clear();
        self.kv.clear();
        self.save_files();
        self.save_kv();
    }

    pub fn kv_get(&mut self, key: &str) -> Option<String> {
        self.load();
        self.kv.get(key).cloned()
    }

    pub fn kv_set(&mut self, key: &str, value: &str) {
        self.load();
        self.kv.insert(key.to_string(), value.to_string());
        self.save_kv();
    }
}

/// A name that is only a file name (the page never chooses a path).
pub fn clean(name: &str) -> Result<&str> {
    if name.is_empty() || name.contains(['/', '\\', '\0']) || name == "." || name == ".." || name.starts_with('.') {
        return Err(Error::new("err.file.notAccepted").with("name", name));
    }
    Ok(name)
}

fn stamp(path: &Path) -> Option<(u64, u64)> {
    let m = std::fs::metadata(path).ok()?;
    let t = m.modified().ok()?.duration_since(UNIX_EPOCH).ok()?.as_millis() as u64;
    Some((m.len(), t))
}

/// A sample of a maker's file: which sample (`c`) of which dataset (`ds`, one key for the samples that share the members), the members as
/// `(name relative to the dataset, path)` and whether the profile scans are read as centroids.
#[derive(Clone, Debug, PartialEq)]
pub struct Nat {
    pub ds: String,
    pub c: usize,
    pub id: u32,
    pub centroid: bool,
    pub members: Vec<(String, PathBuf)>,
}

impl Nat {
    fn to_json(&self) -> Value {
        let members: Vec<Value> = self.members.iter().map(|(n, p)| json!([n, p.to_string_lossy()])).collect();
        json!({ "ds": self.ds, "c": self.c, "id": self.id, "centroid": self.centroid, "members": members })
    }

    fn from_json(v: &Value) -> Option<Nat> {
        let members: Vec<(String, PathBuf)> = v["members"].as_array()?.iter().filter_map(|m| Some((m[0].as_str()?.to_string(), PathBuf::from(m[1].as_str()?)))).collect();
        (!members.is_empty()).then(|| Nat {
            ds: v["ds"].as_str().unwrap_or("").to_string(),
            c: v["c"].as_u64().unwrap_or(0) as usize,
            id: v["id"].as_u64().unwrap_or(0) as u32,
            centroid: v["centroid"].as_bool().unwrap_or(false),
            members,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub path: PathBuf,
    /// set for a sample of a maker's file (`path` is then its first member)
    pub nat: Option<Nat>,
    pub size: u64,
    /// modification date in milliseconds: the order of the methods (`.dam`) and the test that an index still belongs to its file
    pub mtime: u64,
    /// order of arrival in this run: files of the same millisecond keep it
    pub seq: u64,
}

impl Entry {
    /// Size and date now: of the file, or of all the members of a native sample (the sum of the sizes and the latest date). None when one is gone.
    fn stamp(&self) -> Option<(u64, u64)> {
        let Some(nat) = &self.nat else { return stamp(&self.path) };
        nat.members.iter().try_fold((0u64, 0u64), |(s, t), (_, p)| stamp(p).map(|(a, b)| (s + a, t.max(b))))
    }
}

#[derive(Default)]
pub struct Store {
    dir: Option<PathBuf>,
    loaded: bool,
    files: BTreeMap<String, Entry>,
    kv: BTreeMap<String, String>,
    seq: u64,
}

fn io_error(e: &std::io::Error) -> Error {
    Error::new("err.file.raw").with("detail", e.to_string())
}

fn write_atomic(dir: &Path, name: &str, text: &str) {
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    let tmp = dir.join(format!("{name}.part"));
    if std::fs::write(&tmp, text).is_ok() {
        let _ = std::fs::rename(&tmp, dir.join(name));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn files_and_notebook_survive_a_new_run() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("a.mzML");
        std::fs::write(&f, b"<mzML/>").unwrap();
        let mut s = Store::default();
        s.set_dir(d.path().join("data"));
        s.put("a.mzML", f.clone()).unwrap();
        s.kv_set("notebook", "{\"session\":[]}");
        let mut t = Store::default();
        t.set_dir(d.path().join("data"));
        let l = t.list();
        assert_eq!(l.len(), 1);
        assert_eq!((l[0].0.as_str(), l[0].1.size, l[0].2), ("a.mzML", 7, false));
        assert_eq!(t.kv_get("notebook").as_deref(), Some("{\"session\":[]}"));
        std::fs::write(&f, b"<mzML>x</mzML>").unwrap();
        assert!(t.list()[0].2, "a changed file is flagged");
        std::fs::remove_file(&f).unwrap();
        assert!(t.list().is_empty(), "a file that is gone is forgotten");
    }

    #[test]
    fn names_are_names() {
        assert!(clean("a.mzML").is_ok());
        for bad in ["", "../x", "a/b", ".hidden", "a\\b"] {
            assert!(clean(bad).is_err(), "{bad}");
        }
    }
}
