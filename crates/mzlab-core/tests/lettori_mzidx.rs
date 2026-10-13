//! The `.mzidx` built straight from a vendor file set (`upload::prepare_lettore` + `mzidx::build_lettore`, version 2) equals the one built from the
//! mzML `lettori_mzml::scrivi` writes for the same sample: the same sections byte for byte, the same rows but for their first two columns, the same
//! metadata but for `source` and `version`; and the same index whatever the budget (one pass or many). The two sources of `sorgente` (the mzML by
//! offset and the reader) give the same peaks and traces. Synthetic `.D` always, synthetic `.wiff` with
//! python3, the real `.wiff`, `.D` and `.raw` with the private data repository (MZLAB_DATI or `mzlab-dati` beside this one); skipped without them.
#![allow(clippy::indexing_slicing)]

mod agilent_synth;

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use agilent_synth::{encode, sample};
use mzlab_core::idxread::{Index, Scan, FLAG_LETTORE};
use mzlab_core::json::{self, Value};
use mzlab_core::lettori::{self, FileSet, Limiti, Membro, VoceCromatogramma};
use mzlab_core::mzml::ChromKind;
use mzlab_core::mzmlhead::Mzml;
use mzlab_core::sorgente::{Scansioni, SorgenteLettore, Tracce, TracceMzml};
use mzlab_core::{lettori_mzml, mzidx, upload};

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

fn file(nome: &str, bytes: Vec<u8>) -> Membro {
    Membro {
        nome: nome.into(),
        dim: bytes.len() as u64,
        dati: Box::new(bytes),
    }
}

fn on_disk(dir: &Path, rel: &str) -> Membro {
    let f = std::fs::File::open(dir.join(rel)).unwrap();
    Membro {
        nome: rel.into(),
        dim: f.metadata().unwrap().len(),
        dati: Box::new(f),
    }
}

