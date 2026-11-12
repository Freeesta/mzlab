//! SCIEX `.wiff` scans in the channels: the abbreviation of the experiment (EMS, EPI) and the IDA grouping. A scan of a `.wiff` (or of its msconvert mzML)
//! has no filter, only the id `sample= period= cycle= experiment=`; the scan table names the experiment (`wiff/modes.rs`), else the level tells it.
//! Used by `groups::{code_of, id_ms, group}`.

use std::collections::{HashMap, HashSet};

use crate::idxread::Scan;
use crate::wiff::modes::{self, CODE_FULL, CODE_PRODUCT};

/// The id of an acquisition experiment of a `.wiff`: `sample=` first, then `period=`, `cycle=` and `experiment=`.
fn id_wiff(native: &str) -> bool {
    native.starts_with("sample=")
        && ["period=", "cycle=", "experiment="]
            .iter()
            .all(|k| native.contains(k))
}

/// The abbreviation of the scan type of a `.wiff` scan: the experiment the scan table names (`Scan::scan_type`: EPI, PI, EMS, Q1), else by level
/// (an msconvert mzML does not name it): full scan (EMS) at level 1, EPI at level 2.
pub(super) fn sigla(s: &Scan) -> Option<&'static str> {
    if !id_wiff(&s.native) {
        return None;
    }
    if let Some(t) = s.scan_type() {
        return Some(t);
    }
    modes::tag(Some(if s.level >= 2 {
        CODE_PRODUCT
    } else {
        CODE_FULL
    }))
}

/// The experiment of a `.wiff` scan without its cycle: `sample=1 period=1 experiment=2`.
fn experiment_of(native: &str) -> String {
    native
        .split_whitespace()
        .filter(|w| !w.starts_with("cycle="))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The EPI experiments of the scans whose precursor changes from cycle to cycle (to 0.1 mDa): an IDA picks it from the survey at every cycle, a targeted
/// EPI keeps the one of its method. Read from the scans alone, so an mzML from msconvert is grouped as the reader's own; a scan with a filter string
/// (another maker's, which names its precursor) is never one, nor an experiment of a file with no survey experiment beside it.
pub(super) fn varying(scans: &[Scan]) -> HashSet<String> {
    let wiff = |s: &&Scan| s.filter.is_empty() && sigla(s).is_some();
    let surveys: HashSet<String> = scans
        .iter()
        .filter(wiff)
        .filter(|s| s.level == 1)
        .map(|s| experiment_of(&s.native))
        .collect();
    let mut first: HashMap<String, i64> = HashMap::new();
    let mut out = HashSet::new();
    for s in scans
        .iter()
        .filter(wiff)
        .filter(|s| s.level == 2 && sigla(s) == Some("EPI"))
    {
        let p = s.prec(f64::NAN);
        if p.is_nan() {
            continue;
        }
        let q = (p * 1e4).round() as i64;
        let e = experiment_of(&s.native);
        if *first.entry(e.clone()).or_insert(q) != q {
            out.insert(e);
        }
    }
    // an IDA picks from a survey of its own experiment: without one the precursors are the targets of a fixed method
    out.retain(|e| surveys.iter().any(|v| v != e));
    out
}

/// The product-ion scan of an IDA experiment: no precursor (the file did not give it), or one of an experiment whose precursor varies (`varying`).
pub(super) fn dipendente(s: &Scan, varying: &HashSet<String>) -> bool {
    s.level == 2
        && sigla(s) == Some("EPI")
        && (s.prec(f64::NAN).is_nan() || varying.contains(&experiment_of(&s.native)))
}

/// The channel of the dependent scans of an IDA: `dd:2:EPI`.
pub(super) const ID_IDA: &str = "dd:2:EPI";
