//! The `.mzidx` built straight from a vendor file set (`upload::prepare_lettore` + `mzidx::build_lettore`, version 2) equals the one built from the
//! mzML `readers_mzml::scrivi` writes for the same sample: the same sections byte for byte, the same rows but for their first two columns, the same
//! metadata but for `source` and `version`; and the same index whatever the budget (one pass or many). The two sources of `sorgente` (the mzML by
//! offset and the reader) give the same peaks and traces. Synthetic `.D` always, synthetic `.wiff` with
//! python3, the real `.wiff`, `.D` and `.raw` with the private data repository (MZLAB_DATI or `mzlab-dati` beside this one); skipped without them.
#![allow(clippy::indexing_slicing)]
#![allow(clippy::disallowed_methods)] // tests compare against the standard sorts

mod agilent_synth;
mod readers_routes;

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use agilent_synth::{encode, sample};
use mzlab_core::idxread::{Index, Scan, READER_FLAG};
use mzlab_core::json::{self, Value};
use mzlab_core::mzml::ChromKind;
use mzlab_core::mzmlhead::Mzml;
use mzlab_core::readers::{self, ChromatogramEntry, FileSet, Limits, Member};
use mzlab_core::source::{MzmlTraces, ReaderSource, Scansioni, Traces};
use mzlab_core::{mzidx, readers_mzml, upload};
use readers_routes::{mzml_of, same_routes};

#[path = "../../../tools/dati_percorsi.rs"]
mod dati_percorsi;

const BIG: usize = mzidx::DEFAULT_BUDGET;
/// Indexes built in more than one pass with the small budget.
static MANY: AtomicUsize = AtomicUsize::new(0);

fn u64_at(b: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(b[at..at + 8].try_into().unwrap())
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().unwrap())
}

fn json_at(b: &[u8], off: usize, len: usize) -> Value {
    json::parse(&b[u64_at(b, off) as usize..][..u64_at(b, len) as usize]).unwrap()
}

/// The object without the keys `skip`.
fn without(v: &Value, skip: &[&str]) -> Vec<(String, Value)> {
    match v {
        Value::Obj(o) => o
            .iter()
            .filter(|(k, _)| !skip.contains(&k.as_str()))
            .cloned()
            .collect(),
        _ => panic!("not an object"),
    }
}

fn file(name: &str, bytes: Vec<u8>) -> Member {
    Member {
        nome: name.into(),
        dim: bytes.len() as u64,
        dati: Box::new(bytes),
        sha1: None,
    }
}

fn on_disk(dir: &Path, rel: &str) -> Member {
    let f = std::fs::File::open(dir.join(rel)).unwrap();
    Member {
        nome: rel.into(),
        dim: f.metadata().unwrap().len(),
        dati: Box::new(f),
        sha1: None,
    }
}

