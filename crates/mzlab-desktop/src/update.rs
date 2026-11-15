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
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[path = "update_http.rs"]
mod http;
#[path = "update_serve.rs"]
mod serve;
pub use http::Http;

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

/// Where the files come from (the site, or a map in the tests). `Sync`: the worker threads of `apply` share it.
pub trait Source: Sync {
    fn get(&self, rel: &str) -> Result<Vec<u8>, UpdateError>;
    /// The same file asked again with the manifest version as cache buster (a stale CDN/proxy copy); sources with no cache ignore it.
    fn get_versioned(&self, rel: &str, _version: &str) -> Result<Vec<u8>, UpdateError> {
        self.get(rel)
    }
    /// Does `get_versioned` ask for something other than `get` (so that a second try is worth making)?
    fn retry_differs(&self, _version: &str) -> bool {
        false
    }
}

pub fn parse_manifest(bytes: &[u8]) -> Result<Manifest, UpdateError> {
    let m: Manifest = serde_json::from_slice(bytes)
        .map_err(|e| UpdateError::new("update.error.manifest", e.to_string()))?;
    let bad = |d: String| Err(UpdateError::new("update.error.manifest", d));
    if !plain_version(&m.desktop_min) {
        return bad(format!("desktop_min {}", m.desktop_min));
    }
    // names the rest of the updater compares by exact string: no key may fold onto them by case only
    let fixed = ["index.html", CHECK, MANIFEST]
        .into_iter()
        .chain(NATIVE.iter().copied());
    let fixed: Vec<&str> = fixed.collect();
    // `check` and `download_all` add the sizes up unchecked
    if m.files
        .values()
        .try_fold(0u64, |t, e| t.checked_add(e.size))
        .is_none()
    {
        return bad("total size overflows u64".into());
    }
    let mut seen = std::collections::BTreeSet::new();
    for rel in m.files.keys() {
        safe_rel(rel)?;
        if !seen.insert(rel.to_ascii_lowercase()) {
            return bad(format!("two keys differ only by case: {rel}"));
        }
        if fixed
            .iter()
            .any(|f| rel.eq_ignore_ascii_case(f) && rel.as_str() != *f)
        {
            return bad(format!("key {rel} differs from a fixed name only by case"));
        }
    }
    Ok(m)
}

/// `\d+\.\d+\.\d+` with each part fitting a `u64` (what `build_site.py` writes); anything else would compare as 0 in `native_ok`.
fn plain_version(v: &str) -> bool {
    let parts: Vec<&str> = v.split('.').collect();
    parts.len() == 3
        && parts.iter().all(|p| {
            !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()) && p.parse::<u64>().is_ok()
        })
}

/// A relative path with plain components only (no `..`, no root, no drive, no backslash), in the one spelling the manifest uses:
/// no empty or `.` part (`a//b`, `a/./b`, `a/`), so that exact string comparisons (`NATIVE`, `index.html`) see the file that is written.
pub fn safe_rel(rel: &str) -> Result<(), UpdateError> {
    plain_rel(rel)?;
    if rel.split('/').any(|p| p.is_empty() || p == ".") {
        return Err(UpdateError::new(
            "update.error.manifest",
            format!("non-canonical path {rel}"),
        ));
    }
    Ok(())
}

