//! Robustness of the readers of files (`docs/agenti/test.md`): for every format a valid seed and its deterministic mutations (truncations, flipped bytes,
//! counts and lengths at their maximum, zeros, an empty file, a valid head and a random body) give an `err.*` key or a reading, never a panic, a hang or an
//! allocation out of proportion to the file. Synthetic seeds only (the `.wiff` ones need python3 and skip without it); no `MZLAB_DATI`.
//! `ROBUSTEZZA_TRACCIA=1 cargo test -p mzlab-core --test robustezza_comune -- --nocapture` prints every case before it runs (to find the one that aborts).

mod agilent_synth;
mod robustezza;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use agilent_synth::{encode, sample};
use mzlab_core::idxread::Index;
use mzlab_core::lettori::{self, Limiti};
use mzlab_core::ole::Ole;
use mzlab_core::{lettori_mzml, methodinfo, mzml, wiff};
use robustezza::*;

#[global_allocator]
static ALLOCATORE: Contatore = Contatore;

type Membri = Vec<(String, Vec<u8>)>;

const MZML: &[u8] = include_bytes!("../../../tests/golden/ref/msntree_rotta/msn.mzML");
const MZIDX: &[u8] = include_bytes!("../../../tests/golden/ref/msntree_rotta/msn.mzidx");

/// Everything a reader of the registry offers, on whatever the set is: the keys of the errors it met.
fn esegui_registro(m: &[(String, Vec<u8>)]) -> Vec<String> {
    let mut k = Vec::new();
    let _ = lettori::riconosci(&mut fileset(m));
    let Some(mut l) = nota(lettori::apri(fileset(m), &Limiti::sito()), &mut k) else {
        return k;
    };
    let _ = l.formato();
    for c in 0..l.campioni().len().min(2) {
        if let Some(ix) = nota(l.indice(c), &mut k).map(<[_]>::to_vec) {
            for j in [0, ix.len() / 2, ix.len().saturating_sub(1)] {
                if j < ix.len() {
                    nota(l.scansione(c, j), &mut k);
                    nota(l.intestazione(c, j), &mut k);
                }
            }
        }
        if let Some(cr) = nota(l.cromatogrammi(c), &mut k) {
            if let Some(v) = cr.first() {
                nota(l.cromatogramma(c, &v.id), &mut k);
            }
        }
        nota(l.metodo(c), &mut k);
        nota(
            lettori_mzml::scrivi(&mut *l, c, Vec::new(), &mut |_, _| {}),
            &mut k,
        );
    }
    k
}

fn esegui_mzml(m: &[(String, Vec<u8>)]) -> Vec<String> {
    let mut k = Vec::new();
    nota(mzml::read(&m[0].1[..]), &mut k);
    k
}

fn esegui_mzidx(m: &[(String, Vec<u8>)]) -> Vec<String> {
    let mut k = Vec::new();
    let size = m[0].1.len() as u64;
    let Some(mut ix) = nota(Index::open(Letture(m[0].1.clone()), size), &mut k) else {
        return k;
    };
    for si in 0..ix.sections.len().min(8) {
        nota(ix.arrays(si), &mut k);
        nota(ix.mz_range(si), &mut k);
        nota(ix.search(si, 100.0, false), &mut k);
        nota(ix.sum_window(si, 0, 50), &mut k);
        nota(ix.max_window(si, 0, 50), &mut k);
    }
    k
}

