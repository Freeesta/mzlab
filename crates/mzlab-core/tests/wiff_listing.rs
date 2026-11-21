//! `readers::list_samples` (`samples_list/1`) of a `.wiff` pair read from its container and index alone (`readers::open_listing`): the `.wiff.scan` is
//! never read, the numbers are the ones the full open finds on a clean pair, and a copy cut short shows as index rows past its end (`missing`).
//! Synthetic pairs of tools/wiff_riferimento.py and tools/wiff_grande.py; python3 is required (the test fails without it, it never skips).
#![allow(clippy::disallowed_methods)] // the test sorts file names with the standard sort

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use mzlab_core::idxread::ReadAt;
use mzlab_core::readers::{self, FileSet, Limits, Member};

struct Counted(Vec<u8>, Arc<AtomicUsize>);

impl ReadAt for Counted {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> mzlab_core::Result<()> {
        self.1.fetch_add(1, Ordering::SeqCst);
        self.0.read_at(off, buf)
    }
}

fn python(script: &str, args: &[&Path]) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools")
        .join(script);
    let o = Command::new("python3")
        .arg(path)
        .args(args)
        .output()
        .expect("python3 is required");
    assert!(
        o.status.success(),
        "{script}: {}",
        String::from_utf8_lossy(&o.stderr)
    );
}

fn pairs() -> PathBuf {
    static N: AtomicUsize = AtomicUsize::new(0);
    let out = std::env::temp_dir().join(format!(
        "mzlab_wiff_listing_{}_{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    python("wiff_riferimento.py", &[&out.join("ref")]);
    python(
        "wiff_grande.py",
        &[
            &out.join("big"),
            Path::new("--samples"),
            Path::new("3"),
            Path::new("--scans"),
            Path::new("40"),
            Path::new("--points"),
            Path::new("50"),
        ],
    );
    out
}

fn set(dir: &Path, name: &str, cut: Option<usize>, reads: &Arc<AtomicUsize>) -> FileSet {
    let wiff = std::fs::read(dir.join(format!("{name}.wiff"))).unwrap();
    let mut scan = std::fs::read(dir.join(format!("{name}.wiff.scan"))).unwrap();
    if let Some(n) = cut {
        scan.truncate(scan.len() - n);
    }
    let (w, s) = (wiff.len() as u64, scan.len() as u64);
    FileSet {
        membri: vec![
            Member {
                nome: format!("{name}.wiff"),
                dim: w,
                dati: Box::new(wiff),
                sha1: None,
            },
            Member {
                nome: format!("{name}.wiff.scan"),
                dim: s,
                dati: Box::new(Counted(scan, reads.clone())),
                sha1: None,
            },
        ],
    }
}

/// The listing (or `Err(error_key)`) and how many reads of the `.wiff.scan` it took.
fn json(dir: &Path, name: &str, listing: bool) -> (Result<String, &'static str>, usize) {
    let reads = Arc::new(AtomicUsize::new(0));
    let fs = set(dir, name, None, &reads);
    let l = if listing {
        readers::open_listing(fs, &Limits::sito())
    } else {
        readers::open(fs, &Limits::sito())
    };
    let out = l
        .map(|mut l| readers::list_samples(&mut *l))
        .map_err(|e| e.key);
    (out, reads.load(Ordering::SeqCst))
}

/// The listing without the fields only it carries (`size`, from the index rows).
fn plain(json: &str) -> serde_json::Value {
    let mut v: serde_json::Value = serde_json::from_str(json).unwrap();
    for s in v["samples"].as_array_mut().unwrap() {
        s.as_object_mut().unwrap().remove("size");
    }
    v
}

/// Pairs where the full open and the listing must agree: no refused sample (the full open drops its rows), no unindexed section, no failing block head.
const CLEAN: [&str; 10] = [
    "blocchi",
    "gaps",
    "dadmetodo",
    "dispositivi",
    "energia",
    "epi3200",
    "ida",
    "nomi",
    "one",
    "prodotto",
];

#[test]
fn the_listing_never_reads_the_scan_file_and_says_what_the_full_open_says() {
    let dir = pairs();
    let mut names: Vec<String> = std::fs::read_dir(dir.join("ref"))
        .unwrap()
        .filter_map(|e| {
            e.unwrap()
                .file_name()
                .to_str()?
                .strip_suffix(".wiff.scan")
                .map(String::from)
        })
        .collect();
    names.sort();
    for (d, name) in names
        .iter()
        .map(|n| ("ref", n.as_str()))
        .chain([("big", "grande")])
    {
        let d = dir.join(d);
        if !d.join(format!("{name}.wiff")).exists() {
            continue;
        }
        let (cheap, reads) = json(&d, name, true);
        assert_eq!(reads, 0, "{name}: the .wiff.scan was read");
        let (full, heads) = json(&d, name, false);
        assert!(
            name != "grande" || heads > 0,
            "the full open reads the block heads"
        );
        if let Ok(c) = &cheap {
            let v: serde_json::Value = serde_json::from_str(c).unwrap();
            assert_eq!(v["schema"], "samples_list/1", "{name}");
            assert!(v["bytes"].as_u64().unwrap() > 0, "{name}");
        }
        if CLEAN.contains(&name) || name == "grande" {
            assert_eq!(plain(&cheap.unwrap()), plain(&full.unwrap()), "{name}");
        }
    }
    let big = json(&dir.join("big"), "grande", true).0.unwrap();
    let v: serde_json::Value = serde_json::from_str(&big).unwrap();
    let s = v["samples"].as_array().unwrap();
    assert_eq!(s.len(), 3);
    assert!(
        s.iter().all(|x| x["scans"] == 40
            && x["kinds"] == serde_json::json!(["ms"])
            && x["polarity"] == "+"
            && x["tic"]["apex"].as_f64().unwrap() > 0.0),
        "{big}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_cut_scan_file_shows_as_missing_rows_and_a_short_data_file() {
    let dir = pairs();
    let reads = Arc::new(AtomicUsize::new(0));
    let mut l = readers::open_listing(
        set(&dir.join("big"), "grande", Some(500), &reads),
        &Limits::sito(),
    )
    .unwrap();
    let v: serde_json::Value = serde_json::from_str(&readers::list_samples(&mut *l)).unwrap();
    let last = &v["samples"].as_array().unwrap()[2];
    assert!(last["missing"].as_u64().unwrap() > 0, "{v}");
    assert!(last["scans"].as_u64().unwrap() < 40, "{v}");
    assert!(
        v["scan"]["bytes"].as_u64().unwrap() < v["scan"]["indexed_end"].as_u64().unwrap(),
        "{v}"
    );
    assert_eq!(reads.load(Ordering::SeqCst), 0);
    let _ = std::fs::remove_dir_all(&dir);
}
