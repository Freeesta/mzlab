//! What `screen.suspects` reads beside the table: the MS2 spectra of the MGF of `spectra.ms2` (fragment evidence) and the candidate formulas of `formulas/1` (Level 4).

use crate::sortc::SortC;
use mzlab_core::elements;
use mzlab_core::error::Result;
use mzlab_core::json::{self, Value};

/// One MS2 scan of the MGF.
pub struct Scan {
    /// `FEATURE_ID` when the MGF says it (a `spectra.ms2` given the feature table), else empty.
    pub feature: String,
    pub precursor: f64,
    /// Minutes.
    pub rt: f64,
    pub mz: Vec<f64>,
}

/// The MS2 scans of an MGF text, in file order (`PEPMASS`, `RTINSECONDS` in seconds, `FEATURE_ID`, peaks as `m/z intensity`).
pub fn read_scans(mgf: &[u8]) -> Vec<Scan> {
    let text = String::from_utf8_lossy(mgf);
    let mut scans = Vec::new();
    let mut cur: Option<Scan> = None;
    for line in text.lines().map(str::trim) {
        if line == "BEGIN IONS" {
            cur = Some(Scan {
                feature: String::new(),
                precursor: 0.0,
                rt: 0.0,
                mz: Vec::new(),
            });
        } else if line == "END IONS" {
            scans.extend(cur.take().filter(|s| s.precursor > 0.0 && !s.mz.is_empty()));
        } else if let Some(s) = cur.as_mut() {
            if let Some((k, v)) = line.split_once('=') {
                let first = v.split_whitespace().next().unwrap_or("");
                match k.trim().to_ascii_uppercase().as_str() {
                    "PEPMASS" => s.precursor = first.parse().unwrap_or(0.0),
                    "RTINSECONDS" => s.rt = first.parse::<f64>().unwrap_or(0.0) / 60.0,
                    "FEATURE_ID" => s.feature = v.trim().to_string(),
                    _ => {}
                }
            } else if let Some(m) = line
                .split_whitespace()
                .next()
                .and_then(|v| v.parse::<f64>().ok())
            {
                s.mz.push(m);
            }
        }
    }
    scans
}

/// The fragment m/z values of the MS2 of one feature: the scans that name it, else the scans whose precursor is within `tol_da` of `mz` and whose RT is within `rt_tol` min of `rt`.
pub fn fragments_of(
    scans: &[Scan],
    id: &str,
    mz: f64,
    rt: f64,
    tol_da: f64,
    rt_tol: f64,
) -> Vec<f64> {
    let named = scans.iter().any(|s| s.feature == id);
    scans
        .iter()
        .filter(|s| {
            if named {
                s.feature == id
            } else {
                s.feature.is_empty()
                    && (s.precursor - mz).abs() <= tol_da
                    && (s.rt - rt).abs() <= rt_tol
            }
        })
        .flat_map(|s| s.mz.iter().copied())
        .collect()
}

/// The listed fragments found in `peaks` within `ppm` (the listed value is returned, in the order of the list).
pub fn matched(listed: &[f64], peaks: &[f64], ppm: f64) -> Vec<f64> {
    listed
        .iter()
        .copied()
        .filter(|f| peaks.iter().any(|p| (p - f).abs() <= f * ppm * 1e-6))
        .collect()
}

/// The candidate formulas of a `formulas/1` file as `(feature id, Hill formula)`.
/// Sorted and without repeats, to be searched with `supports`.
pub fn formula_set(files: &[&[u8]]) -> Result<Vec<(String, String)>> {
    let mut set = Vec::new();
    for f in files {
        let v = json::parse(f)?;
        for feat in v.get("features").map(Value::arr).unwrap_or(&[]) {
            let id = feat
                .get("id")
                .and_then(Value::str)
                .map(String::from)
                .or_else(|| feat.get("id").and_then(Value::f64).map(|n| format!("{n}")))
                .unwrap_or_default();
            for c in feat
                .get("result")
                .and_then(|r| r.get("candidates"))
                .map(Value::arr)
                .unwrap_or(&[])
            {
                let text = c
                    .get("neutral")
                    .and_then(Value::str)
                    .or_else(|| c.get("formula").and_then(Value::str));
                if let Some(t) = text {
                    let hill = elements::parse_formula(t)
                        .map(|f| elements::fmt(&f))
                        .unwrap_or_else(|_| t.to_string());
                    set.push((id.clone(), hill));
                }
            }
        }
    }
    set.sortc();
    set.dedup();
    Ok(set)
}

/// Whether `formulas/1` has the formula among the candidates of the feature.
pub fn supports(set: &[(String, String)], id: &str, formula: &str) -> bool {
    set.binary_search_by(|(i, f)| (i.as_str(), f.as_str()).cmp(&(id, formula)))
        .is_ok()
}
