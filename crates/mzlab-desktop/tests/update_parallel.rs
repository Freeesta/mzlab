//! Parallel download of `update::apply`: same folder as a serial run, bounded concurrency, and nothing left behind on an error.

use mzlab_desktop::update::{apply, local_manifest, sha256_hex, validate_web};
use mzlab_desktop::update::{Entry, Manifest, Source, UpdateError};
use std::collections::BTreeMap;
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

/// A source that counts the requests in flight at the same time.
struct Slow {
    files: BTreeMap<String, Vec<u8>>,
    now: AtomicUsize,
    peak: AtomicUsize,
}

impl Slow {
    fn new(files: BTreeMap<String, Vec<u8>>) -> Self {
        Slow {
            files,
            now: AtomicUsize::new(0),
            peak: AtomicUsize::new(0),
        }
    }
}

impl Source for Slow {
    fn get(&self, rel: &str) -> Result<Vec<u8>, UpdateError> {
        let n = self.now.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak.fetch_max(n, Ordering::SeqCst);
        std::thread::sleep(Duration::from_millis(5));
        self.now.fetch_sub(1, Ordering::SeqCst);
        self.files.get(rel).cloned().ok_or(UpdateError {
            key: "update.error.http",
            detail: "404".into(),
        })
    }
}

fn manifest(files: &BTreeMap<String, Vec<u8>>) -> Manifest {
    Manifest {
        version: "b".into(),
        desktop_min: "0.1.0".into(),
        files: files
            .iter()
            .map(|(n, b)| {
                let e = Entry {
                    sha256: sha256_hex(b),
                    size: b.len() as u64,
                };
                (n.clone(), e)
            })
            .collect(),
    }
}

fn many(n: usize) -> BTreeMap<String, Vec<u8>> {
    (0..n)
        .map(|i| (format!("static/f{i}.js"), format!("body {i}").into_bytes()))
        .collect()
}

fn empty() -> Manifest {
    Manifest {
        version: "a".into(),
        desktop_min: "0.1.0".into(),
        files: BTreeMap::new(),
    }
}

#[test]
fn parallel_download_gives_the_same_folder_and_is_bounded() {
    let tmp = tempfile::tempdir().unwrap();
    let data = tmp.path();
    let list = many(60);
    let remote = manifest(&list);
    let src = Slow::new(list.clone());
    let (mut count, mut last) = (0, None);
    let n = apply(data, &empty(), &remote, "0.1.0", &src, |p| {
        count += 1;
        last = Some(p);
    })
    .unwrap();
    assert_eq!(n, 60);
    let peak = src.peak.load(Ordering::SeqCst);
    assert!(peak > 1 && peak <= 8, "peak {peak}");
    let p = last.unwrap();
    assert_eq!((count, p.done, p.total), (60, 60, 60));
    assert_eq!(p.bytes, p.total_bytes);
    let web = data.join("web");
    for (name, body) in &list {
        assert_eq!(&fs::read(web.join(name)).unwrap(), body);
    }
    assert!(validate_web(&web));
    assert_eq!(local_manifest(data, b"{}").unwrap(), remote);
}

#[test]
fn bad_hash_among_many_keeps_web_and_removes_next() {
    let tmp = tempfile::tempdir().unwrap();
    let data = tmp.path();
    let mut served = many(40);
    let remote = manifest(&served);
    served.insert("static/f17.js".into(), b"tampered".to_vec());
    // a populated web/ that must stay untouched
    fs::create_dir_all(data.join("web/static")).unwrap();
    fs::write(data.join("web/static/old.js"), b"old").unwrap();
    let src = Slow::new(served);
    let err = apply(data, &empty(), &remote, "0.1.0", &src, |_| {}).unwrap_err();
    assert_eq!(err.key, "update.error.hash");
    assert_eq!(err.detail, "static/f17.js");
    assert_eq!(fs::read(data.join("web/static/old.js")).unwrap(), b"old");
    assert!(!data.join("web/static/f0.js").exists());
    assert!(!data.join("web.next").exists() && !data.join("web.old").exists());
    // no request is left in flight when apply returns
    assert_eq!(src.now.load(Ordering::SeqCst), 0);
}

#[test]
fn missing_file_midway_leaves_no_next_folder() {
    let tmp = tempfile::tempdir().unwrap();
    let data = tmp.path();
    let mut served = many(30);
    let remote = manifest(&served);
    served.remove("static/f20.js");
    let src = Slow::new(served);
    let err = apply(data, &empty(), &remote, "0.1.0", &src, |_| {}).unwrap_err();
    assert_eq!(err.key, "update.error.http");
    assert!(!data.join("web").exists() && !data.join("web.next").exists());
}
