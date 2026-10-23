//! "Where does this ion come from?": a port of `ionfamily.origin_report`. The evidence for the ion X against a candidate precursor P over
//! the full-scan files of a session, read from the `.mzidx` of each file. Nothing is concluded: every number is a measurement with its
//! own limits, and `warnings` lists them as messages `{key, params}`.

use crate::error::Result;
use crate::idxread::{Index, ReadAt};
use crate::json::Value;
use crate::jsonb::{floats, int, msg, num, opt, text, Obj};
use crate::kinetics::kinetics;
use crate::ms2sim::{ms2_membership, ms2_similarity};
use crate::peaks::MIN_SCANS_RELIABLE;
use crate::ratio::{ratio_constancy, ratio_fit};
use crate::rois::{build_rois, extract_trace, Window};
use crate::roles::{isotope_check, Losses};
use crate::similarity::profile_similarity;
use crate::stats::{psum, wpearson};
use crate::tracestat::{argmin, detect, prep};

/// A full-scan file of the session: its label, treatment time (minutes) and the section of its `.mzidx` with the survey table.
pub struct Sample<'a, R: ReadAt> {
    pub label: String,
    pub time: Option<f64>,
    pub idx: &'a mut Index<R>,
    pub si: usize,
}

/// Product-ion spectra `(m/z, intensity)` of X (if acquired) and of P.
pub struct Ms2 {
    pub x: Option<(Vec<f64>, Vec<f64>)>,
    pub p: (Vec<f64>, Vec<f64>),
}

pub struct Request<'a> {
    pub mz: f64,
    pub parent_mz: f64,
    pub rt0: Option<f64>,
    pub rt1: Option<f64>,
    pub reference: Option<usize>,
    pub formula_p: Option<&'a str>,
    /// Co-eluting ions to rank (the page asks for 25).
    pub top: usize,
}

const TOL_MZ: f64 = 0.35;
const MIN_REL_AREA: f64 = 0.005;

fn lower(a: &[f64], v: f64) -> usize {
    a.partition_point(|x| *x < v)
}
fn upper(a: &[f64], v: f64) -> usize {
    a.partition_point(|x| *x <= v)
}
fn sum_range(v: &[f64], a: usize, b: usize) -> f64 {
    let (a, b) = (a.min(v.len()), b.min(v.len()));
    if a < b { psum(&v[a..b]) } else { 0.0 }
}
fn vmax(v: &[f64]) -> f64 {
    v.iter().cloned().fold(f64::MIN, f64::max)
}

/// `ionfamily._area_in`: trapezoid area of the baseline-corrected trace between two times.
fn area_in(rt: &[f64], y: &[f64], t0: f64, t1: f64) -> f64 {
    let (yc, _, _) = prep(y);
    let idx: Vec<usize> = (0..rt.len()).filter(|&i| rt[i] >= t0 && rt[i] <= t1).collect();
    if idx.len() < 2 {
        return 0.0;
    }
    psum(&idx.windows(2).map(|w| (yc[w[1]] + yc[w[0]]) / 2.0 * (rt[w[1]] - rt[w[0]])).collect::<Vec<_>>())
}

struct Ref {
    rt: Vec<f64>,
    apex_rt: f64,
    fwhm_min: f64,
    lo: usize,
    hi: usize,
    n_scans: usize,
    idx: usize,
}

/// The reference file (where P is strongest unless chosen) and the highest peak of P in it; `None` if P has no peak there.
fn find_reference<R: ReadAt>(samples: &mut [Sample<R>], r: &Request) -> Result<Option<Ref>> {
    let ix = match r.reference {
        Some(k) => k,
        None => {
            let mut tops = Vec::new();
            for s in samples.iter_mut() {
                let (rt, y) = extract_trace(s.idx, s.si, r.parent_mz, 0.5, None)?;
                tops.push(if rt.is_empty() { 0.0 } else { vmax(&y) });
            }
            tops.iter().enumerate().fold(0, |b, (i, v)| if *v > tops[b] { i } else { b })
        }
    };
    let s = &mut samples[ix];
    let (rt, p) = extract_trace(s.idx, s.si, r.parent_mz, 0.5, None)?;
    Ok(detect(&rt, &p).first().map(|pk| Ref { apex_rt: pk.e.apex_rt, fwhm_min: pk.e.fwhm_min, lo: pk.p.lo, hi: pk.p.hi, n_scans: pk.p.n_scans, idx: ix, rt }))
}

