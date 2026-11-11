//! The synthetic Orbitrap v66 `.raw` of `examples/thermo_synthetic.rs` for the tests: written into a temporary folder and opened as the site and the app open it.
#![allow(dead_code)]
use mzlab_core::readers::{FileSet, Limits, Member};
use std::path::{Path, PathBuf};
use std::process::Command;

/// The built example next to the test binary (`cargo test` builds it), else `cargo run`.
pub fn generate(out: &Path, args: &[&str]) {
    let ex = std::env::current_exe()
        .ok()
        .and_then(|e| Some(e.parent()?.parent()?.join("examples/thermo_synthetic")))
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
                "thermo_synthetic",
                "--",
            ])
            .arg(out)
            .args(args)
            .status(),
    };
    assert!(st.is_ok_and(|s| s.success()), "thermo_synthetic failed");
}

pub fn folder(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "mzlab_thermo_synthetic_{}_{name}",
        std::process::id()
    ));
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// A file opened as the site and the app open it.
pub fn reader_of(b: Vec<u8>) -> Box<dyn mzlab_core::readers::Reader> {
    let set = FileSet {
        membri: vec![Member {
            nome: "hr.raw".into(),
            dim: b.len() as u64,
            dati: Box::new(b),
            sha1: None,
        }],
    };
    mzlab_lettori::apri(set, &Limits::sito()).expect("open")
}
