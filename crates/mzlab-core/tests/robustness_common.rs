//! Robustness of the readers of files (`docs/agenti/test.md`): for every format a valid seed and its deterministic mutations (truncations, flipped bytes,
//! counts and lengths at their maximum, zeros, an empty file, a valid head and a random body) give an `err.*` key or a reading, never a panic, a hang or an
//! allocation out of proportion to the file. Synthetic seeds only (the `.wiff` ones need python3 and skip without it); no `MZLAB_DATI`.
//! `ROBUSTEZZA_TRACCIA=1 cargo test -p mzlab-core --test robustness_common -- --nocapture` prints every case before it runs (to find the one that aborts).

#![allow(clippy::disallowed_methods)] // tests compare against the standard sorts

mod agilent_synth;
mod robustness;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use agilent_synth::{encode, sample};
use mzlab_core::idxread::Index;
use mzlab_core::ole::Ole;
use mzlab_core::readers::{self, Limits};
use mzlab_core::{methodinfo, mzml, readers_mzml, wiff};
use robustness::*;

#[global_allocator]
static ALLOCATORE: Counter = Counter;

type Members = Vec<(String, Vec<u8>)>;

const MZML: &[u8] = include_bytes!("../../../tests/golden/ref/msntree_rotta/msn.mzML");
const MZIDX: &[u8] = include_bytes!("../../../tests/golden/ref/msntree_rotta/msn.mzidx");

/// Everything a reader of the registry offers, on whatever the set is: the keys of the errors it met.
fn run_registry(m: &[(String, Vec<u8>)]) -> Vec<String> {
    let mut k = Vec::new();
    let _ = readers::riconosci(&mut fileset(m));
    let Some(mut l) = note(readers::apri(fileset(m), &Limits::sito()), &mut k) else {
        return k;
    };
    let _ = l.format();
    for c in 0..l.samples().len().min(2) {
        if let Some(ix) = note(l.index(c), &mut k).map(<[_]>::to_vec) {
            for j in [0, ix.len() / 2, ix.len().saturating_sub(1)] {
                if j < ix.len() {
                    note(l.scan(c, j), &mut k);
                    note(l.scan_header(c, j), &mut k);
                }
            }
        }
        if let Some(cr) = note(l.chromatograms(c), &mut k)
            && let Some(v) = cr.first()
        {
            note(l.chromatogram(c, &v.id), &mut k);
        }
        note(l.method(c), &mut k);
        note(
            readers_mzml::scrivi(&mut *l, c, Vec::new(), &mut |_, _| {}),
            &mut k,
        );
    }
    k
}

fn execute_mzml(m: &[(String, Vec<u8>)]) -> Vec<String> {
    let mut k = Vec::new();
    note(mzml::read(&m[0].1[..]), &mut k);
    k
}

fn execute_mzidx(m: &[(String, Vec<u8>)]) -> Vec<String> {
    let mut k = Vec::new();
    let size = m[0].1.len() as u64;
    let Some(mut ix) = note(Index::open(Letture(m[0].1.clone()), size), &mut k) else {
        return k;
    };
    for si in 0..ix.sections.len().min(8) {
        note(ix.arrays(si), &mut k);
        note(ix.mz_range(si), &mut k);
        note(ix.search(si, 100.0, false), &mut k);
        note(ix.sum_window(si, 0, 50), &mut k);
        note(ix.max_window(si, 0, 50), &mut k);
    }
    k
}

/// The container of a `.wiff` and the methods read from it (the `.dam` parser).
fn execute_ole(m: &[(String, Vec<u8>)]) -> Vec<String> {
    let mut k = Vec::new();
    if let Some(ole) = note(
        Ole::open(
            m[0].1.clone(),
            "x.wiff",
            &mzlab_core::readers::Limits::held(),
        ),
        &mut k,
    ) {
        let paths: Vec<String> = ole.paths_list().map(String::from).collect();
        paths.iter().take(64).for_each(|p| drop(ole.read(p)));
        let _ = ole.sha1_hex();
    }
    note(
        wiff::container(m[0].1.clone(), "x.wiff").map(|o| wiff::samples(&o)),
        &mut k,
    );
    note(methodinfo::read_methods(m[0].1.clone(), "x.wiff"), &mut k);
    k
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A synthetic `.wiff` pair from `tools/wiff_riferimento.py`; `None` without python3.
fn wiff_pair() -> Option<&'static Members> {
    static C: OnceLock<Option<Members>> = OnceLock::new();
    C.get_or_init(|| {
        let out = std::env::temp_dir().join(format!("mzlab_robustezza_{}", std::process::id()));
        let ok = Command::new("python3")
            .arg(root().join("tools/wiff_riferimento.py"))
            .arg(&out)
            .output()
            .ok()
            .filter(|o| o.status.success())?;
        drop(ok);
        let leggi = |n: &str| std::fs::read(out.join(n)).ok();
        let pair = (|| {
            Some(vec![
                ("one.wiff".into(), leggi("one.wiff")?),
                ("one.wiff.scan".into(), leggi("one.wiff.scan")?),
            ])
        })();
        let _ = std::fs::remove_dir_all(&out);
        pair
    })
    .as_ref()
}

