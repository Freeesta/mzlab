//! Updates of the web part of the desktop app (pure logic, no window; `main.rs` only wires it to commands).
//!
//! The published site (`https://freeesta.github.io/mzlab/`) carries `static/manifest.json`: for every file its SHA-256 and size, the
//! commit as `version` and `desktop_min`, the oldest native binary the web part works with. The app compares that manifest with its
//! own copy (the one in `<app data>/web/static/manifest.json`, else the one embedded in the binary), downloads ONLY the files that
//! differ, checks each SHA-256, builds the complete new folder next to the old one and swaps the two in a single rename. The protocol
//! of the window serves `<app data>/web/` first and the embedded copy second (`serve_override`), so a damaged folder (`validate_web`)
//! is set aside and the embedded copy takes over. Network code lives only here, in the crate of the window.

use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Where the published site lives; the only host the program talks to.
pub const SITE: &str = "https://freeesta.github.io/mzlab/";
/// Manifest path, relative to the site root.
pub const MANIFEST: &str = "static/manifest.json";
/// Where the installers of the native part are found.
pub const RELEASES: &str = "https://github.com/Freeesta/mzlab/actions/workflows/desktop.yml";
/// File of `<app data>/web/` with the hash of every file as it is on the disk (checked at start).
pub const CHECK: &str = "check.json";
/// Files that belong to the native part: they come with the binary and are never downloaded.
const NATIVE: [&str; 3] = ["static/rust-worker.js", "static/desktop-shim.js", "sw.js"];

const CSP_EXTRA: &str = " mzlab: http://mzlab.localhost ipc: http://ipc.localhost";
const BROWSER_TAG: &str = "<script src=\"static/browser.js\"></script>";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub sha256: String,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub version: String,
    pub desktop_min: String,
    pub files: BTreeMap<String, Entry>,
}

/// An error for the page: a key of `lang/` plus a detail for the log; never a sentence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateError {
    pub key: &'static str,
    pub detail: String,
}