/// Co-eluting ROIs ranked by profile correlation with P (the student chooses; nothing is picked for them).
fn candidates(w: &Window, a: usize, b: usize, pw: &[f64], apex: f64, r: &Request, losses: &Losses) -> Result<Vec<Value>> {
    if w.mz.is_empty() {
        return Ok(Vec::new());
    }
    let n = w.n_scans;
    let area: Vec<f64> = (0..w.n_roi()).map(|j| sum_range(&w.x[j * n..(j + 1) * n], a, b)).collect();
    let mut order: Vec<usize> = (0..area.len()).collect();
    crate::sortc::slice_by(&mut order, |&i, &j| area[j].total_cmp(&area[i]));
    let pmax = vmax(&area);
    let mut out: Vec<(f64, Value)> = Vec::new();
    for &j in order.iter().take(r.top) {
        if area[j] < MIN_REL_AREA * pmax {
            continue;
        }
        let s2 = profile_similarity(&w.rt, &w.x[j * n..(j + 1) * n], pw, Some(apex), 0, 100, 0).done();
        if s2.get("n_scans").and_then(|v| v.f64()).unwrap_or(0.0) == 0.0 {
            continue;
        }
        let mut o = Obj::new().n("mz", w.mz[j]).n("area_rel", if pmax > 0.0 { area[j] / pmax } else { 0.0 });
        for k in ["pearson", "spearman", "deriv_corr", "cosine", "apex_diff_s", "apex_diff_sigma_s", "fwhm_ratio", "p_shift", "lag_best", "reliable", "x_has_peak"] {
            o = o.put(k, s2.get(k).cloned().unwrap_or(Value::Null));
        }
        o = o.put("roles", losses.annotate(w.mz[j], r.parent_mz, r.formula_p)?);
        let key = s2.get("pearson").and_then(|v| v.f64()).filter(|v| v.is_finite()).unwrap_or(-2.0);
        out.push((-key, o.done()));
    }
    crate::sortc::slice_by(&mut out, |x, y| x.0.total_cmp(&y.0));
    Ok(out.into_iter().map(|c| c.1).collect())
}

/// The lighter neighbours 1 and 2 Da below X (is X the M+1 / M+2 of another ion?): area ratio X/neighbour and profile correlation.
fn neighbors(w: &Window, xw: &[f64], mz: f64, a: usize, b: usize) -> Vec<Value> {
    let mut out = Vec::new();
    for kk in 1..=2 {
        if w.mz.is_empty() {
            continue;
        }
        let target = mz - kk as f64 * 1.003355;
        let jn = argmin(&w.mz.iter().map(|v| (v - target).abs()).collect::<Vec<_>>());
        if (w.mz[jn] - target).abs() > 0.5 {
            continue;
        }
        let n = w.n_scans;
        let row = &w.x[jn * n..(jn + 1) * n];
        let (an, ax) = (sum_range(row, a, b), sum_range(xw, a, b));
        let (x_s, n_s) = (&xw[a.min(xw.len())..b.min(xw.len()).max(a.min(xw.len()))], &row[a.min(n)..b.min(n).max(a.min(n))]);
        let rn = wpearson(x_s, n_s, None);
        out.push(Obj::new().i("delta", kk).n("mz", w.mz[jn]).n("area_ratio", if an > 0.0 { ax / an } else { f64::NAN }).n("pearson", rn).done());
    }
    out
}

struct Rows {
    rows: Vec<Value>,
    ratios: Vec<f64>,
    ses: Vec<f64>,
    labels: Vec<String>,
    timed: Vec<(f64, f64, f64)>,
}

