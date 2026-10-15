// Ported from Attributio paths.py.
//! Where the files are: two closures instead of a disk. `read(path)` gives the bytes of a file (`None` when there is none) and `list(dir)` the names directly
//! inside a folder; paths are text with «/», relative to the campaign folder (a leading «/» is outside it, the closure decides what that means).

use std::collections::BTreeMap;

/// A name inside a folder and whether it is a folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub is_dir: bool,
}

pub struct Io<'a> {
    read: &'a dyn Fn(&str) -> Option<Vec<u8>>,
    list: &'a dyn Fn(&str) -> Vec<Entry>,
}

/// `Path(base) / rel`: an absolute `rel` replaces the base.
pub fn join(base: &str, rel: &str) -> String {
    if rel.starts_with('/') || base.is_empty() {
        rel.to_string()
    } else {
        format!("{}/{}", base.trim_end_matches('/'), rel)
    }
}

/// The last component of a path (`Path.name`).
pub fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn split(path: &str) -> (&str, &str) {
    match path.rfind('/') {
        Some(i) => (&path[..i], &path[i + 1..]),
        None => ("", path),
    }
}

impl<'a> Io<'a> {
    pub fn new(
        read: &'a dyn Fn(&str) -> Option<Vec<u8>>,
        list: &'a dyn Fn(&str) -> Vec<Entry>,
    ) -> Self {
        Self { read, list }
    }
    pub fn read(&self, path: &str) -> Option<Vec<u8>> {
        (self.read)(path)
    }
    /// The text of a file (bad UTF-8 replaced, as `errors="replace"`).
    pub fn text(&self, path: &str) -> Option<String> {
        self.read(path)
            .map(|b| String::from_utf8_lossy(&b).into_owned())
    }
    pub fn list(&self, dir: &str) -> Vec<Entry> {
        (self.list)(dir)
    }
    fn entry(&self, path: &str) -> Option<Entry> {
        let (dir, name) = split(path);
        self.list(dir).into_iter().find(|e| e.name == name)
    }
    /// A file or a folder is there.
    pub fn exists(&self, path: &str) -> bool {
        self.entry(path).is_some() || self.read(path).is_some()
    }
    pub fn is_dir(&self, path: &str) -> bool {
        self.entry(path).is_some_and(|e| e.is_dir)
    }
}

/// Files in memory (a map from path to bytes), for the tests and for the files a page has already read.
#[derive(Default)]
pub struct MemFiles(pub BTreeMap<String, Vec<u8>>);

impl MemFiles {
    pub fn add(&mut self, path: &str, data: &[u8]) {
        self.0.insert(path.to_string(), data.to_vec());
    }
    pub fn read(&self, path: &str) -> Option<Vec<u8>> {
        self.0.get(path).cloned()
    }
    pub fn list(&self, dir: &str) -> Vec<Entry> {
        let prefix = if dir.is_empty() {
            String::new()
        } else {
            format!("{}/", dir.trim_end_matches('/'))
        };
        let mut out: Vec<Entry> = Vec::new();
        for k in self.0.keys().filter(|k| k.starts_with(&prefix)) {
            let rest = &k[prefix.len()..];
            let (name, is_dir) = match rest.find('/') {
                Some(i) => (&rest[..i], true),
                None => (rest, false),
            };
            if !out.iter().any(|e| e.name == name) {
                out.push(Entry {
                    name: name.to_string(),
                    is_dir,
                });
            }
        }
        out
    }
}
