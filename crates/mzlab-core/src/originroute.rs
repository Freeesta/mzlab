//! `api/origin` ("Da dove viene questo ione?") over the files of the session, from their indexes: `App.origin` of Python.
//! Any full-scan file with an index (unit or high resolution, centroids or profile points); `Ok(None)` = Python's (a file without index).

use crate::hash::Mappa;

use crate::error::{Error, Result};
use crate::idxread::{Index, ReadAt};
use crate::iteminfo::group_is_hr;
use crate::json::Value;
use crate::jsonb::{floats, Obj};
use crate::origin::{origin_report, Ms2, Request, Sample};
use crate::routes::Routes;

/// One item of the session, in session order. `full` = a full-scan file that is not a blank (`kind != "mrm" and type != "blank"`).
pub struct ItemMeta {
    pub slot: u32,
    pub label: String,
    pub time: Option<f64>,
    pub full: bool,
}

/// What the page asks: the ion `mz` against `parent`, from the file of item `k`.
pub struct Ask<'a> {
    pub k: i64,
    pub mz: f64,
    pub parent: f64,
    pub rt0: Option<f64>,
    pub rt1: Option<f64>,
    pub formula: Option<&'a str>,
}

/// The strongest 40 peaks, as Python sends them for the proofs of the page.
fn strongest(s: &(Vec<f64>, Vec<f64>)) -> Value {
    let mut k: Vec<usize> = (0..s.1.len()).collect();
    crate::sortc::slice_by(&mut k, |&a, &b| s.1[b].total_cmp(&s.1[a]));
    k.truncate(40);
    let a: Vec<f64> = k.iter().map(|&i| s.0[i]).collect();
    let y: Vec<f64> = k.iter().map(|&i| s.1[i]).collect();
    Value::Arr(vec![floats(&a), floats(&y)])
}

fn has_scans<R: ReadAt>(r: &Routes<R>, level: u8) -> bool {
    r.idx.scans.iter().any(|s| s.level == level)
}

/// The product-ion spectra of X and of P from the first file that has MS2 scans (whole run), `None` if there is no spectrum of P.
fn find_ms2<R: ReadAt>(items: &[ItemMeta], routes: &mut Mappa<u32, Routes<R>>, a: &Ask) -> Result<Option<Ms2>> {
    for it in items {
        let Some(r) = routes.get_mut(&it.slot) else { continue };
        if !has_scans(r, 2) {
            continue;
        }
        let rts = r.idx.scans.iter().map(|s| s.rt);
        let lo = rts.clone().fold(f64::INFINITY, f64::min);
        let hi = rts.fold(f64::NEG_INFINITY, f64::max);
        let mut spec = |mz: f64| -> Result<Option<(Vec<f64>, Vec<f64>)>> {
            Ok(r.binned(2, lo, hi, Some(mz), 0.1, None)?.filter(|b| b.n > 0).map(|b| (b.mz, b.y)))
        };
        let (sx, sp) = (spec(a.mz)?, spec(a.parent)?);
        if let Some(p) = sp {
            return Ok(Some(Ms2 { x: sx, p }));
        }
    }
    Ok(None)
}

/// The report of `App.origin` for the session `items` (whose indexes are in `routes`, by slot), or `None` when Python must answer.
pub fn origin_route<R: ReadAt>(items: &[ItemMeta], routes: &mut Mappa<u32, Routes<R>>, a: &Ask) -> Result<Option<Value>> {
    let mut full: Vec<(usize, &ItemMeta)> = Vec::new();
    let mut hr = false;
    for (i, it) in items.iter().enumerate() {
        let Some(r) = routes.get(&it.slot) else { return Ok(None) };
        if it.full && has_scans(r, 1) {
            hr |= group_is_hr(&r.idx.scans, true);
            full.push((i, it));
        }
    }
    if full.is_empty() {
        return Err(Error::new("err.origin.needFullScan"));
    }
    let mut slots: Vec<u32> = full.iter().map(|(_, it)| it.slot).collect();
    crate::sortc::slice_ord(&mut slots);
    if slots.windows(2).any(|w| w[0] == w[1]) {
        return Ok(None);
    }
    let ms2 = find_ms2(items, routes, a)?;
    let pol = full.iter().find_map(|(_, it)| routes.get(&it.slot)?.idx.scans.iter().find(|s| s.polarity != 0).map(|s| s.polarity as i64)).unwrap_or(1);
    let reference = full.iter().position(|(i, _)| *i as i64 == a.k);
    let mut by_slot: Mappa<u32, &mut Index<R>> = routes.iter_mut().map(|(k, r)| (*k, &mut r.idx)).collect();
    let mut samples: Vec<Sample<R>> = Vec::new();
    for (_, it) in &full {
        let idx = by_slot.remove(&it.slot).expect("slot checked above");
        let Some(si) = idx.section_of(1, None) else { return Ok(None) };
        samples.push(Sample { label: it.label.clone(), time: it.time, idx, si });
    }
    let req = Request { mz: a.mz, parent_mz: a.parent, rt0: a.rt0, rt1: a.rt1, reference, formula_p: a.formula, top: 25 };
    let rep = origin_report(&mut samples, ms2.as_ref(), &req)?;
    let Value::Obj(fields) = rep else { return Ok(None) };
    let mut out = Obj::new();
    for (k, v) in fields {
        out.set(&k, v);
    }
    out.set("hr", Value::Bool(hr));
    out.set("pol", Value::Num(pol as f64));
    if let Some(m) = &ms2 {
        let x = m.x.as_ref().map_or(Value::Null, strongest);
        out.set("ms2_peaks", Obj::new().put("x", x).put("p", strongest(&m.p)).done());
    }
    Ok(Some(out.done()))
}