/// The container of a `.wiff` and the methods read from it (the `.dam` parser).
fn esegui_ole(m: &[(String, Vec<u8>)]) -> Vec<String> {
    let mut k = Vec::new();
    if let Some(ole) = nota(Ole::open(m[0].1.clone(), "x.wiff"), &mut k) {
        let percorsi: Vec<String> = ole.paths_list().map(String::from).collect();
        percorsi.iter().take(64).for_each(|p| drop(ole.read(p)));
        let _ = ole.sha1_hex();
    }
    nota(
        wiff::container(m[0].1.clone(), "x.wiff").map(|o| wiff::samples(&o)),
        &mut k,
    );
    nota(methodinfo::read_methods(m[0].1.clone(), "x.wiff"), &mut k);
    k
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A synthetic `.wiff` pair from `tools/wiff_riferimento.py`; `None` without python3.
fn coppia_wiff() -> Option<&'static Membri> {
    static C: OnceLock<Option<Membri>> = OnceLock::new();
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
        let coppia = (|| {
            Some(vec![
                ("one.wiff".into(), leggi("one.wiff")?),
                ("one.wiff.scan".into(), leggi("one.wiff.scan")?),
            ])
        })();
        let _ = std::fs::remove_dir_all(&out);
        coppia
    })
    .as_ref()
}

fn percorso_agilent() -> Membri {
    let (idx, peak) = encode(&sample());
    vec![
        ("x.D/AcqData/MSScan.bin".into(), idx),
        ("x.D/AcqData/MSPeak.bin".into(), peak),
    ]
}

/// Magic of the format followed by a body of pseudo-random bytes (a real file cannot be built without the vendor's writer).
fn con_magic(nome: &str, magic: &[u8], corpo: usize) -> Membri {
    let mut b = magic.to_vec();
    let mut r = Rng(0xA5A5_1234_DEAD_BEEF);
    b.extend((0..corpo).map(|_| r.next() as u8));
    vec![(nome.into(), b)]
}

fn prova(nome: &str, seme: &Membri, testo: &[bool], esegui: Esegui) {
    let (casi, guasti) = corri(seme, testo, esegui);
    verifica(nome, casi, &guasti);
}

#[test]
fn the_valid_seeds_are_read() {
    let mut set = fileset(&percorso_agilent());
    assert_eq!(
        lettori::riconosci(&mut set).map(|v| v.id),
        Some("agilent-d")
    );
    assert!(
        esegui_registro(&percorso_agilent()).is_empty(),
        "the Agilent seed must read without errors"
    );
    assert!(mzml::read(MZML).is_ok());
    assert!(esegui_mzidx(&[("a.mzidx".into(), MZIDX.to_vec())]).is_empty());
    if let Some(w) = coppia_wiff() {
        assert_eq!(
            lettori::riconosci(&mut fileset(w)).map(|v| v.id),
            Some("sciex-wiff")
        );
    }
}

#[test]
fn agilent_d_folder() {
    prova(
        "agilent-d",
        &percorso_agilent(),
        &[false, false],
        esegui_registro,
    );
}

#[test]
fn agilent_data_ms() {
    let seme = con_magic("x.D/DATA.MS", &[0x01, 0x32], 2000);
    prova("agilent DATA.MS", &seme, &[false], esegui_registro);
}

#[test]
fn thermo_raw() {
    let magic = [
        0x01, 0xA1, b'F', 0, b'i', 0, b'n', 0, b'n', 0, b'i', 0, b'g', 0, b'a', 0, b'n', 0,
    ];
    let seme = con_magic("x.raw", &magic, 6000);
    prova("thermo-raw", &seme, &[false], esegui_registro);
}

#[test]
fn sciex_wiff_pair() {
    let Some(w) = coppia_wiff() else {
        return eprintln!("SKIP robustezza wiff: no python3");
    };
    prova("sciex-wiff", w, &[false, false], esegui_registro);
}

#[test]
fn ole_container_and_methods() {
    let Some(w) = coppia_wiff() else {
        return eprintln!("SKIP robustezza ole: no python3");
    };
    prova("ole", &w[..1].to_vec(), &[false], esegui_ole);
}

#[test]
fn mzml_text() {
    prova(
        "mzml",
        &vec![("msn.mzML".into(), MZML.to_vec())],
        &[true],
        esegui_mzml,
    );
}

#[test]
fn mzidx_index() {
    prova(
        "mzidx",
        &vec![("msn.mzidx".into(), MZIDX.to_vec())],
        &[false],
        esegui_mzidx,
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
