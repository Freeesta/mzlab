// Ported from Attributio readers/ms2.py.
//! MS/MS spectra for the confirmation of Level 1 against the in-house reference-standard library. Two inputs: the feature spectra (the MGF that mzMine exports
//! for GNPS/FBMN from the merged alignment, `merged_<pol>_GNPS.mgf`, one MS2 block per feature, FEATURE_ID = id of the merged table) and the standard spectra
//! (the mzMine spectral library in JSON lines, built from injections of the standards mix on the same instrument and fragmentation). The similarity between
//! two spectra is not here: it belongs to the matching of the review (`mzlab-lib`).

use crate::attributio::campaign::Campaign;
use crate::attributio::error::{CampaignError, Result};
use crate::attributio::io::{join, Io};
use crate::sortc::SortC;
use mzlab_core::json::{self, Value};
use std::collections::HashMap;

/// A centroid spectrum with its precursor m/z (infinite when unknown).
#[derive(Clone, Debug, PartialEq)]
pub struct Spectrum {
    pub mz: Vec<f64>,
    pub intensity: Vec<f64>,
    pub precursor: f64,
}

/// The FBMN MGF of the merged alignment of a polarity (`merged_<pol>_gnps.mgf` of the merged folder, matched without regard to case).
pub fn feature_mgf(c: &Campaign, io: &Io, pol: &str) -> Option<String> {
    let want = format!("merged_{}_gnps.mgf", pol.to_lowercase());
    let dir = c.merged_dir(pol);
    if !io.is_dir(&dir) {
        return None;
    }
    let mut names: Vec<String> = io.list(&dir).into_iter().map(|e| e.name).collect();
    names.sortc();
    names
        .into_iter()
        .find(|n| n.to_lowercase() == want)
        .map(|n| join(&dir, &n))
}

/// Python `float()` of a text.
fn py_float(s: &str) -> Option<f64> {
    s.trim().parse::<f64>().ok()
}

/// `int(float(v))`, `None` when it fails.
fn py_int(s: &str) -> Option<i64> {
    py_float(s).filter(|v| v.is_finite()).map(|v| v as i64)
}

#[derive(Default)]
struct Block {
    fid: Option<i64>,
    prec: Option<f64>,
    level: i64,
    mz: Vec<f64>,
    it: Vec<f64>,
}

impl Block {
    fn new() -> Block {
        Block {
            level: 2,
            ..Block::default()
        }
    }
}

fn add_header(b: &mut Block, line: &str) {
    let (k, v) = line.split_once('=').unwrap_or((line, ""));
    match k.to_uppercase().as_str() {
        "FEATURE_ID" => b.fid = py_int(v).or(b.fid),
        "PEPMASS" => b.prec = v.split_whitespace().next().and_then(py_float).or(b.prec),
        "MSLEVEL" => b.level = py_int(v).unwrap_or(b.level),
        _ => {}
    }
}

fn add_peak(b: &mut Block, line: &str) {
    let mut p = line.split_whitespace();
    if let (Some(a), Some(i)) = (p.next(), p.next()) {
        if let (Some(a), Some(i)) = (py_float(a), py_float(i)) {
            b.mz.push(a);
            b.it.push(i);
        }
    }
}

/// `{feature id: spectrum}` for the MS2 blocks of an MGF; of several blocks with one id the one with the most peaks stays.
pub fn read_feature_spectra(text: &str) -> HashMap<i64, Spectrum> {
    let mut out: HashMap<i64, Spectrum> = HashMap::new();
    let mut cur = Block::new();
    for raw in text.replace("\r\n", "\n").replace('\r', "\n").split('\n') {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if line == "BEGIN IONS" {
            cur = Block::new();
        } else if line == "END IONS" {
            if let (Some(fid), 2, false) = (cur.fid, cur.level, cur.mz.is_empty()) {
                if out.get(&fid).is_none_or(|s| cur.mz.len() > s.mz.len()) {
                    let b = std::mem::replace(&mut cur, Block::new());
                    out.insert(
                        fid,
                        Spectrum {
                            mz: b.mz,
                            intensity: b.it,
                            precursor: b.prec.unwrap_or(f64::INFINITY),
                        },
                    );
                }
            }
        } else if line.contains('=') && !line.starts_with(|c: char| c.is_ascii_digit()) {
            add_header(&mut cur, line);
        } else {
            add_peak(&mut cur, line);
        }
    }
    out
}

fn peaks_of(v: &Value) -> Option<Vec<(f64, f64)>> {
    let parsed;
    let arr = match v {
        Value::Str(s) => {
            parsed = json::parse(s.as_bytes()).ok()?;
            &parsed
        }
        other => other,
    };
    let Value::Arr(rows) = arr else { return None };
    rows.iter()
        .map(|p| match p {
            Value::Arr(xy) if xy.len() >= 2 => match (&xy[0], &xy[1]) {
                (Value::Num(a), Value::Num(b)) => Some((*a, *b)),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

/// A library of reference spectra: `{compound name: spectra}`, in file order within a name.
pub type RefLibrary = HashMap<String, Vec<Spectrum>>;

/// The mzMine JSON-lines spectral library: one spectrum per line (`compound_name`, `precursor_mz`, `peaks` as a list or as the text of one).
pub fn read_library_spectra(text: &str) -> Result<RefLibrary> {
    let mut out: RefLibrary = HashMap::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let d = json::parse(line.as_bytes()).map_err(|_| {
            CampaignError::value(format!(
                "JSONDecodeError: line {} of the spectral library is not valid JSON",
                n + 1
            ))
        })?;
        let Some(peaks) = d.get("peaks").and_then(peaks_of).filter(|p| !p.is_empty()) else {
            continue;
        };
        let precursor = match d.get("precursor_mz") {
            Some(Value::Num(v)) if *v != 0.0 => *v,
            Some(Value::Str(s)) => py_float(s).unwrap_or(f64::INFINITY),
            _ => f64::INFINITY,
        };
        let name = match d.get("compound_name") {
            Some(Value::Str(s)) => s.trim().to_string(),
            Some(Value::Num(v)) => format!("{v}"),
            _ => String::new(),
        };
        out.entry(name).or_default().push(Spectrum {
            mz: peaks.iter().map(|p| p.0).collect(),
            intensity: peaks.iter().map(|p| p.1).collect(),
            precursor,
        });
    }
    Ok(out)
}

/// The number of spectra in a library.
pub fn library_size(lib: &RefLibrary) -> usize {
    lib.values().map(Vec::len).sum()
}
