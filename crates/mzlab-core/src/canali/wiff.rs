//! SCIEX `.wiff` scans in the channels: the abbreviation of the experiment (EMS, EPI) and the IDA grouping. A scan of a `.wiff` (or of its msconvert mzML)
//! has no filter, only the id `sample= period= cycle= experiment=`; the scan table names the experiment (`wiff/modi.rs`), else the level tells it.
//! Used by `gruppi::{sigla_di, id_ms}`.

use crate::idxread::Scan;
use crate::wiff::modi::{self, CODE_FULL, CODE_PRODUCT};

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
    modi::sigla(Some(if s.level >= 2 {
        CODE_PRODUCT
    } else {
        CODE_FULL
    }))
}

/// The product-ion scan of an IDA experiment: the method gives it no precursor, which the survey scan picks at every cycle.
pub(super) fn dipendente(s: &Scan) -> bool {
    s.level == 2 && sigla(s) == Some("EPI") && s.prec(f64::NAN).is_nan()
}

/// The channel of the dependent scans of an IDA: `dd:2:EPI`.
pub(super) const ID_IDA: &str = "dd:2:EPI";
