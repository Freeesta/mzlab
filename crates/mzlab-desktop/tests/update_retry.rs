//! A stale or damaged download of `update::apply`: one retry with the manifest version, then `update.error.hash` with the detail; a republished site restarts the diff once.

use mzlab_desktop::update::{
    apply, apply_refetch, sha256_hex, Entry, Manifest, Source, UpdateError, MANIFEST,
};
use std::collections::BTreeMap;
use std::fs;
use std::sync::Mutex;

fn manifest(version: &str, files: &[(&str, &[u8])]) -> Manifest {
    Manifest {
        version: version.into(),
        desktop_min: "0.1.0".into(),
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

/// `get` answers `stale`, `get_versioned` answers `fresh`; the manifest is served by `get` when `files` has it.
struct Stale {
    stale: Vec<u8>,
    fresh: Vec<u8>,
    files: BTreeMap<String, Vec<u8>>,
    calls: Mutex<Vec<String>>,
}

impl Source for Stale {
    fn get(&self, rel: &str) -> Result<Vec<u8>, UpdateError> {
        self.calls.lock().unwrap().push(rel.to_string());
        Ok(self
            .files
            .get(rel)
            .cloned()
            .unwrap_or_else(|| self.stale.clone()))
    }
    fn retry_differs(&self, _version: &str) -> bool {
        true
    }
    fn get_versioned(&self, rel: &str, v: &str) -> Result<Vec<u8>, UpdateError> {
        self.calls.lock().unwrap().push(format!("{rel}?v={v}"));
        Ok(self.fresh.clone())
    }
}

fn stale(stale: &[u8], fresh: &[u8]) -> Stale {
    Stale {
        stale: stale.to_vec(),
        fresh: fresh.to_vec(),
        files: BTreeMap::new(),
        calls: Mutex::new(vec![]),
    }
}

#[test]
fn stale_file_is_fetched_again_with_the_version() {
    let tmp = tempfile::tempdir().unwrap();
    let local = manifest("a", &[("static/a.js", b"old")]);
    let remote = manifest("b", &[("static/a.js", b"new")]);
    let src = stale(b"old", b"new");
    assert_eq!(
        apply(tmp.path(), &local, &remote, "0.1.0", &src, |_| {}).unwrap(),
        1
    );
    assert_eq!(
        *src.calls.lock().unwrap(),
        ["static/a.js", "static/a.js?v=b"]
    );
    assert_eq!(
        fs::read(tmp.path().join("web/static/a.js")).unwrap(),
        b"new"
    );
}

#[test]
fn permanently_bad_file_keeps_web_and_names_the_file() {
    let tmp = tempfile::tempdir().unwrap();
    let data = tmp.path();
    fs::create_dir_all(data.join("web/static")).unwrap();
    fs::write(data.join("web/static/a.js"), b"old").unwrap();
    let local = manifest("a", &[("static/a.js", b"old")]);
    let remote = manifest("b", &[("static/a.js", b"new")]);
    let err = apply(data, &local, &remote, "0.1.0", &stale(b"xx", b"xx"), |_| {}).unwrap_err();
    assert_eq!(err.key, "update.error.hash");
    assert_eq!(
        err.detail,
        format!(
            "static/a.js: got 2/{}, want 3/{}, tries 2",
            &sha256_hex(b"xx")[..8],
            &sha256_hex(b"new")[..8]
        )
    );
    assert_eq!(fs::read(data.join("web/static/a.js")).unwrap(), b"old");
    assert!(!data.join("web.next").exists());
}

#[test]
fn republished_site_restarts_the_diff_once() {
    let tmp = tempfile::tempdir().unwrap();
    let local = manifest("a", &[("static/a.js", b"old")]);
    let seen = manifest("b", &[("static/a.js", b"new")]);
    let fresh = manifest("c", &[("static/a.js", b"newer")]);
    let mut src = stale(b"zz", b"zz");
    src.files.insert("static/a.js".into(), b"newer".to_vec());
    src.files
        .insert(MANIFEST.into(), serde_json::to_vec(&fresh).unwrap());
    assert_eq!(
        apply_refetch(tmp.path(), &local, &seen, "0.1.0", &src, |_| {}).unwrap(),
        1
    );
    assert_eq!(
        fs::read(tmp.path().join("web/static/a.js")).unwrap(),
        b"newer"
    );
    let manifest_gets = |s: &Stale| {
        s.calls
            .lock()
            .unwrap()
            .iter()
            .filter(|c| *c == MANIFEST)
            .count()
    };
    assert_eq!(manifest_gets(&src), 1, "exactly one manifest refetch");
    // the same manifest again: the hash error comes back, one refetch, no second diff, web/ as it was
    let mut same = stale(b"bad", b"bad");
    same.files
        .insert(MANIFEST.into(), serde_json::to_vec(&seen).unwrap());
    let data = tempfile::tempdir().unwrap();
    fs::create_dir_all(data.path().join("web/static")).unwrap();
    fs::write(data.path().join("web/static/a.js"), b"old").unwrap();
    let err = apply_refetch(data.path(), &local, &seen, "0.1.0", &same, |_| {}).unwrap_err();
    assert_eq!(err.key, "update.error.hash");
    assert_eq!(manifest_gets(&same), 1);
    assert_eq!(
        fs::read(data.path().join("web/static/a.js")).unwrap(),
        b"old"
    );
    assert!(!data.path().join("web.next").exists());
}

/// A source with no distinct retry (or a version it cannot send): one try, and the detail says so.
struct Plain;
impl Source for Plain {
    fn get(&self, _: &str) -> Result<Vec<u8>, UpdateError> {
        Ok(b"xx".to_vec())
    }
}

#[test]
fn no_cache_buster_is_not_reported_as_a_second_try() {
    let tmp = tempfile::tempdir().unwrap();
    let local = manifest("a", &[("static/a.js", b"old")]);
    let remote = manifest("b", &[("static/a.js", b"new")]);
    let err = apply(tmp.path(), &local, &remote, "0.1.0", &Plain, |_| {}).unwrap_err();
    assert!(
        err.detail
            .ends_with("tries 1, no cache-buster for this version"),
        "{}",
        err.detail
    );
}
