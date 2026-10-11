//! What an mzML contains, read from the file itself and not from its name (`explore.sniff` and `App._sniff` of `mzlab/explore.py`,
//! `mzlab/app.py`): kind, number of scans, SRM, polarity, mixed parts, spectrum mode, "heavy high-resolution profile" and the hint of the name.

use crate::error::Result;
use crate::idxread::{Index, ReadAt};
use crate::iteminfo::{file_parts, item_info, Facts, Meta};
use crate::json::{self, Out};
use crate::mzmlhead::Mzml;

/// The hint the file name gives: "mrm", "ms2" or "full" (the first rule that matches the lower-case stem).
pub fn hint(file_name: &str) -> Option<&'static str> {
    let base = file_name.rsplit('/').next().unwrap_or(file_name);
    let stem = match base.rfind('.') {
        Some(i) if i > 0 && i + 1 < base.len() => &base[..i],
        _ => base,
    };
    let n = stem.to_lowercase();
    let has = |ws: &[&str]| ws.iter().any(|w| n.contains(w));
    if has(&["mrm", "srm"]) {
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
fn hr_profile<R: ReadAt, M: ReadAt>(
    idx: &Index<R>,
    m: &mut Mzml<M>,
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
    let head = crate::scaninfo::header(&mut m.r, s.start, s.end)?;
    // _cv(head, "MS:1000800"): accession="MS:1000800"[^>]*?value="([^"]*)"
    let key = "accession=\"MS:1000800\"";
    if let Some(p) = head.find(key) {
        let t = &head[p + key.len()..];
        let tag_end = t.find('>').unwrap_or(t.len());
        if let Some(v) = t[..tag_end].find("value=\"") {
            let val = &t[v + 7..];
            if let Some(e) = val.find('"') {
                if let Ok(r) = val[..e].trim().parse::<f64>() {
                    if r >= 10000.0 {
                        return Ok(true);
                    }
                }
            }
        }
    }
    let low = instrument.to_lowercase();
    Ok([
        "orbitrap", "exploris", "exactive", "astral", "tof", "ft-icr", "ltq ft",
    ]
    .iter()
    .any(|w| low.contains(w)))
}

/// `{kind, scans, srm, polarity, mixed, spectrum_mode, hr_profile, hint}` of a whole file.
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
    prec.sort();
    prec.dedup();
    let hrp = hr_profile(idx, m, &facts.instrument)?;
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
    o.raw("}");
    Ok(o.0)
}
