//! The vendor raw readers (Agilent `.D` LC/MS, SCIEX `.wiff` pairs of every variant `tools/wiff_riferimento.py` writes) against the mutations of the shared bench:
//! no panic, hang or allocation out of proportion to the file, and every error a key of the catalogue whose text names what happened without the reader's raw
//! detail (`{detail}` stays after the key for scripts, never in the sentence the page shows). Synthetic seeds only; the `.wiff` ones skip without python3.

#![cfg(feature = "estesi")]
#![allow(clippy::disallowed_methods)] // tests compare against the standard sorts

mod agilent_synth;
mod robustness;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

use agilent_synth::lc;
use mzlab_core::error::{Error, Result};
use mzlab_core::readers::{self, Limits};
use mzlab_core::readers_mzml;
use robustness::*;

#[global_allocator]
static ALLOCATORE: Counter = Counter;

type Members = Vec<(String, Vec<u8>)>;

/// Every key met by every case, with one full error as an example.
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

/// Everything the page and the CLI read from a set: open, index, three scans and their headers, the first trace, the method, the mzML writer.
fn read_all(m: &[(String, Vec<u8>)]) -> Vec<String> {
    let mut k = Vec::new();
    let _ = readers::riconosci(&mut fileset(m));
    let Some(mut l) = keep(readers::apri(fileset(m), &Limits::sito()), &mut k) else {
        return k;
    };
    for c in 0..l.samples().len().min(3) {
        if let Some(ix) = keep(l.index(c), &mut k).map(<[_]>::to_vec) {
            for j in [0, ix.len() / 2, ix.len().saturating_sub(1)] {
                if j < ix.len() {
                    keep(l.scan(c, j), &mut k);
                    keep(l.scan_header(c, j), &mut k);
                }
            }
        }
        if let Some(cr) = keep(l.chromatograms(c), &mut k)
            && let Some(v) = cr.first()
        {
            keep(l.chromatogram(c, &v.id), &mut k);
        }
        keep(l.method(c), &mut k);
        keep(
            readers_mzml::scrivi(&mut *l, c, Vec::new(), &mut |_, _| {}),
            &mut k,
        );
    }
    k
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The English text of `key` in the catalogue of the page, if it has one.
fn catalogue_text(key: &str) -> Option<String> {
    static CAT: OnceLock<String> = OnceLock::new();
    let cat = CAT.get_or_init(|| {
        std::fs::read_to_string(root().join("mzlab/web/lang/en/errori.js")).unwrap_or_default()
    });
    let at = cat.find(&format!("\"{key}\":"))? + key.len() + 3;
    let rest = cat[at..].trim_start().strip_prefix('"')?;
    Some(rest[..rest.find("\",\n")?].to_string())
}

/// Runs the bench on `seed` and then judges the keys met: in the catalogue, and without the raw detail in the sentence.
fn hunt(name: &str, seed: &Members) {
    // one hunt at a time: `MET` is shared by the tests, which run in parallel
    static ONE: Mutex<()> = Mutex::new(());
    let _g = ONE.lock().unwrap_or_else(|p| p.into_inner());
    MET.lock().unwrap().clear();
    let text = vec![false; seed.len()];
    let (cases, mut failures) = corri(seed, &text, read_all);
    let met = std::mem::take(&mut *MET.lock().unwrap());
    for (key, example) in &met {
        match catalogue_text(key) {
            None => failures.push(format!(
                "{name}: {key} has no text in the catalogue ({example})"
            )),
            Some(t) if t.contains("{detail}") => failures.push(format!(
                "{name}: {key} shows the reader's raw detail to the user ({example})"
            )),
            Some(_) => {}
        }
    }
    eprintln!("{name}: keys met {:?}", met.keys().collect::<Vec<_>>());
    verify(name, cases, &failures);
}

const XSD: &str = "x.D/AcqData/MSScan.xsd";

/// A QqQ run of three MRM visits in `MSPeak.bin` (format 3: `n` m/z as f64 then `n` intensities as f32) with its method.
fn agilent_qqq() -> Members {
    let pts = [(150.1f64, 1200.0f32), (220.2, 40.5), (305.3, 9.0)];
    let mut peak = Vec::new();
    let mut recs = Vec::new();
    for id in 1..=3 {
        let at = peak.len() as i64;
        pts.iter().for_each(|p| peak.extend(p.0.to_le_bytes()));
        pts.iter()
            .for_each(|p| peak.extend((p.1 * id as f32).to_le_bytes()));
        recs.push(lc::record(id, 256, 3, at, 36, 3, 0));
    }
    vec![
        ("x.D/AcqData/MSScan.bin".into(), lc::index(&recs)),
        ("x.D/AcqData/MSPeak.bin".into(), peak),
        (XSD.into(), lc::xsd("DataDependentScanParamType")),
        (
            "x.D/AcqData/AcqMethod.xml".into(),
            b"<RCDevicesXml>&lt;Section&gt;&lt;ParameterName&gt;Gas Temp&lt;/ParameterName&gt;&lt;Value&gt;300&lt;/Value&gt;&lt;/Section&gt;</RCDevicesXml>".to_vec(),
        ),
    ]
}

/// A Q-TOF run of two centroid scans (format 2: flight times, turned into m/z by the default calibration).
fn agilent_qtof() -> Members {
    let mut peak = Vec::new();
    let mut recs = Vec::new();
    for id in 1..=2 {
        let at = peak.len() as i64;
        for t in [40_000.0f64, 52_000.0, 61_000.0] {
            peak.extend((t + id as f64).to_le_bytes());
        }
        for y in [10.0f32, 300.0, 25.0] {
            peak.extend(y.to_le_bytes());
        }
        recs.push(lc::record(id, 1, 2, at, 36, 3, 0));
    }
    vec![
        ("x.D/AcqData/MSScan.bin".into(), lc::index(&recs)),
        ("x.D/AcqData/MSPeak.bin".into(), peak),
        (XSD.into(), lc::xsd("DataDependentScanParamType")),
        (
            "x.D/AcqData/DefaultMassCal.xml".into(),
            b"<DefaultMassCal><DefaultCalibration DefaultCalibrationID=\"1\"><Step StepNumber=\"1\"><CalibrationFormula>Traditional</CalibrationFormula><Value Number=\"1\">0.1</Value><Value Number=\"2\">0</Value></Step></DefaultCalibration></DefaultMassCal>".to_vec(),
        ),
    ]
}

/// The `.wiff` pairs `tools/wiff_riferimento.py` writes, by stem; empty without python3.
fn wiff_pairs() -> &'static BTreeMap<String, Members> {
    static C: OnceLock<BTreeMap<String, Members>> = OnceLock::new();
    C.get_or_init(|| {
        let out = std::env::temp_dir().join(format!("mzlab_vendors_{}", std::process::id()));
        let ok = Command::new("python3")
            .arg(root().join("tools/wiff_riferimento.py"))
            .arg(&out)
            .output()
            .ok()
            .is_some_and(|o| o.status.success());
        let mut pairs = BTreeMap::new();
        if ok {
            for stem in PAIRS {
                let read = |n: String| std::fs::read(out.join(&n)).ok().map(|b| (n, b));
                if let (Some(w), Some(s)) = (
                    read(format!("{stem}.wiff")),
                    read(format!("{stem}.wiff.scan")),
                ) {
                    pairs.insert(stem.to_string(), vec![w, s]);
                }
            }
        }
        let _ = std::fs::remove_dir_all(&out);
        pairs
    })
}

