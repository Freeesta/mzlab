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

// The data repository was tidied (reorganize-step1): folders moved. (old prefix, new prefix, name test); `percorso`
// tries the path as asked, then the moved/unmoved twin, so code and data can merge in any order.
type Move = (&'static str, &'static str, fn(&str) -> bool);
const MOVES: &[Move] = &[
    ("esperimenti/", "esperimenti/2026-10/", |n| {
        n.starts_with("loc_") || n.starts_with("coo_")
    }),
    ("esperimenti/", "esperimenti/altro/", |n| {
        ["Computazio", "Sostanze psicoattive", "motore_wasm"].contains(&n)
    }),
    ("coordinamento/", "coordinamento/documenti/", |n| {
        [
            "studi",
            "proposte",
            "consegne",
            "riferimenti",
            "gemini_esiti",
        ]
        .contains(&n)
    }),
    ("coordinamento/prompt/raw", "prompt/raw", |_| true),
    // step 2: book, validation and study folders
    ("Libro_Untargeted", "libro/testo", |_| true),
    ("coordinamento/libro_untargeted", "libro/piano", |_| true),
    ("corso/untargeted", "libro/lezioni", |_| true),
    ("studio/benchmark", "validazione/benchmark", |_| true),
    ("studio/validazione/", "validazione/", |n| {
        [
            "DATI_REALI_dam.md",
            "DATI_REALI_mzfinder_pipeline.md",
            "DATI_REALI_parita_pr320_pr319.md",
            "DATI_REALI_raw_thermo.md",
            "DATI_REALI_rotte_lr_hr.md",
            "DATI_REALI_wiff.md",
            "G1",
            "H1_K1_timololo.md",
            "RUOLI_TEMPI.md",
            "V0_timololo.md",
            "V1_sintetici.md",
            "VERITA_TP_schema.md",
        ]
        .contains(&n)
    }),
    (
        "studio/nts-validazione",
        "validazione/nts-validazione",
        |_| true,
    ),
    ("studio/", "coordinamento/documenti/studi/", |n| {
        [
            "AUDIT_USO_LIMITI.md",
            "MZMINE_FATTIBILITA.md",
            "PIPELINE_IDENTITA.md",
            "PIPELINE_IDENTITA_PACCHETTI.md",
            "CD",
        ]
        .contains(&n)
    }),
    ("studio/paper", "coordinamento/documenti/paper", |_| true),
    ("studio/degdma", "esperimenti/altro/degdma", |_| true),
];

fn first_name(rest: &str, prefix: &str) -> String {
    if prefix.ends_with('/') {
        rest.split('/').next().unwrap_or("").to_string()
    } else {
        String::new()
    }
}

/// The other spellings of `rel` (old layout <-> new layout).
fn gemelli(rel: &str) -> Vec<String> {
    let mut out = Vec::new();
    for &(old, new, ok) in MOVES {
        if let Some(rest) = rel.strip_prefix(old) {
            if ok(&first_name(rest, old)) {
                out.push(format!("{new}{rest}"));
            }
        } else if let Some(rest) = rel.strip_prefix(new)
            && ok(&first_name(rest, new))
        {
            out.push(format!("{old}{rest}"));
        }
    }
    out
}

/// Existing path for `rel` (or its moved/unmoved twin), or None when there is no data repository or no such path.
pub fn percorso(rel: &str) -> Option<PathBuf> {
    let r = radice()?;
    std::iter::once(rel.to_string())
        .chain(gemelli(rel))
        .map(|c| r.join(c))
        .find(|p| p.exists())
}
