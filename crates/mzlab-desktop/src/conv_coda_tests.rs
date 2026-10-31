//! Tests of the conversion queue (`conv_coda.rs`) on small synthetic files.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::conv_coda::{dataset_root, Coda, Indexing};

#[path = "../../mzlab-lettori/tests/comune/mzxml.rs"]
mod mzxml;

// the helpers the included encoder asks of its parent
fn b64(b: &[u8]) -> String {
    const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::new();
    for c in b.chunks(3) {
        let n = c
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, x)| n | u32::from(*x) << (16 - 8 * i));
        for i in 0..=c.len() {
            s.push(A[(n >> (18 - 6 * i) & 63) as usize] as char);
        }
        s.push_str(&"==="[..3 - c.len()]);
    }
    s
}

fn zlib(b: &[u8]) -> Vec<u8> {
    b.to_vec()
}

#[allow(dead_code)]
fn insieme(nome: &str, byte: Vec<u8>) -> mzlab_core::lettori::FileSet {
    mzlab_core::lettori::FileSet {
        membri: vec![mzlab_core::lettori::Membro {
            nome: nome.into(),
            dim: byte.len() as u64,
            dati: Box::new(byte),
            sha1: None,
        }],
    }
}

/// A small mzXML of the first `n` scans of the synthetic run of the tests of `mzlab-lettori`.
fn mzxml_file(path: &Path, n: usize) {
    let scans: Vec<_> = mzxml::corsa().into_iter().take(n).collect();
    let o = mzxml::Opz {
        zlib: false,
        ..mzxml::PIATTO
    };
    std::fs::write(path, mzxml::mzxml(&scans, o)).unwrap();
}