impl UpdateError {
    fn new(key: &'static str, detail: impl Into<String>) -> Self {
        UpdateError {
            key,
            detail: detail.into(),
        }
    }
    fn io(e: impl std::fmt::Display) -> Self {
        Self::new("update.error.io", e.to_string())
    }
    pub fn to_json(&self) -> String {
        serde_json::json!({ "status": "error", "error_key": self.key, "detail": self.detail })
            .to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum Check {
    /// Nothing to download.
    Uptodate { version: String },
    /// `files` changed files, `bytes` to download.
    Available {
        version: String,
        files: usize,
        bytes: u64,
    },
    /// The new web part needs a newer native binary.
    Newapp { min: String, url: &'static str },
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Progress {
    pub done: usize,
    pub total: usize,
    pub bytes: u64,
    pub total_bytes: u64,
}

/// Where the files come from (the site, or a map in the tests).
pub trait Source {
    fn get(&self, rel: &str) -> Result<Vec<u8>, UpdateError>;
}

/// The published site over HTTPS: no redirects (the host cannot change), time limits, size limit.
pub struct Http {
    agent: ureq::Agent,
    base: String,
}

impl Http {
    pub fn new() -> Self {
        Self::with_base(SITE)
    }
    pub fn with_base(base: &str) -> Self {
        let agent = ureq::AgentBuilder::new()
            .redirects(0)
            .timeout_connect(Duration::from_secs(10))
            .timeout_read(Duration::from_secs(60))
            .user_agent("mzlab-desktop")
            .build();
        Http {
            agent,
            base: base.to_string(),
        }
    }
}

impl Default for Http {
    fn default() -> Self {
        Self::new()
    }
}

impl Source for Http {
    fn get(&self, rel: &str) -> Result<Vec<u8>, UpdateError> {
        safe_rel(rel)?;
        let url = format!("{}{}", self.base, rel);
        let resp = self.agent.get(&url).call().map_err(|e| match e {
            ureq::Error::Status(code, _) => UpdateError::new("update.error.http", code.to_string()),
            ureq::Error::Transport(t) => UpdateError::new("update.error.offline", t.to_string()),
        })?;
        if resp.status() != 200 {
            return Err(UpdateError::new(
                "update.error.http",
                resp.status().to_string(),
            )); // with no redirects a 3xx comes back as a response
        }
        let mut body = Vec::new();
        resp.into_reader()
            .take(600 << 20)
            .read_to_end(&mut body)
            .map_err(|e| UpdateError::new("update.error.offline", e.to_string()))?;
        Ok(body)
    }
}

pub fn parse_manifest(bytes: &[u8]) -> Result<Manifest, UpdateError> {
    let m: Manifest = serde_json::from_slice(bytes)
        .map_err(|e| UpdateError::new("update.error.manifest", e.to_string()))?;
    for rel in m.files.keys() {
        safe_rel(rel)?;
    }
    Ok(m)
}

/// A relative path with plain components only (no `..`, no root, no drive, no backslash).
pub fn safe_rel(rel: &str) -> Result<(), UpdateError> {
    let bad = rel.is_empty()
        || rel.contains('\\')
        || rel.contains(':')
        || Path::new(rel)
            .components()
            .any(|c| !matches!(c, Component::Normal(_)));
    if bad {
        Err(UpdateError::new(
            "update.error.manifest",
            format!("unsafe path {rel}"),
        ))
    } else {
        Ok(())
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn triple(v: &str) -> [u64; 3] {
    let mut out = [0u64; 3];
    for (i, p) in v.split('.').take(3).enumerate() {
        out[i] = p
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse()
            .unwrap_or(0);
    }
    out
}

/// True if the native binary (`native`, e.g. `0.1.0`) is at least `min`.
pub fn native_ok(native: &str, min: &str) -> bool {
    triple(native) >= triple(min)
}

/// Files of `remote` that are new or different from `local`, without the ones that come with the binary.
pub fn diff(local: &Manifest, remote: &Manifest) -> Vec<String> {
    remote
        .files
        .iter()
        .filter(|(rel, _)| !NATIVE.contains(&rel.as_str()) && rel.as_str() != MANIFEST)
        .filter(|(rel, e)| local.files.get(*rel).is_none_or(|l| l.sha256 != e.sha256))
        .map(|(rel, _)| rel.clone())
        .collect()
}

/// Compares the manifests. `native` is the version of this binary.
pub fn check(local: &Manifest, remote: &Manifest, native: &str) -> Check {
    if !native_ok(native, &remote.desktop_min) {
        return Check::Newapp {
            min: remote.desktop_min.clone(),
            url: RELEASES,
        };
    }
    let files = diff(local, remote);
    if files.is_empty() {
        return Check::Uptodate {
            version: remote.version.clone(),
        };
    }
    let bytes = files.iter().map(|f| remote.files[f].size).sum();
    Check::Available {
        version: remote.version.clone(),
        files: files.len(),
        bytes,
    }
}

/// The page of the desktop is the page of the site with two changes (the same ones `prepare.py` makes).
pub fn patch_index(html: &str) -> Result<String, UpdateError> {
    let key = "connect-src 'self'";
    let at = html
        .find(key)
        .ok_or_else(|| UpdateError::new("update.error.manifest", "connect-src not found"))?;
    let mut out = String::with_capacity(html.len() + 200);
    out.push_str(&html[..at + key.len()]);
    out.push_str(CSP_EXTRA);
    out.push_str(&html[at + key.len()..]);
    if !out.contains(BROWSER_TAG) {
        return Err(UpdateError::new(
            "update.error.manifest",
            "browser.js tag not found",
        ));
    }
    Ok(out.replacen(
        BROWSER_TAG,
        &format!("<script src=\"static/desktop-shim.js\"></script>\n{BROWSER_TAG}"),
        1,
    ))
}

fn copy_tree(from: &Path, to: &Path) -> Result<(), UpdateError> {
    fs::create_dir_all(to).map_err(UpdateError::io)?;
    for e in fs::read_dir(from).map_err(UpdateError::io)? {
        let e = e.map_err(UpdateError::io)?;
        let dst = to.join(e.file_name());
        if e.file_type().map_err(UpdateError::io)?.is_dir() {
            copy_tree(&e.path(), &dst)?;
        } else {
            fs::copy(e.path(), &dst).map_err(UpdateError::io)?;
        }
    }
    Ok(())
}

fn write_file(root: &Path, rel: &str, bytes: &[u8]) -> Result<(), UpdateError> {
    let p = root.join(rel);
    if let Some(d) = p.parent() {
        fs::create_dir_all(d).map_err(UpdateError::io)?;
    }
    fs::write(p, bytes).map_err(UpdateError::io)
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) -> std::io::Result<()> {
    for e in fs::read_dir(dir)? {
        let e = e?;
        if e.file_type()?.is_dir() {
            walk(root, &e.path(), out)?;
        } else if let Ok(r) = e.path().strip_prefix(root) {
            out.push(r.to_string_lossy().replace('\\', "/"));
        }
    }
    Ok(())
}

/// Hashes of the files as they are on the disk (what `validate_web` checks).
fn write_check(root: &Path) -> Result<(), UpdateError> {
    let mut names = Vec::new();
    walk(root, root, &mut names).map_err(UpdateError::io)?;
    let mut map = BTreeMap::new();
    for n in names.into_iter().filter(|n| n != CHECK) {
        let b = fs::read(root.join(&n)).map_err(UpdateError::io)?;
        map.insert(
            n,
            Entry {
                sha256: sha256_hex(&b),
                size: b.len() as u64,
            },
        );
    }
    let json = serde_json::to_vec(&map).map_err(UpdateError::io)?;
    write_file(root, CHECK, &json)
}

/// Downloads the changed files, verifies them and swaps `<data>/web` for the new folder. Returns the number of files downloaded.
/// Nothing visible changes until the last rename: on any error the current folder stays as it was.
pub fn apply(
    data: &Path,
    local: &Manifest,
    remote: &Manifest,
    native: &str,
    src: &dyn Source,
    mut on: impl FnMut(Progress),
) -> Result<usize, UpdateError> {
    if !native_ok(native, &remote.desktop_min) {
        return Err(UpdateError::new(
            "update.error.newapp",
            remote.desktop_min.clone(),
        ));
    }
    let files = diff(local, remote);
    let (web, next, old) = (
        data.join("web"),
        data.join("web.next"),
        data.join("web.old"),
    );
    let _ = fs::remove_dir_all(&next);
    fs::create_dir_all(data).map_err(UpdateError::io)?;
    if web.is_dir() {
        copy_tree(&web, &next)?;
    } else {
        fs::create_dir_all(&next).map_err(UpdateError::io)?;
    }
    let mut run = || -> Result<(), UpdateError> {
        let total_bytes: u64 = files.iter().map(|f| remote.files[f].size).sum();
        let mut bytes = 0u64;
        for (i, rel) in files.iter().enumerate() {
            let want = &remote.files[rel];
            let mut body = src.get(rel)?;
            if body.len() as u64 != want.size || sha256_hex(&body) != want.sha256 {
                return Err(UpdateError::new("update.error.hash", rel.clone()));
            }
            if rel == "index.html" {
                body = patch_index(&String::from_utf8_lossy(&body))?.into_bytes();
            }
            write_file(&next, rel, &body)?;
            bytes += want.size;
            on(Progress {
                done: i + 1,
                total: files.len(),
                bytes,
                total_bytes,
            });
        }
        let json = serde_json::to_vec(remote).map_err(UpdateError::io)?;
        write_file(&next, MANIFEST, &json)?;
        write_check(&next)
    };
    if let Err(e) = run() {
        let _ = fs::remove_dir_all(&next);
        return Err(e);
    }
    let _ = fs::remove_dir_all(&old);
    if web.is_dir() {
        fs::rename(&web, &old).map_err(UpdateError::io)?;
    }
    if let Err(e) = fs::rename(&next, &web) {
        let _ = fs::rename(&old, &web);
        return Err(UpdateError::io(e));
    }
    let _ = fs::remove_dir_all(&old);
    Ok(files.len())
}

/// At start: finishes or undoes an interrupted swap, then sets a damaged `web/` aside so that the embedded copy is served.
pub fn recover(data: &Path) {
    let (web, next, old, bad) = (
        data.join("web"),
        data.join("web.next"),
        data.join("web.old"),
        data.join("web.bad"),
    );
    let _ = fs::remove_dir_all(&next);
    if !web.is_dir() && old.is_dir() {
        let _ = fs::rename(&old, &web);
    }
    let _ = fs::remove_dir_all(&old);
    if web.is_dir() && !validate_web(&web) {
        let _ = fs::remove_dir_all(&bad);
        if fs::rename(&web, &bad).is_err() {
            let _ = fs::remove_dir_all(&web);
        }
    }
}

/// Every file listed in `check.json` exists and has its hash. A folder without `check.json` is not valid.
pub fn validate_web(web: &Path) -> bool {
    let Ok(raw) = fs::read(web.join(CHECK)) else {
        return false;
    };
    let Ok(map) = serde_json::from_slice::<BTreeMap<String, Entry>>(&raw) else {
        return false;
    };
    map.iter().all(|(rel, e)| {
        safe_rel(rel).is_ok()
            && fs::read(web.join(rel))
                .is_ok_and(|b| b.len() as u64 == e.size && sha256_hex(&b) == e.sha256)
    })
}

/// The manifest of what the app shows now: the updated folder if there is one, else the embedded one.
pub fn local_manifest(data: &Path, embedded: &[u8]) -> Result<Manifest, UpdateError> {
    match fs::read(data.join("web").join(MANIFEST)) {
        Ok(b) => parse_manifest(&b).or_else(|_| parse_manifest(embedded)),
        Err(_) => parse_manifest(embedded),
    }
}

fn mime(rel: &str) -> &'static str {
    match rel
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" | "webmanifest" => "application/json",
        "wasm" => "application/wasm",
        "zip" => "application/zip",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "txt" | "md" => "text/plain; charset=utf-8",
        "mzml" | "xml" => "application/xml",
        _ => "application/octet-stream",
    }
}

/// The file of the updated folder for a request path of the window (`/` = the page), if it is there. Anything else (and any path that
/// tries to leave the folder) returns `None` and the embedded copy answers.
pub fn serve_override(web: &Path, req_path: &str) -> Option<(Vec<u8>, &'static str)> {
    let rel = req_path.trim_start_matches('/');
    let rel = if rel.is_empty() { "index.html" } else { rel };
    if rel == CHECK || safe_rel(rel).is_err() {
        return None;
    }
    let p: PathBuf = web.join(rel);
    let body = fs::read(&p).ok()?;
    Some((body, mime(rel)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    struct Mem(BTreeMap<String, Vec<u8>>, RefCell<Vec<String>>);
    impl Source for Mem {
        fn get(&self, rel: &str) -> Result<Vec<u8>, UpdateError> {
            self.1.borrow_mut().push(rel.to_string());
            self.0
                .get(rel)
                .cloned()
                .ok_or_else(|| UpdateError::new("update.error.http", "404"))
        }
    }

    fn manifest(version: &str, min: &str, files: &[(&str, &[u8])]) -> Manifest {
        Manifest {
            version: version.into(),
            desktop_min: min.into(),
            files: files
                .iter()
                .map(|(n, b)| {
                    (
                        n.to_string(),
                        Entry {
                            sha256: sha256_hex(b),
                            size: b.len() as u64,
                        },
                    )
                })
                .collect(),
        }
    }
    fn mem(files: &[(&str, &[u8])]) -> Mem {
        Mem(
            files
                .iter()
                .map(|(n, b)| (n.to_string(), b.to_vec()))
                .collect(),
            RefCell::new(vec![]),
        )
    }

    const INDEX: &str = "<meta content=\"default-src 'self'; connect-src 'self'; img-src data:\"><script src=\"static/browser.js\"></script>";

    #[test]
    fn sha256_known_value() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn native_version_compare() {
        assert!(native_ok("0.1.0", "0.1.0"));
        assert!(native_ok("0.2.0", "0.1.9"));
        assert!(native_ok("1.0.0", "0.9.9"));
        assert!(!native_ok("0.1.0", "0.1.1"));
        assert!(!native_ok("0.9.0", "0.10.0"));
    }

    #[test]
    fn diff_lists_only_changed_new_and_not_native() {
        let local = manifest(
            "a",
            "0.1.0",
            &[
                ("index.html", b"1"),
                ("static/a.js", b"a"),
                ("static/pyodide/p.wasm", b"big"),
            ],
        );
        let remote = manifest(
            "b",
            "0.1.0",
            &[
                ("index.html", b"2"),
                ("static/a.js", b"a"),
                ("static/pyodide/p.wasm", b"big"),
                ("static/n.js", b"n"),
                ("static/rust-worker.js", b"x"),
                ("sw.js", b"s"),
            ],
        );
        assert_eq!(
            diff(&local, &remote),
            vec!["index.html".to_string(), "static/n.js".to_string()]
        );
    }

    #[test]
    fn check_cases() {
        let a = manifest("a", "0.1.0", &[("static/a.js", b"a")]);
        assert_eq!(
            check(&a, &a, "0.1.0"),
            Check::Uptodate {
                version: "a".into()
            }
        );
        let b = manifest("b", "0.1.0", &[("static/a.js", b"aa")]);
        assert_eq!(
            check(&a, &b, "0.1.0"),
            Check::Available {
                version: "b".into(),
                files: 1,
                bytes: 2
            }
        );
        let c = manifest("c", "0.2.0", &[("static/a.js", b"aa")]);
        assert!(matches!(check(&a, &c, "0.1.0"), Check::Newapp { .. }));
    }

    #[test]
    fn rejects_unsafe_paths() {
        for bad in ["../x", "/etc/passwd", "a/../../b", "a\\b", "C:/x", ""] {
            assert!(safe_rel(bad).is_err(), "{bad}");
        }
        assert!(safe_rel("static/pyodide/pyodide.asm.wasm").is_ok());
    }

    #[test]
    fn patch_index_matches_prepare_py() {
        let p = patch_index(INDEX).unwrap();
        assert!(p.contains(
            "connect-src 'self' mzlab: http://mzlab.localhost ipc: http://ipc.localhost;"
        ));
        assert!(p.contains("<script src=\"static/desktop-shim.js\"></script>\n<script src=\"static/browser.js\"></script>"));
        assert!(patch_index("<html></html>").is_err());
    }

    #[test]
    fn apply_downloads_only_changed_files_and_swaps() {
        let tmp = tempfile::tempdir().unwrap();
        let data = tmp.path();
        let local = manifest(
            "a",
            "0.1.0",
            &[
                ("index.html", INDEX.as_bytes()),
                ("static/big.wasm", b"BIG"),
                ("static/a.js", b"old"),
            ],
        );
        let remote = manifest(
            "b",
            "0.1.0",
            &[
                ("index.html", INDEX.as_bytes()),
                ("static/big.wasm", b"BIG"),
                ("static/a.js", b"new"),
                ("static/n.js", b"n"),
            ],
        );
        let src = mem(&[
            ("static/a.js", b"new"),
            ("static/n.js", b"n"),
            ("static/big.wasm", b"BIG"),
        ]);
        let mut seen = vec![];
        let n = apply(data, &local, &remote, "0.1.0", &src, |p| {
            seen.push((p.done, p.total))
        })
        .unwrap();
        assert_eq!(n, 2);
        assert_eq!(
            *src.1.borrow(),
            vec!["static/a.js".to_string(), "static/n.js".to_string()]
        ); // the big file is not asked for
        assert_eq!(seen, vec![(1, 2), (2, 2)]);
        let web = data.join("web");
        assert_eq!(fs::read(web.join("static/a.js")).unwrap(), b"new");
        assert!(validate_web(&web));
        assert!(!data.join("web.next").exists() && !data.join("web.old").exists());
        // the manifest of the folder is the new one: a second check finds nothing to do
        let now = local_manifest(data, b"{}").unwrap();
        assert_eq!(
            check(&now, &remote, "0.1.0"),
            Check::Uptodate {
                version: "b".into()
            }
        );
        assert_eq!(serve_override(&web, "/static/a.js").unwrap().0, b"new");
        assert!(serve_override(&web, "/static/big.wasm").is_none());
    }

    #[test]
    fn index_is_patched_and_files_of_an_earlier_update_are_kept() {
        let tmp = tempfile::tempdir().unwrap();
        let data = tmp.path();
        let v1 = manifest(
            "1",
            "0.1.0",
            &[("index.html", INDEX.as_bytes()), ("static/a.js", b"a1")],
        );
        let v2 = manifest(
            "2",
            "0.1.0",
            &[("index.html", INDEX.as_bytes()), ("static/a.js", b"a2")],
        );
        let v3 = manifest(
            "3",
            "0.1.0",
            &[
                ("index.html", b"<x>"),
                ("static/a.js", b"a2"),
                ("static/b.js", b"b3"),
            ],
        );
        let emb = manifest(
            "0",
            "0.1.0",
            &[("index.html", b"<0>"), ("static/a.js", b"a0")],
        );
        apply(
            data,
            &emb,
            &v1,
            "0.1.0",
            &mem(&[("index.html", INDEX.as_bytes()), ("static/a.js", b"a1")]),
            |_| {},
        )
        .unwrap();
        let idx = String::from_utf8(serve_override(&data.join("web"), "/").unwrap().0).unwrap();
        assert!(idx.contains("desktop-shim.js"));
        apply(
            data,
            &v1,
            &v2,
            "0.1.0",
            &mem(&[("static/a.js", b"a2")]),
            |_| {},
        )
        .unwrap();
        assert!(
            idx.contains("mzlab:") && serve_override(&data.join("web"), "/index.html").is_some()
        );
        // v3 has an index.html without the markers: the update fails and nothing changes
        let err = apply(
            data,
            &v2,
            &v3,
            "0.1.0",
            &mem(&[("index.html", b"<x>"), ("static/b.js", b"b3")]),
            |_| {},
        )
        .unwrap_err();
        assert_eq!(err.key, "update.error.manifest");
        assert_eq!(fs::read(data.join("web/static/a.js")).unwrap(), b"a2");
        assert!(validate_web(&data.join("web")) && !data.join("web.next").exists());
    }

    #[test]
    fn corrupt_download_changes_nothing() {
        let tmp = tempfile::tempdir().unwrap();
        let data = tmp.path();
        let local = manifest("a", "0.1.0", &[("static/a.js", b"old")]);
        let remote = manifest("b", "0.1.0", &[("static/a.js", b"new")]);
        let err = apply(
            data,
            &local,
            &remote,
            "0.1.0",
            &mem(&[("static/a.js", b"nex")]),
            |_| {},
        )
        .unwrap_err();
        assert_eq!(err.key, "update.error.hash");
        assert!(!data.join("web").exists() && !data.join("web.next").exists());
    }

    #[test]
    fn offline_changes_nothing() {
        let tmp = tempfile::tempdir().unwrap();
        let local = manifest("a", "0.1.0", &[("static/a.js", b"old")]);
        let remote = manifest("b", "0.1.0", &[("static/a.js", b"new")]);
        struct Down;
        impl Source for Down {
            fn get(&self, _: &str) -> Result<Vec<u8>, UpdateError> {
                Err(UpdateError::new("update.error.offline", "no route"))
            }
        }
        let err = apply(tmp.path(), &local, &remote, "0.1.0", &Down, |_| {}).unwrap_err();
        assert_eq!(err.key, "update.error.offline");
        assert!(!tmp.path().join("web").exists());
    }

    #[test]
    fn newer_web_part_than_the_binary_is_refused() {
        let tmp = tempfile::tempdir().unwrap();
        let local = manifest("a", "0.1.0", &[("static/a.js", b"old")]);
        let remote = manifest("b", "0.3.0", &[("static/a.js", b"new")]);
        let err = apply(
            tmp.path(),
            &local,
            &remote,
            "0.1.0",
            &mem(&[("static/a.js", b"new")]),
            |_| {},
        )
        .unwrap_err();
        assert_eq!(err.key, "update.error.newapp");
    }

    #[test]
    fn damaged_folder_is_set_aside_and_embedded_copy_takes_over() {
        let tmp = tempfile::tempdir().unwrap();
        let data = tmp.path();
        let local = manifest("a", "0.1.0", &[("static/a.js", b"old")]);
        let remote = manifest("b", "0.1.0", &[("static/a.js", b"new")]);
        apply(
            data,
            &local,
            &remote,
            "0.1.0",
            &mem(&[("static/a.js", b"new")]),
            |_| {},
        )
        .unwrap();
        fs::write(data.join("web/static/a.js"), b"corrupt").unwrap();
        assert!(!validate_web(&data.join("web")));
        recover(data);
        assert!(!data.join("web").exists());
        assert!(serve_override(&data.join("web"), "/static/a.js").is_none());
        // the manifest falls back to the embedded one, so the next check downloads the file again
        let emb = serde_json::to_vec(&local).unwrap();
        assert_eq!(local_manifest(data, &emb).unwrap(), local);
    }

    #[test]
    fn interrupted_swap_is_finished_or_undone() {
        let tmp = tempfile::tempdir().unwrap();
        let data = tmp.path();
        let local = manifest("a", "0.1.0", &[("static/a.js", b"old")]);
        let remote = manifest("b", "0.1.0", &[("static/a.js", b"new")]);
        apply(
            data,
            &local,
            &remote,
            "0.1.0",
            &mem(&[("static/a.js", b"new")]),
            |_| {},
        )
        .unwrap();
        fs::rename(data.join("web"), data.join("web.old")).unwrap(); // the app died between the two renames
        fs::create_dir(data.join("web.next")).unwrap();
        recover(data);
        assert!(
            validate_web(&data.join("web"))
                && !data.join("web.old").exists()
                && !data.join("web.next").exists()
        );
    }

    #[test]
    fn serve_override_never_leaves_the_folder() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir(tmp.path().join("web")).unwrap();
        fs::write(tmp.path().join("secret.txt"), b"s").unwrap();
        assert!(serve_override(&tmp.path().join("web"), "/../secret.txt").is_none());
        assert!(serve_override(&tmp.path().join("web"), "/check.json").is_none());
    }

    #[test]
    fn http_source_reads_a_file_and_reports_offline() {
        use std::io::{Read, Write};
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let h = std::thread::spawn(move || {
            for _ in 0..2 {
                let (mut c, _) = l.accept().unwrap();
                let mut buf = [0u8; 2048];
                let n = c.read(&mut buf).unwrap();
                let first = String::from_utf8_lossy(&buf[..n])
                    .lines()
                    .next()
                    .unwrap_or("")
                    .to_string();
                let resp = if first.contains("static/a.js") {
                    "HTTP/1.1 200 OK\r\nContent-Length: 3\r\nConnection: close\r\n\r\nabc"
                } else {
                    "HTTP/1.1 302 Found\r\nLocation: http://elsewhere/\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                };
                c.write_all(resp.as_bytes()).unwrap();
            }
        });
        let src = Http::with_base(&format!("http://127.0.0.1:{port}/"));
        assert_eq!(src.get("static/a.js").unwrap(), b"abc");
        // a redirect is not followed: the host cannot change
        assert_eq!(
            src.get("static/other.js").unwrap_err().key,
            "update.error.http"
        );
        h.join().unwrap();
        let down = Http::with_base(&format!("http://127.0.0.1:{port}/"));
        assert_eq!(
            down.get("static/a.js").unwrap_err().key,
            "update.error.offline"
        );
        assert!(down.get("../x").is_err());
    }
}