fn agilent_path() -> Members {
    let (idx, peak) = encode(&sample());
    vec![
        ("x.D/AcqData/MSScan.bin".into(), idx),
        ("x.D/AcqData/MSPeak.bin".into(), peak),
    ]
}

/// Magic of the format followed by a body of pseudo-random bytes (a real file cannot be built without the vendor's writer).
fn with_magic(name: &str, magic: &[u8], corpo: usize) -> Members {
    let mut b = magic.to_vec();
    let mut r = Rng(0xA5A5_1234_DEAD_BEEF);
    b.extend((0..corpo).map(|_| r.next() as u8));
    vec![(name.into(), b)]
}

fn trial(name: &str, seme: &Members, text: &[bool], execute: Execute) {
    let (cases, guasti) = corri(seme, text, execute);
    verify(name, cases, &guasti);
}

#[test]
fn the_valid_seeds_are_read() {
    let mut set = fileset(&agilent_path());
    assert_eq!(
        readers::riconosci(&mut set).map(|v| v.id),
        Some("agilent-d")
    );
    assert!(
        run_registry(&agilent_path()).is_empty(),
        "the Agilent seed must read without errors"
    );
    assert!(mzml::read(MZML).is_ok());
    assert!(execute_mzidx(&[("a.mzidx".into(), MZIDX.to_vec())]).is_empty());
    if let Some(w) = wiff_pair() {
        assert_eq!(
            readers::riconosci(&mut fileset(w)).map(|v| v.id),
            Some("sciex-wiff")
        );
    }
}

#[test]
fn agilent_d_folder() {
    trial("agilent-d", &agilent_path(), &[false, false], run_registry);
}

#[test]
fn agilent_data_ms() {
    let seme = with_magic("x.D/DATA.MS", &[0x01, 0x32], 2000);
    trial("agilent DATA.MS", &seme, &[false], run_registry);
}

#[test]
fn thermo_raw() {
    let magic = [
        0x01, 0xA1, b'F', 0, b'i', 0, b'n', 0, b'n', 0, b'i', 0, b'g', 0, b'a', 0, b'n', 0,
    ];
    let seme = with_magic("x.raw", &magic, 6000);
    trial("thermo-raw", &seme, &[false], run_registry);
}

#[test]
fn sciex_wiff_pair() {
    let Some(w) = wiff_pair() else {
        return eprintln!("SKIP robustezza wiff: no python3");
    };
    trial("sciex-wiff", w, &[false, false], run_registry);
}

#[test]
fn ole_container_and_methods() {
    let Some(w) = wiff_pair() else {
        return eprintln!("SKIP robustezza ole: no python3");
    };
    trial("ole", &w[..1].to_vec(), &[false], execute_ole);
}

#[test]
fn mzml_text() {
    trial(
        "mzml",
        &vec![("msn.mzML".into(), MZML.to_vec())],
        &[true],
        execute_mzml,
    );
}

#[test]
fn mzidx_index() {
    trial(
        "mzidx",
        &vec![("msn.mzidx".into(), MZIDX.to_vec())],
        &[false],
        execute_mzidx,
    );
}

/// The bench itself: a reader that panics, allocates by a count of the file or answers with a key of its own is caught.
#[test]
fn the_bench_catches_a_panic_a_big_allocation_and_a_foreign_key() {
    fn cattivo(m: &[(String, Vec<u8>)]) -> Vec<String> {
        let b = &m[0].1;
        match b.len() {
            0 => vec!["boom".into()],
            1 => {
                let v = vec![1u8; 100 << 20];
                vec![format!("err.big{}", v.is_empty())]
            }
            2 => panic!("short file"),
            _ => Vec::new(),
        }
    }
    let seme = vec![("x".to_string(), vec![7u8; 64])];
    let (_, guasti) = corri(&seme, &[false], cattivo);
    for tema in ["without err.* prefix", "memory peak", "panic short file"] {
        assert!(
            guasti.iter().any(|g| g.contains(tema)),
            "not caught: {tema}\n{guasti:#?}"
        );
    }
}