/// One pair per variant the reader serves: full and product scans, MRM, MRM with collision energies, scheduled MRM, periods, the TOF block layout, the v13
/// blocks, a DAD method, device traces.
const PAIRS: [&str; 10] = [
    "one",
    "two",
    "transizioni",
    "energia",
    "programmati",
    "periodi",
    "bloccotof",
    "blocchi",
    "dadmetodo",
    "dispositivi",
];

#[test]
fn the_seeds_read_whole() {
    for (name, seed) in [("qqq", agilent_qqq()), ("qtof", agilent_qtof())] {
        MET.lock().unwrap().clear();
        let mut set = fileset(&seed);
        assert_eq!(
            readers::riconosci(&mut set).map(|v| v.id),
            Some("agilent-d"),
            "{name}"
        );
        let k = read_all(&seed);
        assert!(k.is_empty(), "{name}: {:?}", MET.lock().unwrap());
        let mut l = readers::apri(fileset(&seed), &Limits::sito()).unwrap();
        let ix = l.index(0).unwrap().to_vec();
        // the MRM visits of the QqQ become traces, the Q-TOF centroids scans
        let read = if name == "qqq" {
            l.chromatograms(0).unwrap().len() > 1
        } else {
            !ix.is_empty() && !l.scan(0, 0).unwrap().mz.is_empty()
        };
        assert!(read, "{name}");
    }
    for (stem, pair) in wiff_pairs() {
        assert_eq!(
            readers::riconosci(&mut fileset(pair)).map(|v| v.id),
            Some("sciex-wiff"),
            "{stem}"
        );
    }
}

#[test]
fn agilent_qqq_mutated() {
    hunt("agilent qqq", &agilent_qqq());
}

#[test]
fn agilent_qtof_mutated() {
    hunt("agilent qtof", &agilent_qtof());
}

#[test]
fn sciex_pairs_mutated() {
    let pairs = wiff_pairs();
    if pairs.is_empty() {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 not available"
        );
        return eprintln!("SKIP robustness_vendors wiff: no python3");
    }
    for (stem, pair) in pairs {
        hunt(&format!("sciex {stem}"), pair);
    }
}