/// The same measures in every sample: areas of P and X in the peak of P, the ratio F/P and where P's apex is.
fn across_samples<R: ReadAt>(samples: &mut [Sample<R>], r: &Request, win: (f64, f64), peak: (f64, f64), ref_apex: f64) -> Result<Rows> {
    let mut o = Rows { rows: vec![], ratios: vec![], ses: vec![], labels: vec![], timed: vec![] };
    for (k, s) in samples.iter_mut().enumerate() {
        let (rt, p) = extract_trace(s.idx, s.si, r.parent_mz, TOL_MZ, Some(win))?;
        let (_, x) = extract_trace(s.idx, s.si, r.mz, TOL_MZ, Some(win))?;
        if rt.len() < 7 {
            continue;
        }
        let (ap, ax) = (area_in(&rt, &p, peak.0, peak.1), area_in(&rt, &x, peak.0, peak.1));
        let mut row = Obj::new().i("k", k as i64).put("label", text(&s.label)).put("time", opt(s.time)).n("area_parent", ap).n("area_ion", ax);
        let (a0, a1) = (lower(&rt, peak.0), upper(&rt, peak.1));
        if ap > 0.0 && a1 as i64 - 1 - a0 as i64 >= 4 {
            let rf = ratio_fit(&x, &p, (a0, a1), 60, 0);
            if rf.get("ok").and_then(|v| v.bool()) == Some(true) {
                let (rr, se) = (rf.get("r_tls").and_then(|v| v.f64()).unwrap_or(f64::NAN), rf.get("r_se").and_then(|v| v.f64()).unwrap_or(f64::NAN));
                row = row.n("ratio", rr).n("ratio_se", se);
                o.ratios.push(rr);
                o.ses.push(se);
                o.labels.push(s.label.clone());
            }
        }
        let near = detect(&rt, &p).into_iter().min_by(|a, b| (a.e.apex_rt - ref_apex).abs().total_cmp(&(b.e.apex_rt - ref_apex).abs()));
        row = row.put("parent_apex_rt", near.filter(|k| (k.e.apex_rt - ref_apex).abs() < 0.3).map_or(Value::Null, |k| num(k.e.apex_rt)));
        if let Some(t) = s.time {
            o.timed.push((t, ap, ax));
        }
        o.rows.push(row.done());
    }
    Ok(o)
}

fn flat_top_warning(pw: &[f64], warnings: &mut Vec<Value>) -> i64 {
    let top = if pw.is_empty() { 0.0 } else { vmax(pw) };
    let flat = pw.iter().filter(|&&v| v >= 0.97 * top).count() as i64;
    if flat >= 3 && top > 0.0 {
        warnings.push(msg("of.warn.flatTop", &[("n", int(flat))]));
    }
    flat
}

fn early(rep: Obj, warn: Value) -> Result<Value> {
    Ok(rep.put("warnings", Value::Arr(vec![warn])).done())
}

/// The warning that a neighbouring ion (0.4-1.0 away, more than 5 % of the area of the parent) may coincide in mass with X.
fn coincidence_warning(cands: &[Value], mz: f64) -> Option<Value> {
    let near: Vec<f64> = cands.iter().filter_map(|c| {
        let m = c.get("mz")?.f64()?;
        let ar = c.get("area_rel")?.f64()?;
        ((m - mz).abs() > 0.4 && (m - mz).abs() <= 1.0 && ar > 0.05).then_some(m)
    }).take(3).collect();
    if near.is_empty() {
        return None;
    }
    let list = near.iter().map(|m| format!("{m:.1}")).collect::<Vec<_>>().join(", ");
    Some(msg("of.warn.massCoincidence", &[("mz", text(&list))]))
}