/// Index of sample `c` from `make()` both ways; returns the number of scans compared.
fn same(label: &str, make: &dyn Fn() -> FileSet, c: usize) -> usize {
    let mut fs = make();
    let members = upload::impronte(&mut fs).unwrap();
    let mut l = readers::apri(fs, &Limits::desktop()).unwrap();
    let name = format!("{label}.mzML");
    // A: through the mzML
    let mzml = readers_mzml::scrivi(&mut *l, c, Vec::new(), &mut |_, _| {}).unwrap();
    let pa = upload::prepare(&mut mzml.clone(), mzml.len() as u64, &name).unwrap();
    let (a, _) = mzidx::build_mzml_to_vec(&pa.table, &mzml, BIG).unwrap();
    same_sources(label, mzml, &a, &mut *l, c);
    // B: straight from the reader, with a large and a small budget
    let pb = upload::prepare_reader(&mut *l, c, &members, &name).unwrap();
    let build = |l: &mut dyn readers::Reader, budget: usize| {
        let mut out = Cursor::new(Vec::new());
        let st = mzidx::build_reader(&pb.table, l, c, &mut out, budget)?;
        Ok::<_, mzlab_core::error::Error>((out.into_inner(), st))
    };
    let (b, _) = build(&mut *l, BIG).unwrap();
    // the smallest budget that holds the largest bucket: several passes when the file has enough peaks
    let (small, st_small) = [1 << 12, 1 << 15, 1 << 18, 1 << 21]
        .into_iter()
        .find_map(|budget| build(&mut *l, budget).ok())
        .unwrap();
    if st_small.passes > 1 {
        MANY.fetch_add(1, Ordering::Relaxed);
    }
    assert!(small == b, "{label}: the budget changes the index");

    compare(label, &a, &b, &pb.sha256, members.len(), l.samples()[c].id);
    let entries = l.index(c).unwrap().to_vec();
    // the reader of the index takes it, with the positions of the reader
    let idx = Index::open(b.clone(), b.len() as u64).unwrap();
    assert_eq!(idx.scans.len(), entries.len());
    for (k, (s, v)) in idx.scans.iter().zip(&entries).enumerate() {
        assert_eq!(
            (s.k, s.position, s.start),
            (Some(k), v.position, 0),
            "{label}: {k}"
        );
    }
    same_routes(label, &mzml_of(&mut *l, c), &a, &b, &mut *l, c);
    entries.len()
}

/// Index `b` (from the reader) against `a` (from the mzML): header, sections, rows from the third column, metadata and source.
fn compare(label: &str, a: &[u8], b: &[u8], sha256: &str, n_members: usize, id: u32) {
    // header: version and flags, the same layout of the sections
    assert_eq!((u32_at(a, 8), u32_at(a, 12)), (1, 0), "{label}");
    assert_eq!((u32_at(b, 8), u32_at(b, 12)), (2, READER_FLAG), "{label}");
    assert_eq!(u64_at(b, 48), b.len() as u64);
    let scans_off = u64_at(a, 32) as usize;
    assert_eq!(u64_at(b, 32) as usize, scans_off, "{label}: sections");
    assert!(
        a[64..scans_off] == b[64..scans_off],
        "{label}: sections differ"
    );
    // rows: all but the first two columns
    let (ra, rb) = (json_at(a, 32, 40), json_at(b, 32, 40));
    assert_eq!(ra.arr().len(), rb.arr().len(), "{label}: rows");
    for (k, (x, y)) in ra.arr().iter().zip(rb.arr()).enumerate() {
        assert_eq!(x.arr()[2..], y.arr()[2..], "{label}: row {k}");
        assert_eq!(y.at(1).and_then(Value::f64), Some(k as f64));
    }
    // metadata: all but source and version
    let (ma, mb) = (json_at(a, 16, 24), json_at(b, 16, 24));
    assert_eq!(
        without(&ma, &["source", "version"]),
        without(&mb, &["source", "version"]),
        "{label}: metadata"
    );
    assert_eq!(mb.get("version").and_then(Value::f64), Some(2.0));
    let src = mb.get("source").unwrap();
    assert_eq!(src.get("members").unwrap().arr().len(), n_members);
    assert_eq!(src.get("sha256").and_then(Value::str), Some(sha256));
    assert_eq!(
        src.get("sample")
            .and_then(|s| s.get("id"))
            .and_then(Value::f64),
        Some(f64::from(id))
    );
}

