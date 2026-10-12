//! The folder of the data pointed at by the user (desktop app): the list of the compatible files it holds, which the page shows in grey until one is opened.
//! Same kinds as `mzlab/web/cartella-puntata.js` (`kindOf`): data (mzML, raw, wiff), method (.dam), library (.msp .mgf .mzlib .jsonl .ndjson), table (.csv .tsv);
//! a `.wiff.scan` is listed as `scan` (the page pairs it with its `.wiff`). Hidden names are skipped; depth and number are limited. Only paths relative to the
//! folder travel to the page; `resolve` turns one back into a real path and refuses `..` and absolute paths.

use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::Serialize;

pub const MAX_DEPTH: usize = 3;
pub const CAP: usize = 3000;

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct Item {
    /// path relative to the folder, with `/`
    pub path: String,
    pub name: String,
    /// folder of the file relative to the root, empty or ending with `/`
    pub dir: String,
    pub kind: &'static str,
    pub size: u64,
    /// milliseconds since 1970, like `File.lastModified`
    pub mtime: u64,
}

pub fn kind_of(name: &str) -> Option<&'static str> {
    let n = name.to_lowercase();
    let ends = |s: &[&str]| s.iter().any(|e| n.ends_with(e));
    if n.ends_with(".wiff.scan") {
        Some("scan")
    } else if ends(&[".mzml", ".raw", ".wiff"]) {
        Some("data")
    } else if ends(&[".dam"]) {
        Some("method")
    } else if ends(&[".msp", ".mgf", ".mzlib", ".jsonl", ".ndjson"]) {
        Some("library")
    } else if ends(&[".csv", ".tsv"]) {
        Some("table")
    } else {
        None
    }
}

fn hidden(name: &str) -> bool {
    name.starts_with('.') || name == "__MACOSX"
}

fn walk(root: &Path, rel: &str, depth: usize, out: &mut Vec<Item>) {
    let Ok(rd) = fs::read_dir(root.join(rel)) else {
        return;
    };
    let mut entries: Vec<_> = rd.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        if out.len() >= CAP {
            return;
        }
        let name = e.file_name().to_string_lossy().into_owned();
        if hidden(&name) {
            continue;
        }
        let Ok(meta) = e.metadata() else { continue };
        if meta.is_dir() {
            if depth < MAX_DEPTH {
                walk(root, &format!("{rel}{name}/"), depth + 1, out);
            }
        } else if let Some(kind) = kind_of(&name) {
            let mtime = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_millis() as u64);
            out.push(Item {
                path: format!("{rel}{name}"),
                name,
                dir: rel.to_string(),
                kind,
                size: meta.len(),
                mtime,
            });
        }
    }
}

/// The compatible files of the folder, in the order of their paths.
pub fn list(root: &Path) -> Vec<Item> {
    let mut out = Vec::new();
    walk(root, "", 0, &mut out);
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

/// A relative path of the list as a real path inside `root`; None for `..`, an absolute path or an empty one.
pub fn resolve(root: &Path, rel: &str) -> Option<PathBuf> {
    let p = Path::new(rel);
    if rel.is_empty() || !p.components().all(|c| matches!(c, Component::Normal(_))) {
        return None;
    }
    Some(root.join(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds() {
        assert_eq!(kind_of("A.MZML"), Some("data"));
        assert_eq!(kind_of("s.wiff"), Some("data"));
        assert_eq!(kind_of("s.wiff.scan"), Some("scan"));
        assert_eq!(kind_of("m.DAM"), Some("method"));
        assert_eq!(kind_of("l.mzlib"), Some("library"));
        assert_eq!(kind_of("t.csv"), Some("table"));
        assert_eq!(kind_of("notes.txt"), None);
        assert_eq!(kind_of("x.json"), None);
    }

    #[test]
    fn list_is_recursive_skips_hidden_and_follows_changes() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        for f in [
            "a.mzML",
            "note.txt",
            "sub/w.wiff",
            "sub/w.wiff.scan",
            "sub/m.dam",
            "sub/.git/x.mzML",
            ".cache/y.mzML",
            "a/b/c/d/deep.mzML",
        ] {
            let p = r.join(f);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, b"x").unwrap();
        }
        let got: Vec<_> = list(r)
            .into_iter()
            .map(|i| format!("{}:{}", i.path, i.kind))
            .collect();
        assert_eq!(
            got,
            [
                "a.mzML:data",
                "sub/m.dam:method",
                "sub/w.wiff:data",
                "sub/w.wiff.scan:scan"
            ]
        );
        fs::write(r.join("new.raw"), b"xy").unwrap();
        let again = list(r);
        assert_eq!(again.len(), 5);
        assert_eq!(again.iter().find(|i| i.name == "new.raw").unwrap().size, 2);
        assert_eq!(again[0].dir, "");
    }

    #[test]
    fn resolve_refuses_escapes() {
        let r = Path::new("/data");
        assert_eq!(
            resolve(r, "sub/a.mzML"),
            Some(PathBuf::from("/data/sub/a.mzML"))
        );
        for bad in ["", "../x.mzML", "sub/../../x", "/etc/passwd"] {
            assert_eq!(resolve(r, bad), None, "{bad}");
        }
    }
}
