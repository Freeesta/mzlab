//! The synthetic vendor files of the tests of `mzlab-cli convert`: an mzXML and a netCDF (the encoders of the tests of `mzlab-lettori`, included
//! from there), a folder of a maker that no reader takes, and a `.wiff` + `.wiff.scan` pair made by `tools/wiff_grande.py` (skipped without python3).

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::lettori::{FileSet, Membro};

#[path = "../../mzlab-lettori/tests/comune/andi.rs"]
pub mod andi;
#[path = "../../mzlab-lettori/tests/comune/mzxml.rs"]
pub mod mzxml;

/// The helpers the included encoders ask of their parent: base64, zlib (the files here are written without it) and a one-file dataset.
pub fn b64(b: &[u8]) -> String {
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

pub fn zlib(b: &[u8]) -> Vec<u8> {
    b.to_vec()
}

pub fn insieme(nome: &str, byte: Vec<u8>) -> FileSet {
    FileSet {
        membri: vec![Membro {
            nome: nome.into(),
            dim: byte.len() as u64,
            dati: Box::new(byte),
            sha1: None,
        }],
    }
}

/// A new empty folder of the test `tag`.
pub fn folder(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_cli_convert_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// `in/` with `a.mzXML`, `b.cdf` and, with `broken`, `rotto.raw` (bytes no reader knows).
pub fn makers(dir: &Path, broken: bool) -> PathBuf {
    let src = dir.join("in");
    std::fs::create_dir_all(&src).unwrap();
    let o = mzxml::Opz {
        zlib: false,
        ..mzxml::PIATTO
    };
    std::fs::write(src.join("a.mzXML"), mzxml::mzxml(&mzxml::corsa(), o)).unwrap();
    std::fs::write(src.join("b.cdf"), andi::andi_ms(&andi::PIATTA)).unwrap();
    if broken {
        std::fs::write(
            src.join("rotto.raw"),
            b"not a raw file of any maker".repeat(20),
        )
        .unwrap();
    }
    src
}

/// `name.wiff` and `name.wiff.scan` with `samples` samples in `src`; false without python3 and numpy (`MZLAB_GOLDEN_STRICT=1` fails instead).
pub fn wiff(src: &Path, name: &str, samples: usize) -> bool {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let ok = Command::new("python3")
        .arg(root.join("tools/wiff_grande.py"))
        .arg(src)
        .args(["--name", name, "--samples", &samples.to_string()])
        .args(["--scans", "12", "--points", "40"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !ok {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 or numpy missing"
        );
        eprintln!("skipped: python3/numpy not available");
    }
    ok
}
