//! `Item.scan_params` of `mzlab/explore.py`: for the window «Metodo» of a high-resolution / DDA file that has no `.dam`, what its scans say about how
//! they were acquired (scan window, resolving power, injection time, activation, collision energy, isolation width, scans per cycle). Written as
//! `,"scan_params":{...}` at the end of `Item.method()`; nothing for a file that is not high resolution. The physical file counts, whichever part the item is.

use crate::idxread::{Index, ReadAt};
use crate::iteminfo::{group_an, group_is_hr};
use crate::itemroutes::cv_value;
use crate::json::{round, Out};
use crate::Result;

fn med(mut v: Vec<f64>) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    crate::sortc::by(&mut v, f64::total_cmp);
    let n = v.len();
    Some(if n % 2 == 1 { v[n / 2] } else { (v[n / 2 - 1] + v[n / 2]) / 2.0 })
}

/// The numbers `acc` takes in the headers of the scans sampled at about 300 (`sample` and `vals` of Python).
fn vals<M: ReadAt>(sc: &[&crate::idxread::Scan], acc: &str, mzml: &mut M) -> Result<Vec<f64>> {
    let mut out = Vec::new();
    for s in sc.iter().step_by((sc.len() / 300).max(1)) {
        let h = crate::sorgente::testo_scansione(mzml, s)?;
        let cut = h.find("<binaryDataArrayList").unwrap_or(h.len().saturating_sub(1));
        if let Some(v) = cv_value(&h[..cut], acc).and_then(|v| v.trim().parse::<f64>().ok()) {
            out.push(v);
        }
    }
    Ok(out)
}

fn sorted_set(v: impl Iterator<Item = f64>) -> Vec<f64> {
    let mut v: Vec<f64> = v.collect();
    crate::sortc::by(&mut v, f64::total_cmp);
    v.dedup();
    v
}

fn extremes(v: &[f64]) -> (Option<f64>, Option<f64>) {
    (v.iter().copied().reduce(f64::min), v.iter().copied().reduce(f64::max))
}

pub(crate) fn write<R: ReadAt, M: ReadAt>(idx: &Index<R>, mzml: &mut M, instrument: &str, o: &mut Out) -> Result<()> {
    let sc1: Vec<&crate::idxread::Scan> = idx.scans.iter().filter(|s| s.level == 1).collect();
    let sc2: Vec<&crate::idxread::Scan> = idx.scans.iter().filter(|s| s.level > 1).collect();
    if sc1.is_empty() || !(group_is_hr(&idx.scans, true) || group_is_hr(&idx.scans, false)) {
        return Ok(());
    }
    let res = |sc: &[&crate::idxread::Scan]| med(sc.iter().filter_map(|s| s.res).filter(|r| *r != 0.0).collect());
    let (lo, hi) = (vals(&sc1, "MS:1000501", mzml)?, vals(&sc1, "MS:1000500", mzml)?);
    o.raw(",\"scan_params\":{\"ms1\":{\"n\":").int(sc1.len() as i64).raw(",\"window\":");
    match (extremes(&lo).0, extremes(&hi).1) {
        (Some(a), Some(b)) if !lo.is_empty() && !hi.is_empty() => o.raw("[").num(a).raw(",").num(b).raw("]"),
        _ => o.raw("null"),
    };
    o.raw(",\"res\":").opt(res(&sc1)).raw(",\"inject\":").opt(med(vals(&sc1, "MS:1000927", mzml)?)).raw(",\"an\":").str(&group_an(&idx.scans, true)).raw("},\"ms2\":");
    if sc2.is_empty() {
        o.raw("null");
    } else {
        let acts: Vec<String> = {
            let mut a: Vec<String> = sc2.iter().filter_map(|s| s.act.clone()).filter(|a| !a.is_empty()).collect();
            a.sort();
            a.dedup();
            a
        };
        let nce = idx.run.get("nce").and_then(|v| v.bool()).unwrap_or(false);
        let ces = sorted_set(sc2.iter().filter_map(|s| s.collision_energy));
        let iso: Vec<f64> = sorted_set(sc2.iter().filter_map(|s| s.iso).map(|(a, b)| round((b - a) / 2.0, 3))).into_iter().take(4).collect();
        let lo2 = vals(&sc2[..sc2.len().min(50)], "MS:1000501", mzml)?;
        o.raw("{\"n\":").int(sc2.len() as i64).raw(",\"per_cycle\":").num(round(sc2.len() as f64 / sc1.len() as f64, 2)).raw(",\"act\":[");
        for (i, a) in acts.iter().enumerate() {
            o.raw(if i > 0 { "," } else { "" }).str(a);
        }
        o.raw("],\"nce\":").raw(if nce { "true" } else { "false" }).raw(",\"ce\":").floats(&ces).raw(",\"iso\":").floats(&iso);
        o.raw(",\"res\":").opt(res(&sc2)).raw(",\"inject\":").opt(med(vals(&sc2, "MS:1000927", mzml)?)).raw(",\"an\":").str(&group_an(&idx.scans, false));
        o.raw(",\"window_lo\":").opt(extremes(&lo2).0).raw("}");
    }
    o.raw(",\"instrument\":").str(instrument).raw("}");
    Ok(())
}
