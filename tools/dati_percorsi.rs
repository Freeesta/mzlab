//! The one place that knows where the private data repository (mzlab-dati) is and how it is laid out (Rust twin of `dati_percorsi.py`).
//! Included by the tests with `#[path = ".../tools/dati_percorsi.rs"] mod dati_percorsi;`; reads `dati_mappa.tsv`, so both languages share the layout.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

const MAPPA: &str = include_str!("dati_mappa.tsv");
// A folder is the data repository when it has the new index or one of the legacy top folders.
const MARKERS: [&str; 6] = [
    "dati/INDICE.md",
    "mzML",
    "dati_formati",
    "3200 qtrap",
    "HRMS",
    "scratchpad",
];

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

/// Existing path for `rel` (new layout, else legacy), or None when there is no data repository or no such path.
pub fn percorso(rel: &str) -> Option<PathBuf> {
    let r = radice()?;
    if r.join(rel).exists() {
        return Some(r.join(rel));
    }
    let mut righe: Vec<(&str, &str)> = MAPPA
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| l.split_once('\t'))
        .collect();
    righe.sort_by_key(|(n, _)| std::cmp::Reverse(n.len()));
    for (nuovo, vecchio) in righe {
        if rel == nuovo || rel.strip_prefix(nuovo).is_some_and(|t| t.starts_with('/')) {
            let p = r.join(format!("{vecchio}{}", &rel[nuovo.len()..]));
            return p.exists().then_some(p);
        }
    }
    None
}