/// `Scansioni` and `Tracce` give the same peaks and traces from the mzML by offset (rows of version 1) and from the reader (rows of version 2).
fn same_sources(label: &str, mzml: Vec<u8>, a: &[u8], l: &mut dyn readers::Reader, c: usize) {
    let size = mzml.len() as u64;
    let rows = Index::open(a.to_vec(), a.len() as u64).unwrap().scans;
    let n = rows.len();
    let mut by_offset = mzml;
    let mut src = ReaderSource { l, c };
    for k in (0..n).step_by((n / 40).max(1)) {
        let native = Scan {
            k: Some(k),
            ..Scan::default()
        };
        assert!(
            by_offset.spettro(&rows[k]).unwrap() == src.spettro(&native).unwrap(),
            "{label}: peaks of scan {k}"
        );
    }
    let mut m = Mzml::new(by_offset, size);
    let mut tr = MzmlTraces::new(&mut m, rows.last().map_or(0, |s| s.end));
    // the mass-spectrometer traces are compared with their kind; the optical, pressure and device ones (whose kind the mzML may spell otherwise) by id
    // and points, on both sides
    let ms = |k: ChromKind| {
        !matches!(
            k,
            ChromKind::Pda | ChromKind::Pressure | ChromKind::Analog | ChromKind::Other
        )
    };
    let (mut written, mut other): (Vec<ChromatogramEntry>, Vec<_>) = (Vec::new(), Vec::new());
    for v in src.chromatograms().unwrap() {
        let t = src.chromatogram(&v.id).unwrap();
        match (t.time.is_empty(), ms(t.kind)) {
            (true, _) => {}
            (false, true) => written.push(v),
            (false, false) => other.push(t),
        }
    }
    let all = tr.chromatograms().unwrap();
    let listed: Vec<_> = all.iter().filter(|x| ms(x.tipo)).collect();
    assert_eq!(listed.len(), written.len(), "{label}: chromatograms");
    assert_eq!(all.len(), written.len() + other.len(), "{label}: other traces");
    for q in &other {
        let p = tr.chromatogram(&q.id).unwrap();
        assert!(
            p.time == q.time && p.intensity == q.intensity,
            "{label}: trace {}",
            q.id
        );
    }
    for (x, y) in listed.iter().zip(&written) {
        assert_eq!((&x.id, x.tipo), (&y.id, y.tipo), "{label}");
        let (p, q) = (
            tr.chromatogram(&x.id).unwrap(),
            src.chromatogram(&y.id).unwrap(),
        );
        assert!(
            p.time == q.time && p.intensity == q.intensity,
            "{label}: trace {}",
            x.id
        );
    }
    let mut srm_src = mzlab_core::srm::leggi(&mut src).unwrap();
    srm_src.retain(|t| !t.t.is_empty());
    assert_eq!(
        mzlab_core::srm::leggi(&mut tr).unwrap(),
        srm_src,
        "{label}: SRM traces"
    );
    let (ha, hb) = (tr.file_header().unwrap(), src.file_header().unwrap());
    assert!(
        ha.starts_with(&hb) || hb.starts_with(&ha),
        "{label}: header"
    );
}

/// Every sample of a set.
fn same_all(label: &str, make: &dyn Fn() -> FileSet) -> usize {
    let n = readers::apri(make(), &Limits::desktop())
        .unwrap()
        .samples()
        .len();
    (0..n).map(|c| same(&format!("{label}#{c}"), make, c)).sum()
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .collect();
    v.sort();
    for p in v {
        if p.is_dir() {
            walk(&p, out);
        } else {
            out.push(p);
        }
    }
}

fn wiff_set(dir: &Path, name: &str) -> FileSet {
    FileSet {
        membri: vec![
            on_disk(dir, &format!("{name}.wiff")),
            on_disk(dir, &format!("{name}.wiff.scan")),
        ],
    }
}

/// A `.D` folder: every file, named `X.D/<path>`.
fn d_set(dir: &Path) -> FileSet {
    let parent = dir.parent().unwrap();
    let mut files = Vec::new();
    walk(dir, &mut files);
    FileSet {
        membri: files
            .iter()
            .map(|f| {
                let rel = f
                    .strip_prefix(parent)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                on_disk(parent, &rel)
            })
            .collect(),
    }
}

#[test]
fn a_synthetic_d_gives_the_index_of_its_mzml() {
    let (idx, peak) = encode(&sample());
    let make = || FileSet {
        membri: vec![
            file("x.D/AcqData/MSScan.bin", idx.clone()),
            file("x.D/AcqData/MSPeak.bin", peak.clone()),
        ],
    };
    assert!(same_all("synthetic.D", &make) > 0);
}

