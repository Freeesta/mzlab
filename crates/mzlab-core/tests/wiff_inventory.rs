//! `wiff::inventory` (the read-only `file info` / `file verify` of the CLI and the `wiffwrite` module) on the synthetic pairs of `tools/wiff_riferimento.py`:
//! every sample listed with a status, the refusals named, a file equal to itself and different from another. Skipped without python3.
#![cfg(feature = "wiffwrite")]

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::json::Value;
use mzlab_core::readers::{FileSet, Limits, Member};
use mzlab_core::wiff::inventory::{list, verify};

fn fixtures() -> Option<PathBuf> {
    let out = std::env::temp_dir().join(format!("mzlab_wiff_inventory_{}", std::process::id()));
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/wiff_riferimento.py");
    let ok = Command::new("python3")
        .arg(script)
        .arg(&out)
        .output()
        .ok()
        .filter(|o| o.status.success());
    if ok.is_none() {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 not available"
        );
        eprintln!("SKIP wiff_inventory: python3 not available");
        return None;
    }
    Some(out)
}

fn set(dir: &Path, stem: &str) -> FileSet {
    let member = |file: String| {
        let bytes = std::fs::read(dir.join(&file)).unwrap();
        Member {
            nome: file,
            dim: bytes.len() as u64,
            dati: Box::new(bytes),
            sha1: None,
        }
    };
    FileSet {
        membri: vec![
            member(format!("{stem}.wiff")),
            member(format!("{stem}.wiff.scan")),
        ],
    }
}

fn json(s: &str) -> Value {
    mzlab_core::json::parse(s.as_bytes()).unwrap()
}

fn statuses(dir: &Path, stem: &str) -> Vec<String> {
    let (text, _) = list(vec![set(dir, stem)], &Limits::sito(), false).unwrap();
    let v = json(&text);
    let Some(Value::Arr(files)) = v.get("files") else {
        panic!("files")
    };
    let Some(Value::Arr(samples)) = files[0].get("samples") else {
        panic!("samples")
    };
    samples
        .iter()
        .map(|s| match s.get("status") {
            Some(Value::Str(x)) => x.clone(),
            _ => panic!("status"),
        })
        .collect()
}

#[test]
fn every_sample_is_listed_with_a_status() {
    let Some(dir) = fixtures() else { return };
    assert_eq!(statuses(&dir, "two"), ["ok", "ok"]);
    assert_eq!(statuses(&dir, "one"), ["ok"]);
}

#[test]
fn refused_pairs_are_listed_as_refused_never_hidden() {
    let Some(dir) = fixtures() else { return };
    for stem in ["periods", "tof", "code", "tofblock", "version4"] {
        let s = statuses(&dir, stem);
        assert!(s.iter().any(|x| x == "refused"), "{stem}: {s:?}");
    }
}

#[test]
fn a_section_the_index_does_not_list_is_reported() {
    let Some(dir) = fixtures() else { return };
    assert!(statuses(&dir, "unindexed").iter().any(|x| x == "unindexed"));
}

#[test]
fn a_file_equals_itself_and_differs_from_another() {
    let Some(dir) = fixtures() else { return };
    let (text, equal) = verify(set(&dir, "two"), vec![set(&dir, "two")], &Limits::sito()).unwrap();
    assert!(equal, "{text}");
    let (text, equal) = verify(set(&dir, "one"), vec![set(&dir, "two")], &Limits::sito()).unwrap();
    assert!(!equal, "{text}");
}

#[test]
fn the_inventory_lists_the_directory_metadata() {
    let Some(dir) = fixtures() else { return };
    let (text, _) = list(vec![set(&dir, "two")], &Limits::sito(), true).unwrap();
    let v = json(&text);
    let Some(Value::Arr(files)) = v.get("files") else {
        panic!("files")
    };
    let Some(Value::Arr(d)) = files[0].get("directory") else {
        panic!("directory")
    };
    assert!(d
        .iter()
        .any(|e| matches!(e.get("path"), Some(Value::Str(p)) if p == "SampleSubtree/Sample1/Idx")));
}

/// The CLI reads with `Limits::desktop()`, the site with `Limits::sito()`: the inventory only goes through the entries and depth checks of the container
/// (it reads by sector, whatever the size), so both give the same text. At the smallest `entries` that still opens the file the text is the same too; one
/// below it the pair is refused with a named key, never a different listing.
#[test]
fn the_site_and_the_app_limits_give_the_same_inventory() {
    let Some(dir) = fixtures() else { return };
    for stem in ["one", "two", "gaps", "periods", "tof", "unindexed"] {
        for full in [false, true] {
            let a = list(vec![set(&dir, stem)], &Limits::sito(), full).unwrap();
            let b = list(vec![set(&dir, stem)], &Limits::desktop(), full).unwrap();
            assert_eq!(a, b, "{stem} full={full}");
        }
    }
    let at = |n: usize| Limits {
        entries: n,
        ..Limits::sito()
    };
    let want = list(vec![set(&dir, "two")], &Limits::sito(), true).unwrap();
    let ok = |n: usize| list(vec![set(&dir, "two")], &at(n), true);
    let min = (1..=64)
        .find(|&n| ok(n).is_ok())
        .expect("a small entries limit opens the fixture");
    assert_eq!(ok(min).unwrap(), want, "entries = {min}");
    if min > 1 {
        let e = ok(min - 1).unwrap_err();
        assert!(
            e.key.starts_with("err.") || e.key.starts_with("error."),
            "{}",
            e.key
        );
    }
}
