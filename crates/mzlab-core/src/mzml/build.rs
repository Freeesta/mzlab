//! A spectrum or chromatogram of the engine's mzML reader built from its header as the shared reader (`spechead`) gives it: level, RT in minutes
//! for every unit, the immediate precursor, the arrays with their number type (refused when the file does not say it) and an SRM's Q1/Q3 from its
//! isolation windows.

use super::{mobility_of, Array, ArrayKind, Chrom, Spec};
use crate::error::{Error, Result};
use crate::numarray::Num;
use crate::spechead::{self, ArrayHead, Head, RtUnit, Stage, Term};
use std::collections::HashMap;

/// Where a damaged item is named: its id, else its 1-based position.
fn who(h: &Head, n: usize) -> String {
    if h.id.is_empty() {
        (h.index.unwrap_or(n) + 1).to_string()
    } else {
        h.id.clone()
    }
}

/// The array of `a`: its kind (the first m/z, intensity, time or mobility term), number type, zlib flag and the minutes per time unit.
fn array(a: ArrayHead, item: &str) -> Result<Array> {
    let t = &a.terms;
    let mut kind = ArrayKind::Other;
    let mut mobility = "";
    for term in &t.0 {
        let k = match term.acc.as_str() {
            "MS:1000514" => ArrayKind::Mz,
            "MS:1000515" => ArrayKind::Intensity,
            "MS:1000595" => ArrayKind::Time,
            acc => match mobility_of(acc) {
                Some(u) => {
                    mobility = u;
                    ArrayKind::Mobility
                }
                None => continue,
            },
        };
        kind = k;
        break;
    }
    // numpress alone and followed by zlib: read as plain floats, a zlib variant gives garbage or an infinity
    if t.0.iter().any(|x| crate::numpress::ACCESSIONS.contains(&x.acc.as_str())) {
        return Err(Error::new("err.file.numpress"));
    }
    let num = t.0.iter().find_map(|x| Num::of_accession(&x.acc));
    let has_data = !a.text.trim().is_empty();
    let num = match num {
        Some(n) => n,
        // 32-bit data read as 64-bit is garbage: a file that does not say its number type is refused
        None if has_data && kind != ArrayKind::Other => {
            return Err(Error::new("err.file.mzmlPrecision").with("scan", item));
        }
        None => Num::F64,
    };
    let minutes = t.get("MS:1000595").map_or(1.0, |x| RtUnit::of(x).minutes());
    Ok(Array {
        kind,
        num,
        zlib: t.has("MS:1000574"),
        minutes,
        mobility,
        text: a.text,
    })
}

fn arrays(h: &mut Head, item: &str) -> Result<Vec<Array>> {
    std::mem::take(&mut h.arrays)
        .into_iter()
        .map(|a| array(a, item))
        .collect()
}

/// The spectrum of header `h` (the `n`-th of the file), `None` for an optical spectrum (`mzml_dad` reads those). `levels` gives the level of the
/// spectra read so far, by id, for the immediate precursor of an MSn scan.
pub(super) fn spectrum(mut h: Head, n: usize, levels: &HashMap<String, u8>) -> Result<Option<Spec>> {
    if h.terms.has("MS:1000804") {
        return Ok(None);
    }
    let item = who(&h, n);
    let level = h.level();
    let loose;
    let stages: &[Stage] = if h.precursors.is_empty() && (h.terms.has("MS:1000744") || h.terms.has("MS:1000827")) {
        loose = [Stage::loose(&h.terms)];
        &loose
    } else {
        &h.precursors
    };
    let keys: Vec<_> = stages
        .iter()
        .map(|s| (s.target(), s.selected(), s.refer.as_deref()))
        .collect();
    let precursor = spechead::immediate(&keys, level, None, |r| levels.get(r).copied())
        .and_then(|i| stages.get(i))
        .and_then(|s| s.selected().or(s.target()));
    // the scan's own mobility first, then one a precursor's selected ion carries
    let mobility_scan = h
        .terms
        .0
        .iter()
        .chain(h.precursors.iter().flat_map(|s| s.ion.0.iter().chain(&s.window.0)))
        .find_map(|t: &Term| mobility_of(&t.acc).zip(t.float()).map(|(u, v)| (v, u)));
    Ok(Some(Spec {
        index: h.index.unwrap_or(n),
        len: h.length.unwrap_or(0),
        level,
        rt: h.rt_minutes(),
        polarity: h.polarity(),
        profile: h.profile(),
        filter: h.filter().to_string(),
        precursor,
        mobility_scan,
        arrays: arrays(&mut h, &item)?,
        id: h.id,
    }))
}

/// The number after `key` in an id ("SRM SIC Q1=400.2 Q3=150.1").
pub(super) fn number_after(id: &str, key: &str) -> Option<f64> {
    let k = id.find(key)? + key.len();
    let rest = &id[k..];
    let end = rest
        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

/// The chromatogram of header `h` (the `n`-th of the file).
pub(super) fn chromatogram(mut h: Head, n: usize) -> Result<Chrom> {
    let item = who(&h, n);
    // every term of the chromatogram, its arrays and its precursor and product decides its kind (as in `mzlab/reader/mzml.py`)
    let all = || {
        h.terms
            .0
            .iter()
            .chain(h.arrays.iter().flat_map(|a| a.terms.0.iter()))
            .chain(
                h.precursors
                    .iter()
                    .chain(&h.products)
                    .flat_map(|s| s.window.0.iter().chain(&s.ion.0).chain(&s.activation.0)),
            )
    };
    let any = |accs: &[&str]| all().any(|t| accs.contains(&t.acc.as_str()));
    let unit = h
        .arrays
        .iter()
        .find_map(|a| a.terms.get("MS:1000515"))
        .map_or(String::new(), |t| t.unit_name.clone());
    let pda = any(&["MS:1000812", "MS:1000813"]);
    let pressure = any(&["MS:1003019"]);
    let srm = any(&["MS:1001473"]);
    let ms_term = any(&["MS:1000235", "MS:1000627", "MS:1000628", "MS:1001473", "MS:1001472"]);
    // Q1 and Q3 are the targets of the precursor and product isolation windows; an id in the msconvert form is the fallback
    let q1 = h.precursors.first().and_then(Stage::target);
    let q3 = h.products.first().and_then(Stage::target);
    Ok(Chrom {
        index: h.index.unwrap_or(n),
        len: h.length.unwrap_or(0),
        arrays: arrays(&mut h, &item)?,
        unit,
        pda,
        pressure,
        ms_term,
        srm,
        q1,
        q3,
        id: h.id,
    })
}