/// The evidence for the ion `r.mz` against `r.parent_mz` over `samples`, with the product-ion spectra `ms2` if the student loaded them.
pub fn origin_report<R: ReadAt>(samples: &mut [Sample<R>], ms2: Option<&Ms2>, r: &Request) -> Result<Value> {
    let base = Obj::new().n("mz", r.mz).n("parent_mz", r.parent_mz).put("samples", Value::Arr(vec![])).put("candidates", Value::Arr(vec![])).put("timing", Obj::new().done());
    if samples.is_empty() {
        return early(base, msg("of.warn.noFull", &[]));
    }
    let Some(rf) = find_reference(samples, r)? else {
        return early(base, msg("of.warn.noParentRef", &[]));
    };
    let half = (4.0 * rf.fwhm_min).max(0.4);
    let (rt0, rt1) = match (r.rt0, r.rt1) {
        (Some(a), Some(b)) => (a, b),
        _ => (rf.apex_rt - half, rf.apex_rt + half),
    };
    let (peak_t0, peak_t1) = (rf.rt[rf.lo], rf.rt[rf.hi]);
    let mut rep = base
        .i("ref", rf.idx as i64)
        .put("ref_label", text(&samples[rf.idx].label))
        .put("window", floats(&[rt0, rt1]))
        .n("parent_apex_rt", rf.apex_rt)
        .n("parent_fwhm_s", rf.fwhm_min * 60.0);
    let mut warnings: Vec<Value> = Vec::new();
    let s = &mut samples[rf.idx];
    let w = build_rois(s.idx, s.si, rt0, rt1, TOL_MZ, 4, 2, None, 0.0)?;
    rep = rep.i("n_rois", w.n_roi() as i64);
    if rf.n_scans < MIN_SCANS_RELIABLE {
        warnings.push(msg("of.warn.parentFewScans", &[("n", int(rf.n_scans as i64))]));
    }
    let (rtw, pw) = extract_trace(s.idx, s.si, r.parent_mz, TOL_MZ, Some((rt0, rt1)))?;
    rep = rep.i("parent_flat_top_scans", flat_top_warning(&pw, &mut warnings));
    let (_, xw) = extract_trace(s.idx, s.si, r.mz, TOL_MZ, Some((rt0, rt1)))?;
    let sim = profile_similarity(&rtw, &xw, &pw, Some(rf.apex_rt), 200, 400, 0).done();
    for wv in sim.get("warnings").map_or(&[][..], |v| v.arr()) {
        warnings.push(wv.clone());
    }
    let (a, b) = (lower(&rtw, peak_t0), upper(&rtw, peak_t1));
    let ratio = ratio_fit(&xw, &pw, (a, b), 200, 0);
    let losses = Losses::new();
    let cands = candidates(&w, a, b, &pw, rf.apex_rt, r, &losses)?;
    rep = rep.put("roles", losses.annotate(r.mz, r.parent_mz, r.formula_p)?).put("isotopes", isotope_check(&w, r.mz, a, b)).put("neighbors", Value::Arr(neighbors(&w, &xw, r.mz, a, b)));
    warnings.extend(coincidence_warning(&cands, r.mz));
    let nr = ratio.get("n").and_then(|v| v.f64()).unwrap_or(f64::INFINITY);
    if ratio.get("ok").and_then(|v| v.bool()) == Some(true) && nr < 6.0 {
        warnings.push(msg("of.warn.fewRatio", &[("n", int(nr as i64))]));
    }
    if sim.get("x_snr").map_or(1.0, |v| v.f64().unwrap_or(f64::NAN)) < 5.0 {
        warnings.push(msg("of.warn.weakIon", &[]));
    }
    let per = across_samples(samples, r, (rt0, rt1), (peak_t0, peak_t1), rf.apex_rt)?;
    if per.ratios.len() >= 2 {
        rep = rep.put("ratio_constancy", ratio_constancy(&per.ratios, &per.ses, &per.labels));
    }
    if per.timed.len() >= 3 {
        let (t, p, x): (Vec<f64>, Vec<f64>, Vec<f64>) = per.timed.iter().fold((vec![], vec![], vec![]), |mut acc, (a, b, c)| {
            acc.0.push(*a);
            acc.1.push(*b);
            acc.2.push(*c);
            acc
        });
        let kin = kinetics(&t, &p, &x);
        warnings.extend(kin.get("warnings").map_or(vec![], |v| v.arr().to_vec()));
        rep = rep.put("kinetics", kin);
    } else {
        warnings.push(msg("of.warn.fewTimed", &[]));
    }
    if let Some(m) = ms2 {
        let pv = (&m.p.0[..], &m.p.1[..]);
        rep = rep.put("ms2", match &m.x {
            Some(x) => ms2_similarity(r.mz, (&x.0[..], &x.1[..]), r.parent_mz, pv),
            None => ms2_membership(r.mz, r.parent_mz, pv),
        });
    }
    Ok(rep.put("profile", sim).put("ratio", ratio).put("candidates", Value::Arr(cands)).put("samples", Value::Arr(per.rows)).put("warnings", Value::Arr(warnings)).put("note", msg("of.note", &[])).done())
}