/// `safe_rel` without the canonical-form rule: a request path of the window (`/static//a.js`) is normalised by `override_rel` after this.
fn plain_rel(rel: &str) -> Result<(), UpdateError> {
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

/// Download threads: the time of an update is the latency of hundreds of small requests, not their bytes.
const THREADS: usize = 8;

/// Fetches one changed file, checks size and SHA-256 and writes it into `next` (`index.html` is patched first).
/// A size or hash mismatch is retried once with `?v=<manifest version>` (a stale cached copy); a second one is `update.error.hash` with what was got and wanted.
fn fetch_one(
    next: &Path,
    rel: &str,
    want: &Entry,
    version: &str,
    src: &dyn Source,
) -> Result<u64, UpdateError> {
    let ok = |b: &[u8]| b.len() as u64 == want.size && sha256_hex(b) == want.sha256;
    let mut body = src.get(rel)?;
    if !ok(&body) {
        let retry = src.retry_differs(version);
        if retry {
            body = src.get_versioned(rel, version)?;
        }
        if !ok(&body) {
            let (h, w) = (sha256_hex(&body), &want.sha256);
            let tries = if retry {
                "tries 2"
            } else {
                "tries 1, no cache-buster for this version"
            };
            return Err(UpdateError::new(
                "update.error.hash",
                format!(
                    "{rel}: got {}/{}, want {}/{}, {tries}",
                    body.len(),
                    h.get(..8).unwrap_or(&h),
                    want.size,
                    w.get(..8).unwrap_or(w)
                ),
            ));
        }
    }
    if rel == "index.html" {
        body = patch_index(&String::from_utf8_lossy(&body))?.into_bytes();
    }
    write_file(next, rel, &body)?;
    Ok(want.size)
}

/// Downloads `files` with up to `THREADS` workers; `on` is called from this thread after each file.
/// At the first error no new file is started, the requests already in flight finish and the error is returned.
fn download_all(
    next: &Path,
    files: &[String],
    remote: &Manifest,
    src: &dyn Source,
    on: &mut impl FnMut(Progress),
) -> Result<(), UpdateError> {
    let total_bytes: u64 = files.iter().map(|f| remote.files[f].size).sum();
    let (cursor, abort) = (AtomicUsize::new(0), AtomicBool::new(false));
    let (tx, rx) = mpsc::channel::<Result<u64, UpdateError>>();
    let (mut done, mut bytes, mut first_err) = (0usize, 0u64, None);
    std::thread::scope(|scope| {
        for _ in 0..THREADS.min(files.len()) {
            let tx = tx.clone();
            let (cursor, abort) = (&cursor, &abort);
            scope.spawn(move || loop {
                let i = cursor.fetch_add(1, Ordering::Relaxed);
                if i >= files.len() || abort.load(Ordering::Relaxed) {
                    break;
                }
                let r = fetch_one(
                    next,
                    &files[i],
                    &remote.files[&files[i]],
                    &remote.version,
                    src,
                );
                if r.is_err() {
                    abort.store(true, Ordering::Relaxed);
                }
                if tx.send(r).is_err() {
                    break;
                }
            });
        }
        drop(tx);
        for r in rx {
            match r {
                Ok(size) => {
                    done += 1;
                    bytes += size;
                    on(Progress {
                        done,
                        total: files.len(),
                        bytes,
                        total_bytes,
                    });
                }
                Err(e) => {
                    first_err.get_or_insert(e);
                }
            }
        }
    });
    first_err.map_or(Ok(()), Err)
}

/// Deletes every file of `root` that `remote` does not list (except `check.json` and the manifest), then the folders left empty:
/// a file removed from the site must not stay reachable through `serve_override`.
fn prune(root: &Path, remote: &Manifest) -> Result<(), UpdateError> {
    let mut names = Vec::new();
    walk(root, root, &mut names).map_err(UpdateError::io)?;
    for n in names {
        if n != CHECK && n != MANIFEST && !remote.files.contains_key(&n) {
            fs::remove_file(root.join(&n)).map_err(UpdateError::io)?;
        }
    }
    remove_empty_dirs(root).map_err(UpdateError::io)
}

fn remove_empty_dirs(dir: &Path) -> std::io::Result<()> {
    for e in fs::read_dir(dir)? {
        let e = e?;
        if e.file_type()?.is_dir() {
            remove_empty_dirs(&e.path())?;
            if fs::read_dir(e.path())?.next().is_none() {
                fs::remove_dir(e.path())?;
            }
        }
    }
    Ok(())
}

/// Fills `next` with the new files, drops the ones `remote` no longer lists, writes the new manifest and the hashes of the whole folder.
fn build_next(
    next: &Path,
    files: &[String],
    remote: &Manifest,
    src: &dyn Source,
    on: &mut impl FnMut(Progress),
) -> Result<(), UpdateError> {
    download_all(next, files, remote, src, on)?;
    prune(next, remote)?;
    let json = serde_json::to_vec(remote).map_err(UpdateError::io)?;
    write_file(next, MANIFEST, &json)?;
    write_check(next)
}

/// Replaces `web` with `next`; if the last rename fails the old folder comes back.
fn swap_dirs(web: &Path, next: &Path, old: &Path) -> Result<(), UpdateError> {
    let _ = fs::remove_dir_all(old);
    if web.is_dir() {
        fs::rename(web, old).map_err(UpdateError::io)?;
    }
    if let Err(e) = fs::rename(next, web) {
        let _ = fs::rename(old, web);
        return Err(UpdateError::io(e));
    }
    let _ = fs::remove_dir_all(old);
    Ok(())
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
    // the current folder is the base only if it is the one `local` describes (not when `local` fell back to the embedded manifest)
    let base = fs::read(web.join(MANIFEST))
        .ok()
        .and_then(|b| parse_manifest(&b).ok())
        .is_some_and(|m| m == *local);
    if base {
        copy_tree(&web, &next)?;
    } else {
        fs::create_dir_all(&next).map_err(UpdateError::io)?;
    }
    if let Err(e) = build_next(&next, &files, remote, src, &mut on) {
        let _ = fs::remove_dir_all(&next);
        return Err(e);
    }
    swap_dirs(&web, &next, &old)?;
    Ok(files.len())
}

/// `apply`, and on `update.error.hash` the manifest is fetched once more: if the site was republished meanwhile, the diff is redone once against the new one.
pub fn apply_refetch(
    data: &Path,
    local: &Manifest,
    remote: &Manifest,
    native: &str,
    src: &dyn Source,
    mut on: impl FnMut(Progress),
) -> Result<usize, UpdateError> {
    match apply(data, local, remote, native, src, &mut on) {
        Err(e) if e.key == "update.error.hash" => {
            let fresh = parse_manifest(&src.get(MANIFEST)?)?;
            if fresh == *remote {
                Err(e)
            } else {
                apply(data, local, &fresh, native, src, on)
            }
        }
        r => r,
    }
}

/// At start: finishes or undoes an interrupted swap, then sets `web/` aside so that the embedded copy is served when it is damaged,
/// its manifest does not parse, or it needs a newer native binary than this one (`native`, e.g. a folder left by a newer app).
pub fn recover(data: &Path, native: &str) {
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
    // `web.old` is the last good folder until it is back in place (a locked folder on Windows can make the rename fail)
    if web.is_dir() {
        let _ = fs::remove_dir_all(&old);
    }
    let usable = |w: &Path| {
        validate_web(w)
            && fs::read(w.join(MANIFEST))
                .ok()
                .and_then(|b| parse_manifest(&b).ok())
                .is_some_and(|m| native_ok(native, &m.desktop_min))
    };
    if web.is_dir() && !usable(&web) {
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

/// The file of the updated folder for a request path of the window, if it may be served (`update_serve.rs`); otherwise the embedded copy answers.
pub fn serve_override(web: &Path, req_path: &str) -> Option<(Vec<u8>, &'static str)> {
    serve::serve_override(web, req_path)
}

#[cfg(test)]
#[path = "update_tests.rs"]
mod tests;
