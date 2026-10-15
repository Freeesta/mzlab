//! What an mzML contains, read from the file itself and not from its name (`explore.sniff` and `App._sniff` of `mzlab/explore.py`,
//! `mzlab/app.py`): kind, number of scans, SRM, polarity, mixed parts, spectrum mode, "heavy high-resolution profile" and the hint of the name.

use crate::error::Result;
use crate::idxread::{Index, ReadAt};
use crate::iteminfo::{file_parts, item_info, Facts, Meta};
use crate::json::{self, Out};
use crate::mzmlhead::Mzml;
use crate::sorgente::Scansioni;

/// The hint the file name gives: "infusion", "mrm", "ms2" or "full" (the first rule that matches the lower-case stem).
pub fn hint(file_name: &str) -> Option<&'static str> {
    let base = file_name.rsplit('/').next().unwrap_or(file_name);
    let stem = match base.rfind('.') {
        Some(i) if i > 0 && i + 1 < base.len() => &base[..i],
        _ => base,
    };
    let n = stem.to_lowercase();
    let has = |ws: &[&str]| ws.iter().any(|w| n.contains(w));
    if has(&["infus", "fia", "direct", "syringe"]) {
        Some("infusion")
    } else if has(&["mrm", "srm"]) {
        Some("mrm")
    } else if has(&["ms2", "msms", "ms-ms", "product", "epi"]) {
        Some("ms2")
    } else if has(&["full", "q1", "ems", "scan"]) {
        Some("full")
    } else {
        None
    }
}

/// `True` when the first MS1 scan is a PROFILE spectrum of a high-resolution analyzer (Orbitrap, TOF): the heaviest kind of mzML.
fn hr_profile<R: ReadAt, M: Scansioni + ?Sized>(
    idx: &Index<R>,
    src: &mut M,
    instrument: &str,
) -> Result<bool> {
    let Some(s) = idx.scans.iter().find(|s| s.level == 1) else {
        return Ok(false);
    };
    if !s.profile {
        return Ok(false);
    }
    let up = s.filter.to_uppercase();
    if up.starts_with("FTMS") || up.starts_with("TOFMS") {
        return Ok(true);
    }
    // _cv(head, "MS:1000800"): the value of the resolution of the scan header
    let res = src.intestazione(s)?;
    if let Some(f) = res.iter().find(|f| f.cv.as_deref() == Some("MS:1000800")) {
        if f.valore.trim().parse::<f64>().is_ok_and(|r| r >= 10000.0) {
            return Ok(true);
        }
    }
    let low = instrument.to_lowercase();
    Ok([
        "orbitrap", "exploris", "exactive", "astral", "tof", "ft-icr", "ltq ft",
    ]
    .iter()
    .any(|w| low.contains(w)))
}

/// `{kind, scans, srm, polarity, mixed, spectrum_mode, hr_profile, hint, msn}` of a whole file (`msn`: `infusione.rs`).
pub fn sniff<R: ReadAt, M: ReadAt>(
    idx: &mut Index<R>,
    m: &mut Mzml<M>,
    facts: &Facts,
    file_name: &str,
) -> Result<String> {
    let info = item_info(
        idx,
        facts,
        &Meta {
            file: file_name.into(),
            typ: "sample".into(),
            ..Default::default()
        },
        None,
    )?;
    let v = json::parse(info.as_bytes())?;
    let parts = file_parts(&idx.scans, facts.srm > 0);
    let mut prec: Vec<i64> = idx
        .scans
        .iter()
        .filter(|s| s.level >= 2)
        .filter_map(|s| s.precursor.filter(|p| *p != 0.0))
        .map(|p| p.round_ties_even() as i64)
        .collect();
    crate::sortc::slice_ord(&mut prec);
    prec.dedup();
    let hrp = hr_profile(idx, &mut m.r, &facts.instrument)?;
    let mut o = Out::default();
    o.raw("{\"kind\":")
        .str(v.get("kind").and_then(|x| x.str()).unwrap_or(""))
        .raw(",\"scans\":")
        .int(v.get("scans").and_then(|x| x.f64()).unwrap_or(0.0) as i64);
    o.raw(",\"srm\":")
        .int(facts.srm as i64)
        .raw(",\"polarity\":")
        .str(v.get("polarity").and_then(|x| x.str()).unwrap_or(""))
        .raw(",\"mixed\":[");
    for (i, p) in parts.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"name\":")
            .str(match p.mode {
                "ms1" => "Full Scan",
                "ms2" => "MS2",
                _ => "MRM",
            });
        o.raw(",\"pol\":");
        match p.pol {
            Some(v) => o.int(v as i64),
            None => o.raw("null"),
        };
        o.raw(",\"nprec\":");
        if p.mode == "ms2" && !prec.is_empty() {
            o.int(prec.len() as i64);
        } else {
            o.raw("null");
        }
        o.raw("}");
    }
    o.raw("],\"spectrum_mode\":")
        .str(v.get("spectrum_mode").and_then(|x| x.str()).unwrap_or(""))
        .raw(",\"hr_profile\":")
        .raw(if hrp { "true" } else { "false" });
    o.raw(",\"hint\":");
    match hint(file_name) {
        Some(h) => o.str(h),
        None => o.raw("null"),
    };
    let msn = crate::infusione::infusione(&idx.scans, facts.lc, file_name);
    crate::infusione::json(&mut o, msn.as_ref());
    o.raw("}");
    Ok(o.0)
}