fn wait(c: &Coda, name: &str, state: &str) {
    for _ in 0..600 {
        if c.list()["jobs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|j| j["name"] == name && j["state"] == state)
        {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("{name} never reached {state}: {}", c.list());
}

#[test]
fn two_files_give_the_bytes_of_the_command_line_and_cancel_stops_the_second() {
    let d = tempfile::tempdir().unwrap();
    let (a, b) = (d.path().join("a.mzXML"), d.path().join("b.mzXML"));
    mzxml_file(&a, 4);
    mzxml_file(&b, 3);
    let c = Coda::with_workers(1);
    c.set_folder(d.path().join("cache"));
    // an index being built keeps the queue still until the second file is stopped
    let busy = Indexing::new();
    c.add("a.mzXML", 0, &a);
    c.add("b.mzXML", 0, &b);
    c.cancel(Some("b.mzXML"));
    drop(busy);
    wait(&c, "a.mzXML", "done");
    wait(&c, "b.mzXML", "cancelled");
    // the first stays, ready; the second was never converted
    let got = c.ready("a.mzXML").expect("the first file is ready");
    assert!(c.ready("b.mzXML").is_none());
    let cli = d.path().join("cli");
    let o = mzlab_cli::convert::run(vec![
        a.to_string_lossy().into_owned(),
        "--out".into(),
        cli.to_string_lossy().into_owned(),
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert_eq!(
        std::fs::read(&got).unwrap(),
        std::fs::read(cli.join("a.mzML")).unwrap()
    );
    // saving puts the ready files in one folder
    let to = d.path().join("saved");
    assert_eq!(c.save(&to).unwrap(), 1);
    assert_eq!(
        std::fs::read(to.join("a.mzML")).unwrap(),
        std::fs::read(&got).unwrap()
    );
    // a changed dataset is not ready any more
    mzxml_file(&a, 5);
    assert!(c.ready("a.mzXML").is_none());
}

#[test]
fn a_file_that_cannot_be_converted_fails_with_its_cause() {
    let d = tempfile::tempdir().unwrap();
    let bad = d.path().join("rotto.raw");
    std::fs::write(&bad, b"not a raw file at all").unwrap();
    let c = Coda::with_workers(1);
    c.set_folder(d.path().join("cache"));
    c.add("rotto.raw", 0, &bad);
    wait(&c, "rotto.raw", "failed");
    assert!(!c.list()["jobs"][0]["error"].as_str().unwrap().is_empty());
}

#[test]
fn the_root_of_a_dataset_is_found_from_its_members() {
    let m = vec![(
        "x.D/AcqData/MSScan.bin".to_string(),
        PathBuf::from("/s/x.D/AcqData/MSScan.bin"),
    )];
    assert_eq!(dataset_root(&m), Some(PathBuf::from("/s/x.D")));
    let w = vec![
        ("x.wiff.scan".to_string(), PathBuf::from("/s/x.wiff.scan")),
        ("x.wiff".to_string(), PathBuf::from("/s/x.wiff")),
    ];
    assert_eq!(dataset_root(&w), Some(PathBuf::from("/s/x.wiff")));
}

fn queued(c: &Coda, name: &str, root: &str, stamp: (u64, u64)) {
    c.enqueue(name, 0, Path::new(root), Some(stamp));
}

#[test]
fn a_job_replaced_while_it_runs_cannot_corrupt_the_queue() {
    let c = Coda::with_workers(2);
    queued(&c, "a", "/s/a.raw", (1, 1));
    queued(&c, "b", "/s/b.raw", (1, 1));
    let a = c.inner.lock().unwrap().take_for_test();
    // the dataset of `a` changes while it runs: the old job leaves the front, the list shifts
    queued(&c, "a", "/s/a.raw", (2, 2));
    c.inner
        .lock()
        .unwrap()
        .finish_for_test(a, Ok(vec![PathBuf::from("old.mzML")]));
    // no panic, the stale result is dropped, the others are untouched
    let rows = c.list();
    let rows = rows["jobs"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|r| r["state"] == "queued"), "{rows:?}");
    assert!(c.ready("a").is_none());
}

#[test]
fn a_new_job_of_a_running_dataset_waits_for_it() {
    let c = Coda::with_workers(2);
    queued(&c, "a", "/s/a.raw", (1, 1));
    let first = c.inner.lock().unwrap().take_for_test();
    queued(&c, "a", "/s/a.raw", (2, 2));
    // one output folder: the second worker must not start the same dataset
    assert!(!c.inner.lock().unwrap().can_take_for_test());
    c.inner.lock().unwrap().finish_for_test(first, Ok(vec![]));
    assert!(c.inner.lock().unwrap().can_take_for_test());
}

#[test]
fn the_stamp_covers_every_member_of_a_dataset() {
    let d = tempfile::tempdir().unwrap();
    let wiff = d.path().join("x.wiff");
    std::fs::write(&wiff, b"w").unwrap();
    std::fs::write(d.path().join("x.wiff.scan"), b"1234").unwrap();
    let before = crate::conv_stamp::stamp(&wiff).unwrap();
    std::fs::write(d.path().join("x.wiff.scan"), b"12345678").unwrap();
    assert_ne!(crate::conv_stamp::stamp(&wiff), Some(before));
    let dd = d.path().join("y.D");
    std::fs::create_dir_all(dd.join("AcqData")).unwrap();
    std::fs::write(dd.join("AcqData/MSScan.bin"), b"ab").unwrap();
    let before = crate::conv_stamp::stamp(&dd).unwrap();
    std::fs::write(dd.join("AcqData/MSScan.bin"), b"abcd").unwrap();
    assert_ne!(crate::conv_stamp::stamp(&dd), Some(before));
}

#[test]
fn old_samples_are_removed_only_when_the_dataset_changed() {
    let d = tempfile::tempdir().unwrap();
    let a = d.path().join("a.mzXML");
    mzxml_file(&a, 4);
    let out = d.path().join("out");
    std::fs::create_dir_all(&out).unwrap();
    let stray = out.join("a-7.mzML");
    std::fs::write(&stray, b"old").unwrap();
    let st = crate::conv_stamp::stamp(&a);
    let files = crate::conv_coda::convert_fresh(&a, &out, st).unwrap();
    assert_eq!(files.len(), 1, "{files:?}");
    assert!(!stray.exists());
    // the same bytes again keep what is there (the cache of a second run)
    std::fs::write(&stray, b"kept").unwrap();
    crate::conv_coda::convert_fresh(&a, &out, st).unwrap();
    assert!(stray.exists());
}

#[test]
fn an_index_that_starts_just_after_queueing_holds_the_job_back() {
    let d = tempfile::tempdir().unwrap();
    let a = d.path().join("a.mzXML");
    mzxml_file(&a, 3);
    // a long re-check pause keeps the worker inside its window however slow the machine is, so the order is fixed:
    // job queued, then the index starts, then the worker re-checks
    let c = Coda::with_recheck(3000);
    c.set_folder(d.path().join("cache"));
    c.add("a.mzXML", 0, &a);
    let busy = Indexing::new();
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(c.list()["jobs"][0]["state"], "queued");
    drop(busy);
    wait(&c, "a.mzXML", "done");
}
