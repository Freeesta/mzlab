//! The Bruker timsTOF reader (TDF with MS1 and PASEF frames, TSF line spectra) against the mutations of the shared bench of `mzlab-core`: no panic, hang or
//! allocation out of proportion to the folder, and every error a key of the catalogue whose text does not show the reader's raw detail (it stays after the key
//! for scripts). Synthetic folders only.
#![cfg(feature = "bruker")]

mod common_sqlite;
#[path = "../../mzlab-core/tests/robustness/mod.rs"]
mod robustness;

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

use common_sqlite::*;
use mzlab_core::error::{Error, Result};
use mzlab_core::readers::Limits;
use robustness::*;

#[global_allocator]
static ALLOCATORE: Counter = Counter;

type Members = Vec<(String, Vec<u8>)>;

/// Every key met by the cases of the running hunt, with one full error as an example.
static MET: Mutex<BTreeMap<String, String>> = Mutex::new(BTreeMap::new());

fn keep<T>(r: Result<T>, keys: &mut Vec<String>) -> Option<T> {
    r.map_err(|e: Error| {
        MET.lock()
            .unwrap()
            .entry(e.key.to_string())
            .or_insert_with(|| e.to_string());
        keys.push(e.key.to_string());
    })
    .ok()
}

/// Open, index, three spectra and their headers, the first trace, the mzML writer.
fn read_all(m: &[(String, Vec<u8>)]) -> Vec<String> {
    let mut k = Vec::new();
    let Some(mut l) = keep(mzlab_lettori::apri(fileset(m), &Limits::sito()), &mut k) else {
        return k;
    };
    if let Some(ix) = keep(l.index(0), &mut k).map(<[_]>::to_vec) {
        for j in [0, ix.len() / 2, ix.len().saturating_sub(1)] {
            if j < ix.len() {
                keep(l.scan(0, j), &mut k);
                keep(l.scan_header(0, j), &mut k);
            }
        }
    }
    if let Some(cr) = keep(l.chromatograms(0), &mut k)
        && let Some(v) = cr.first()
    {
        keep(l.chromatogram(0, &v.id), &mut k);
    }
    keep(
        mzlab_core::readers_mzml::scrivi(l.as_mut(), 0, Vec::new(), &mut |_, _| {}),
        &mut k,
    );
    k
}

/// The English text of `key` in the catalogue of the page, if it has one.
fn catalogue_text(key: &str) -> Option<String> {
    static CAT: OnceLock<String> = OnceLock::new();
    let cat = CAT.get_or_init(|| {
        let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mzlab/web/lang/en/errori.js");
        std::fs::read_to_string(p).unwrap_or_default()
    });
    let at = cat.find(&format!("\"{key}\":"))? + key.len() + 3;
    let rest = cat[at..].trim_start().strip_prefix('"')?;
    Some(rest[..rest.find("\",\n")?].to_string())
}

fn hunt(name: &str, seed: &Members) {
    // one hunt at a time: `MET` is shared by the tests, which run in parallel
    static ONE: Mutex<()> = Mutex::new(());
    let _g = ONE.lock().unwrap_or_else(|p| p.into_inner());
    MET.lock().unwrap().clear();
    let (cases, mut failures) = corri(seed, &vec![false; seed.len()], read_all);
    let met = std::mem::take(&mut *MET.lock().unwrap());
    for (key, example) in &met {
        match catalogue_text(key) {
            None => failures.push(format!(
                "{name}: {key} has no text in the catalogue ({example})"
            )),
            Some(t) if t.contains("{message}") || t.contains("{detail}") => failures.push(format!(
                "{name}: {key} shows the reader's raw detail to the user ({example})"
            )),
            Some(_) => {}
        }
    }
    eprintln!("{name}: keys met {met:?}");
    verify(name, cases, &failures);
}

fn tdf_seed() -> Members {
    vec![
        ("x.d/analysis.tdf".into(), minimal_tdf(3)),
        ("x.d/analysis.tdf_bin".into(), block2()),
    ]
}

fn pasef_seed() -> Members {
    let (tdf, bin) = pasef::folder(2, 6, 3);
    vec![
        ("x.d/analysis.tdf".into(), tdf),
        ("x.d/analysis.tdf_bin".into(), bin),
    ]
}

/// Two TSF frames of three lines each: `u32` profile size 0, `u32` line size, then the zstd of three `f64` TOF positions and three `f32` intensities.
fn tsf_seed() -> Members {
    let mut bin = Vec::new();
    let mut at = Vec::new();
    for f in 0..2u32 {
        at.push(bin.len() as i64);
        let mut lines = Vec::new();
        for t in [20_000.0f64, 30_000.0, 40_000.0] {
            lines.extend((t + f64::from(f)).to_le_bytes());
        }
        for y in [10.0f32, 500.0, 40.0] {
            lines.extend((y * (f + 1) as f32).to_le_bytes());
        }
        let z = raw_zstd(&lines);
        bin.extend(0u32.to_le_bytes());
        bin.extend((z.len() as u32).to_le_bytes());
        bin.extend(z);
    }
    vec![
        ("x.d/analysis.tsf".into(), tdf_frames(&at, 3)),
        ("x.d/analysis.tsf_bin".into(), bin),
    ]
}

#[test]
fn the_seeds_read_whole() {
    for (name, seed) in [
        ("tdf", tdf_seed()),
        ("pasef", pasef_seed()),
        ("tsf", tsf_seed()),
    ] {
        MET.lock().unwrap().clear();
        assert!(
            read_all(&seed).is_empty(),
            "{name}: {:?}",
            MET.lock().unwrap()
        );
        let mut l = mzlab_lettori::apri(fileset(&seed), &Limits::sito()).unwrap();
        assert!(!l.scan(0, 0).unwrap().mz.is_empty(), "{name}");
    }
}

#[test]
fn tdf_mutated() {
    hunt("bruker tdf", &tdf_seed());
}

#[test]
fn pasef_mutated() {
    hunt("bruker pasef", &pasef_seed());
}

#[test]
fn tsf_mutated() {
    hunt("bruker tsf", &tsf_seed());
}
