//! The one place that knows where the private data repository (mzlab-dati) is and how it is laid out (Rust twin of `dati_percorsi.py`).
//! Included by the tests with `#[path = ".../tools/dati_percorsi.rs"] mod dati_percorsi;`
#![allow(dead_code)]

use std::path::{Path, PathBuf};

// A folder is the data repository when it has the data index.
const MARKERS: [&str; 1] = ["dati/INDICE.md"];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// The data repository, or None: `MZLAB_DATI` / `QQQ_DATI`, then a sibling folder named `mzlab-dati` (or the old name).
pub fn radice() -> Option<PathBuf> {
    let mut cands: Vec<PathBuf> = ["MZLAB_DATI", "QQQ_DATI"]
        .iter()
        .filter_map(|v| std::env::var_os(v).map(PathBuf::from))
        .collect();
    for up in root().ancestors().skip(1).take(4) {
        cands.push(up.join("mzlab-dati"));
        cands.push(up.join("QqQ-lab-dati"));
    }
    cands.push(PathBuf::from("/home/user/mzlab-dati"));
    cands
        .into_iter()
        .find(|c| MARKERS.iter().any(|m| c.join(m).exists()))
}

/// Existing path for `rel`, or None when there is no data repository or no such path.
pub fn percorso(rel: &str) -> Option<PathBuf> {
    let p = radice()?.join(rel);
    p.exists().then_some(p)
}
