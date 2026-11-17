//! `mzlab-cli folder list DIR --json`: what «Open folder» lists in a pointed folder (`folder/1`). ONE scan for the CLI and the desktop app (the site's twin is
//! `CartPP.collect` in `mzlab/web/cartella-puntata.js`, proved equal on a synthetic tree by `tests/test_cartella_puntata.py`): the compatible files, plus the
//! directory-based formats the drop path reads (`CartCarica.load`): Agilent `.D` / Bruker `.d` folders and Waters `.raw` folders (with `_FUNC001.DAT`).

use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::UNIX_EPOCH;

use mzlab_core::json::Out;

use crate::Outcome;

pub const MAX_DEPTH: usize = 3;
pub const CAP: usize = 3000;

pub const HELP: &str = "mzlab-cli folder: what «Open folder» of the page and the app lists in a folder (folder/1), as JSON.

usage: mzlab-cli folder list DIR [--json] [--out PATH]

  --json         the only format (accepted so that the command reads like the others)
  --out PATH     write there instead of the standard output

The answer is {\"schema\":\"folder/1\", \"root\", \"max_depth\", \"cap\", \"truncated\", \"items\": [{path, name, dir, kind, bundle, members, size, mtime}]},
items in the order of their paths. kind: data (.mzML .raw .wiff and the folders below), method (.dam), library (.msp .mgf .mzlib .jsonl .ndjson), table (.csv .tsv),
scan (a .wiff.scan, listed beside its .wiff). bundle: null for a file; \"d\" for an Agilent .D or Bruker .d folder (any folder called *.d, whole or not: the
loader says what is missing), \"waters-raw\" for a *.raw folder that holds _FUNC001.DAT; members = files inside a bundle (null for a file); size = bytes of
the file or of all the members; mtime in ms since 1970 (latest member). Hidden names are skipped; depth 3, at most 3000 items (truncated tells).
A bundle is one row and is not entered. Exit codes: 0 done; 2 usage; 4 DIR missing or not a folder.";

/// One row of the list.
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    /// path relative to the folder, with `/`
    pub path: String,
    pub name: String,
    /// folder of the item relative to the root, empty or ending with `/`
    pub dir: String,
    pub kind: &'static str,
    /// `Some("d")` or `Some("waters-raw")` for a directory-based format
    pub bundle: Option<&'static str>,
    pub members: Option<u64>,
    pub size: u64,
    pub mtime: u64,
}

impl Item {
    /// The row as a JSON object (stable key order).
    pub fn to_json(&self) -> String {
        let mut o = Out(String::new());
        o.raw("{\"path\":").str(&self.path);
        o.raw(",\"name\":").str(&self.name);
        o.raw(",\"dir\":").str(&self.dir);
        o.raw(",\"kind\":").str(self.kind);
        o.raw(",\"bundle\":");
        match self.bundle {
            Some(b) => o.str(b),
            None => o.raw("null"),
        };
        o.raw(",\"members\":");
        match self.members {
            Some(n) => o.int(n as i64),
            None => o.raw("null"),
        };
        o.raw(",\"size\":").int(self.size as i64);
        o.raw(",\"mtime\":").int(self.mtime as i64);
        o.raw("}");
        o.0
    }
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

fn mtime_ms(m: &fs::Metadata) -> u64 {
    m.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_millis() as u64)
}

/// The directory-based format a folder is, by its name (as `AgilentD.locate` and `WatersRaw.locate` read the drop): `.d` of any case; `.raw` only with `_FUNC001.DAT` inside.
fn bundle_of(name: &str, dir: &Path) -> Option<&'static str> {
    let n = name.to_lowercase();
    if n.ends_with(".d") && n.len() > 2 {
        return Some("d");
    }
    if n.ends_with(".raw") && n.len() > 4 {
        let has = fs::read_dir(dir).ok()?.flatten().any(|e| {
            e.file_name()
                .to_string_lossy()
                .eq_ignore_ascii_case("_func001.dat")
        });
        return has.then_some("waters-raw");
    }
    None
}

/// Number of files, bytes and latest date of everything inside a bundle (hidden names skipped, as the loader does).
fn members(dir: &Path, acc: &mut (u64, u64, u64)) {
    for e in fs::read_dir(dir).into_iter().flatten().flatten() {
        if hidden(&e.file_name().to_string_lossy()) {
            continue;
        }
        let Ok(m) = e.metadata() else { continue };
        if m.is_dir() {
            members(&e.path(), acc);
        } else {
            *acc = (acc.0 + 1, acc.1 + m.len(), acc.2.max(mtime_ms(&m)));
        }
    }
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
            if let Some(b) = bundle_of(&name, &e.path()) {
                let mut acc = (0, 0, mtime_ms(&meta));
                members(&e.path(), &mut acc);
                out.push(Item {
                    path: format!("{rel}{name}"),
                    name,
                    dir: rel.to_string(),
                    kind: "data",
                    bundle: Some(b),
                    members: Some(acc.0),
                    size: acc.1,
                    mtime: acc.2,
                });
            } else if depth < MAX_DEPTH {
                walk(root, &format!("{rel}{name}/"), depth + 1, out);
            }
        } else if let Some(kind) = kind_of(&name) {
            out.push(Item {
                path: format!("{rel}{name}"),
                name,
                dir: rel.to_string(),
                kind,
                bundle: None,
                members: None,
                size: meta.len(),
                mtime: mtime_ms(&meta),
            });
        }
    }
}

/// The items of the folder, in the order of their paths.
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

/// `folder/1`: the answer of the command.
pub fn to_json(root: &Path, items: &[Item]) -> String {
    let name = root.file_name().map_or_else(
        || root.to_string_lossy().into_owned(),
        |n| n.to_string_lossy().into_owned(),
    );
    let mut o = Out(String::from("{\"schema\":\"folder/1\",\"root\":"));
    o.str(&name);
    o.raw(&format!(
        ",\"max_depth\":{MAX_DEPTH},\"cap\":{CAP},\"truncated\":{},\"items\":[",
        items.len() >= CAP
    ));
    o.raw(
        &items
            .iter()
            .map(Item::to_json)
            .collect::<Vec<_>>()
            .join(","),
    );
    o.raw("]}\n");
    o.0
}

pub fn run(args: Vec<String>) -> Outcome {
    let (mut out, mut dir, mut sub) = (None, None, None);
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--help" | "-h" => return Outcome::ok(format!("{HELP}\n").into_bytes()),
            "--json" => {}
            "--out" => match it.next() {
                Some(p) => out = Some(PathBuf::from(p)),
                None => return Outcome::fail("--out needs a path"),
            },
            o if o.starts_with('-') => return Outcome::fail(&format!("unknown argument {o}")),
            o if sub.is_none() => sub = Some(o.to_string()),
            o if dir.is_none() => dir = Some(PathBuf::from(o)),
            o => return Outcome::fail(&format!("unexpected argument {o}")),
        }
    }
    if sub.as_deref() != Some("list") {
        return Outcome::fail("usage: mzlab-cli folder list DIR [--json] [--out PATH]");
    }
    let Some(dir) = dir else {
        return Outcome::fail("folder list needs a DIR");
    };
    if !dir.is_dir() {
        return Outcome {
            code: 4,
            stdout: Vec::new(),
            stderr: format!("error: {} is not a folder\n", dir.display()),
        };
    }
    crate::emit(to_json(&dir, &list(&dir)), &out)
}

#[cfg(test)]
#[path = "folder_cmd_tests.rs"]
mod tests;
