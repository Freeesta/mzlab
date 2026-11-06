#![cfg(feature = "thermo")]
//! The synthetic Orbitrap v66 writer (`examples/thermo_sintetico.rs`, the stand-in for the speed benches of high-resolution `.raw`) gives a file the reader opens:
//! the scan count, the MS1/MS2 cycle, profile MS1 with centroids at the m/z it wrote, MS2 with a precursor. Small file in a temporary folder, removed afterwards.
use mzlab_core::lettori::{FileSet, Limiti, Membro};
use std::path::{Path, PathBuf};
use std::process::Command;

/// The built example next to the test binary (`cargo test` builds it), else `cargo run`.
fn genera(out: &Path, args: &[&str]) {
    let ex = std::env::current_exe()
        .ok()
        .and_then(|e| Some(e.parent()?.parent()?.join("examples/thermo_sintetico")))
        .filter(|p| p.is_file());
    let st = match ex {
        Some(p) => Command::new(p).arg(out).args(args).status(),
        None => Command::new(env!("CARGO"))
            .args([
                "run",
                "--release",
                "-q",
                "-p",
                "mzlab-lettori",
                "--features",
                "thermo",
                "--example",
                "thermo_sintetico",
                "--",
            ])
            .arg(out)
            .args(args)
            .status(),
    };
    assert!(st.is_ok_and(|s| s.success()), "thermo_sintetico failed");
}

fn cartella(nome: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "mzlab_thermo_sintetico_{}_{nome}",
        std::process::id()
    ));
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn the_synthetic_orbitrap_file_opens_with_its_cycle() {
    let d = cartella("ciclo");
    let f = d.join("hr.raw");
    genera(
        &f,
        &[
            "--scans",
            "66",
            "--peaks",
            "40",
            "--ms2",
            "10",
            "--ms2-peaks",
            "12",
            "--trailer",
            "30",
            "--seed",
            "7",
        ],
    );
    let b = std::fs::read(&f).unwrap();
    let _ = std::fs::remove_dir_all(&d);
    let set = FileSet {
        membri: vec![Membro {
            nome: "hr.raw".into(),
            dim: b.len() as u64,
            dati: Box::new(b),
            sha1: None,
        }],
    };
    let mut l = mzlab_lettori::apri(set, &Limiti::sito()).expect("open");
    let ix = l.indice(0).expect("index").to_vec();
    assert_eq!(ix.len(), 66);
    for (k, v) in ix.iter().enumerate() {
        assert_eq!(v.livello, if k % 11 == 0 { 1 } else { 2 }, "scan {k}");
        assert_eq!(v.polarita, 1);
        assert!(v.filtro.starts_with("FTMS"), "{}", v.filtro);
        assert_eq!(v.precursore.is_some(), v.livello == 2, "scan {k}");
    }
    for k in [0usize, 11, 55] {
        let s = l.scansione(0, k).expect("ms1");
        assert_eq!(s.level, 1);
        assert_eq!(s.mz.len(), 40, "the centroid list of the packet");
        assert!(s.mz.windows(2).all(|w| w[0] < w[1]));
        assert!(s.mz.iter().all(|&m| (100.0..=1500.0).contains(&m)));
        let p = l
            .scansione_profilo(0, k)
            .expect("profile")
            .expect("an MS1 profile");
        assert!(p.mz.len() > s.mz.len());
    }
    let s = l.scansione(0, 1).expect("ms2");
    assert_eq!(s.level, 2);
    assert_eq!(s.mz.len(), 12);
    assert!(s.precursor.is_some_and(|p| p > 150.0));
}

#[test]
fn the_same_seed_writes_the_same_bytes() {
    let d = cartella("seme");
    let (a, b) = (d.join("a.raw"), d.join("b.raw"));
    for f in [&a, &b] {
        genera(
            f,
            &[
                "--scans", "24", "--peaks", "30", "--ms2", "3", "--seed", "3",
            ],
        );
    }
    let (x, y) = (std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(x, y);
}