#[test]
fn the_synthetic_wiff_pairs_give_the_index_of_their_mzml() {
    let out = std::env::temp_dir().join(format!("mzlab_lettori_mzidx_{}", std::process::id()));
    let ok = Command::new("python3")
        .arg(root().join("tools/wiff_riferimento.py"))
        .arg(&out)
        .output()
        .ok()
        .filter(|o| o.status.success());
    if ok.is_none() {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 not available"
        );
        eprintln!("SKIP read_mzidx.rs: python3 not available");
        return;
    }
    let n: usize = ["one", "two", "serie", "prodotto"]
        .iter()
        .filter(|n| out.join(format!("{n}.wiff.scan")).exists())
        .map(|n| same_all(n, &|| wiff_set(&out, n)))
        .sum();
    let _ = std::fs::remove_dir_all(&out);
    assert!(n > 0);
    assert!(
        MANY.load(Ordering::Relaxed) > 0,
        "no index built in more than one pass"
    );
}

/// A full scan with a DAD (`tools/wiff_dad_mrm.py`, `dadscan`): the TWC is in the mzML with the points the reader gives.
#[test]
fn a_synthetic_wiff_with_a_dad_gives_the_index_of_its_mzml() {
    let out = std::env::temp_dir().join(format!("mzlab_lettori_mzidx_dad_{}", std::process::id()));
    let ok = Command::new("python3")
        .arg(root().join("tools/wiff_dad_mrm.py"))
        .arg(&out)
        .output()
        .ok()
        .filter(|o| o.status.success());
    if ok.is_none() {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 not available"
        );
        eprintln!("SKIP read_mzidx.rs: python3 not available");
        return;
    }
    let n = same_all("dadscan", &|| wiff_set(&out, "dadscan"));
    let _ = std::fs::remove_dir_all(&out);
    assert!(n > 0);
}

#[test]
fn the_real_files_give_the_index_of_their_mzml() {
    if dati_percorsi::radice().is_none() {
        eprintln!("SKIP read_mzidx.rs: no data repository (MZLAB_DATI)");
        return;
    }
    let mut files = Vec::new();
    for dir in ["dati/3200 qtrap/wiff", "dati/GC-MS", "dati/HRMS"] {
        if let Some(d) = dati_percorsi::percorso(dir) {
            walk(&d, &mut files);
        }
    }
    let ext = |p: &Path, e: &str| p.extension().is_some_and(|x| x.eq_ignore_ascii_case(e));
    let (mut samples, mut scans) = (0usize, 0usize);
    let mut count = |label: &str, make: &dyn Fn() -> FileSet| {
        let n = readers::apri(make(), &Limits::desktop())
            .unwrap()
            .samples()
            .len();
        for c in 0..n {
            scans += same(&format!("{label}#{c}"), make, c);
            samples += 1;
        }
    };
    for f in files.iter().filter(|f| ext(f, "wiff")) {
        let (dir, name) = (
            f.parent().unwrap(),
            f.file_stem().unwrap().to_string_lossy(),
        );
        if dir.join(format!("{name}.wiff.scan")).exists() {
            count(&name, &|| wiff_set(dir, &name));
        }
    }
    let mut dirs: Vec<PathBuf> = files
        .iter()
        .filter_map(|f| f.ancestors().find(|a| ext(a, "d")).map(Path::to_path_buf))
        .collect();
    dirs.dedup();
    for dir in &dirs {
        count(&dir.file_name().unwrap().to_string_lossy(), &|| d_set(dir));
    }
    for f in files.iter().filter(|f| ext(f, "raw")) {
        let (dir, name) = (
            f.parent().unwrap(),
            f.file_name().unwrap().to_string_lossy(),
        );
        count(&name, &|| FileSet {
            membri: vec![on_disk(dir, &name)],
        });
    }
    eprintln!("read_mzidx.rs: {samples} real samples equal ({scans} scans)");
    assert!(samples > 0);
}