/// Index of sample `c` from `make()` both ways; returns the number of scans compared.
fn same(label: &str, make: &dyn Fn() -> FileSet, c: usize) -> usize {
    let mut fs = make();
    let membri = upload::impronte(&mut fs).unwrap();
    let mut l = lettori::apri(fs, &Limiti::desktop()).unwrap();
    let name = format!("{label}.mzML");
    // A: through the mzML
    let mzml = lettori_mzml::scrivi(&mut *l, c, Vec::new(), &mut |_, _| {}).unwrap();
    let pa = upload::prepare(&mut mzml.clone(), mzml.len() as u64, &name).unwrap();
    let (a, _) = mzidx::build_mzml_to_vec(&pa.table, &mzml, BIG).unwrap();
    same_sources(label, mzml, &a, &mut *l, c);
    // B: straight from the reader, with a large and a small budget
    let pb = upload::prepare_lettore(&mut *l, c, &membri, &name).unwrap();
    let build = |l: &mut dyn lettori::Lettore, budget: usize| {
        let mut out = Cursor::new(Vec::new());
        let st = mzidx::build_lettore(&pb.table, l, c, &mut out, budget)?;
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

    // header: version and flags, the same layout of the sections
    assert_eq!((u32_at(&a, 8), u32_at(&a, 12)), (1, 0), "{label}");
    assert_eq!(
        (u32_at(&b, 8), u32_at(&b, 12)),
        (2, FLAG_LETTORE),
        "{label}"
    );
    assert_eq!(u64_at(&b, 48), b.len() as u64);
    let scans_off = u64_at(&a, 32) as usize;
    assert_eq!(u64_at(&b, 32) as usize, scans_off, "{label}: sections");
    assert!(
        a[64..scans_off] == b[64..scans_off],
        "{label}: sections differ"
    );
    // rows: all but the first two columns
    let (ra, rb) = (json_at(&a, 32, 40), json_at(&b, 32, 40));
    assert_eq!(ra.arr().len(), rb.arr().len(), "{label}: rows");
    let voci = l.indice(c).unwrap().to_vec();
    for (k, (x, y)) in ra.arr().iter().zip(rb.arr()).enumerate() {
        assert_eq!(x.arr()[2..], y.arr()[2..], "{label}: row {k}");
        assert_eq!(y.at(1).and_then(Value::f64), Some(k as f64));
    }
    // metadata: all but source and version
    let (ma, mb) = (json_at(&a, 16, 24), json_at(&b, 16, 24));
    assert_eq!(
        without(&ma, &["source", "version"]),
        without(&mb, &["source", "version"]),
        "{label}: metadata"
    );
    assert_eq!(mb.get("version").and_then(Value::f64), Some(2.0));
    let src = mb.get("source").unwrap();
    assert_eq!(src.get("members").unwrap().arr().len(), membri.len());
    assert_eq!(
        src.get("sha256").and_then(Value::str),
        Some(pb.sha256.as_str())
    );
    assert_eq!(
        src.get("sample")
            .and_then(|s| s.get("id"))
            .and_then(Value::f64),
        Some(f64::from(l.campioni()[c].id))
    );
    // the reader of the index takes it, with the positions of the reader
    let idx = Index::open(b.clone(), b.len() as u64).unwrap();
    assert_eq!(idx.scans.len(), voci.len());
    for (k, (s, v)) in idx.scans.iter().zip(&voci).enumerate() {
        assert_eq!(
            (s.k, s.posizione, s.start),
            (Some(k), v.posizione, 0),
            "{label}: {k}"
        );
    }
    voci.len()
}

/// `Scansioni` and `Tracce` give the same peaks and traces from the mzML by offset (rows of version 1) and from the reader (rows of version 2).
fn same_sources(label: &str, mzml: Vec<u8>, a: &[u8], l: &mut dyn lettori::Lettore, c: usize) {
    let size = mzml.len() as u64;
    let rows = Index::open(a.to_vec(), a.len() as u64).unwrap().scans;
    let n = rows.len();
    let mut by_offset = mzml;
    let mut src = SorgenteLettore { l, c };
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
    let mut tr = TracceMzml {
        m: &mut m,
        scans_end: rows.last().map_or(0, |s| s.end),
    };
    let written: Vec<VoceCromatogramma> = src
        .cromatogrammi()
        .unwrap()
        .into_iter()
        .filter(|v| {
            let t = src.cromatogramma(&v.id).unwrap();
            !t.time.is_empty()
                && !matches!(
                    t.kind,
                    ChromKind::Pda | ChromKind::Pressure | ChromKind::Analog | ChromKind::Other
                )
        })
        .collect();
    let listed = tr.cromatogrammi().unwrap();
    assert_eq!(listed.len(), written.len(), "{label}: chromatograms");
    for (x, y) in listed.iter().zip(&written) {
        assert_eq!((&x.id, x.tipo), (&y.id, y.tipo), "{label}");
        let (p, q) = (
            tr.cromatogramma(&x.id).unwrap(),
            src.cromatogramma(&y.id).unwrap(),
        );
        assert!(
            p.time == q.time && p.intensity == q.intensity,
            "{label}: trace {}",
            x.id
        );
    }
    let (ha, hb) = (tr.testata().unwrap(), src.testata().unwrap());
    assert!(
        ha.starts_with(&hb) || hb.starts_with(&ha),
        "{label}: header"
    );
}

/// Every sample of a set.
fn same_all(label: &str, make: &dyn Fn() -> FileSet) -> usize {
    let n = lettori::apri(make(), &Limiti::desktop())
        .unwrap()
        .campioni()
        .len();
    (0..n).map(|c| same(&format!("{label}#{c}"), make, c)).sum()
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn dati() -> Option<PathBuf> {
    let mut cands: Vec<PathBuf> = std::env::var_os("MZLAB_DATI")
        .map(PathBuf::from)
        .into_iter()
        .collect();
    for up in root().canonicalize().ok()?.ancestors().skip(1).take(3) {
        cands.push(up.join("mzlab-dati"));
    }
    cands
        .into_iter()
        .find(|p| p.is_dir() && p.join("nomi_vietati.txt").exists())
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
        eprintln!("SKIP lettori_mzidx: python3 not available");
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

#[test]
fn the_real_files_give_the_index_of_their_mzml() {
    let Some(d) = dati() else {
        eprintln!("SKIP lettori_mzidx: no data repository (MZLAB_DATI)");
        return;
    };
    let mut files = Vec::new();
    walk(&d.join("3200 qtrap"), &mut files);
    walk(&d.join("GC-MS"), &mut files);
    walk(&d.join("HRMS"), &mut files);
    let ext = |p: &Path, e: &str| p.extension().is_some_and(|x| x.eq_ignore_ascii_case(e));
    let (mut samples, mut scans) = (0usize, 0usize);
    let mut count = |label: &str, make: &dyn Fn() -> FileSet| {
        let n = lettori::apri(make(), &Limiti::desktop())
            .unwrap()
            .campioni()
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
    eprintln!("lettori_mzidx: {samples} real samples equal ({scans} scans)");
    assert!(samples > 0);
}
