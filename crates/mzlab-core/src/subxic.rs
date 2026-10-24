//! `api/subxic` over the index of a high-resolution survey: the XIC that sums every sub-formula of an ion and the measures of each one
//! (`App.subxic` and `chem.subformulas.measure` of Python). A search tool: masses and what the file shows at them, no labels.
//! Profile or unit-resolution files, a ppm out of range and a formula without sub-formulas answer with their error (`err.subxic.*`), as in Python.

use crate::elements::fmt;
use crate::error::{Error, Result};
use crate::idxread::ReadAt;
use crate::iteminfo::group_is_hr;
use crate::json::{round, Value};
use crate::jsonb::{floats, opt, text, Obj};
use crate::routes::{Query, Routes};
use crate::stats::psum;
use crate::subformulas::subformulas;

const MAX_ROWS: usize = 300;

/// Correlation of two traces (`pearson`): None when one is flat. Same order of operations as `np.corrcoef`.
fn pearson(a: &[f64], b: &[f64]) -> Option<f64> {
    let n = a.len();
    if n < 3 {
        return None;
    }
    let mean = |v: &[f64]| psum(v) / n as f64;
    let centred = |v: &[f64]| -> Vec<f64> {
        let m = mean(v);
        v.iter().map(|x| x - m).collect()
    };
    let (da, db) = (centred(a), centred(b));
    let sq = |d: &[f64]| psum(&d.iter().map(|x| x * x).collect::<Vec<_>>());
    if sq(&da) / n as f64 == 0.0 || sq(&db) / n as f64 == 0.0 {
        return None;
    }
    let cov = psum(&da.iter().zip(&db).map(|(x, y)| x * y).collect::<Vec<_>>()) / (n - 1) as f64;
    let (sa, sb) = ((sq(&da) / (n - 1) as f64).sqrt(), (sq(&db) / (n - 1) as f64).sqrt());
    Some((cov / sa / sb).clamp(-1.0, 1.0))
}

/// The peaks of one window of the table (`a..b`), read from the index on their own: the whole table of a survey (tens of millions of peaks) is never held.
struct Win {
    mz: Vec<f64>,
    pos: Vec<u32>,
    inten: Vec<f64>,
}

fn read_win<R: ReadAt>(idx: &mut crate::idxread::Index<R>, si: usize, a: u64, b: u64) -> Result<Win> {
    let n = b.saturating_sub(a) as usize;
    let mut w = Win { mz: Vec::new(), pos: Vec::new(), inten: Vec::new() };
    w.mz.try_reserve_exact(n).map_err(|_| crate::error::Error::memory_budget())?;
    w.pos.try_reserve_exact(n).map_err(|_| crate::error::Error::memory_budget())?;
    w.inten.try_reserve_exact(n).map_err(|_| crate::error::Error::memory_budget())?;
    idx.each_peak(si, a, b, |p, mz, y| {
        w.pos.push(p as u32);
        w.inten.push(y);
        w.mz.push(mz);
    })?;
    Ok(w)
}

/// Summed intensity of the peaks of a window in every scan, added in table order (`np.bincount`).
fn trace(t: &Win, n: usize) -> Vec<f64> {
    let mut y = vec![0.0; n];
    for i in 0..t.pos.len() {
        if (t.pos[i] as usize) < n {
            y[t.pos[i] as usize] += t.inten[i];
        }
    }
    y
}

/// One row of the table: the sub-formula `name` at `mz`, from the peaks `w` of its window.
fn row(name: &str, mz: f64, t: &Win, rt: &[f64], full: &[f64], is_full: bool) -> (f64, Value) {
    let y = trace(t, rt.len());
    let apex = y.iter().enumerate().fold(0, |b, (i, v)| if *v > y[b] { i } else { b });
    let wsum = psum(&t.inten);
    let err = (wsum > 0.0).then(|| (psum(&(0..t.inten.len()).map(|i| t.mz[i] * t.inten[i]).collect::<Vec<_>>()) / wsum - mz) / mz * 1e6);
    let r = if is_full { None } else { pearson(&y, full) };
    let o = Obj::new().put("formula", text(name)).n("mz", round(mz, 5)).put("ppm", opt(err.map(|e| round(e, 2))));
    let o = o.n("int", round(y[apex], 1)).n("rt", round(rt[apex], 4)).put("r", opt(r.map(|r| round(r, 3))));
    (round(y[apex], 1), o.done())
}

/// Sub-formulas of `formula` measured in the survey of `r`, as the JSON of `/api/subxic`.
pub fn subxic<R: ReadAt>(r: &mut Routes<R>, k: i64, q: &Query) -> Result<Option<String>> {
    let (Some(formula), Some(ppm)) = (q.some("f"), q.get("ppm").map_or(Some(5.0), |s| s.trim().parse::<f64>().ok())) else { return Ok(None) };
    let (Some(min_mz), Some(z)) = (q.get("min").map_or(Some(100.0), |s| s.trim().parse::<f64>().ok()), q.get("z").map_or(Some(1), |s| s.trim().parse::<i32>().ok())) else { return Ok(None) };
    let survey = r.idx.scans.iter().filter(|s| s.level == 1);
    if survey.clone().any(|s| s.profile) || !group_is_hr(&r.idx.scans, true) {
        return Err(Error::new("err.subxic.lr"));
    }
    if !(0.0 < ppm && ppm <= 100.0) {
        return Err(Error::new("err.subxic.ppm"));
    }
    let (f, subs) = subformulas(formula, z, min_mz)?;
    if subs.is_empty() {
        return Err(Error::new("err.subxic.none").with("mz", min_mz));
    }
    let Some(si) = r.idx.section_of(1, None) else { return Err(Error::new("err.subxic.lr")) };
    let rt = r.idx.arrays(si)?.rt.clone();
    let mut spans: Vec<(usize, usize)> = Vec::with_capacity(subs.len());
    for (_, mz) in &subs {
        let (lo, hi) = (r.idx.search(si, mz - mz * ppm * 1e-6, false)?, r.idx.search(si, mz + mz * ppm * 1e-6, true)?);
        spans.push((lo as usize, hi as usize));
    }
    let whole = fmt(&f);
    let full_i = subs.iter().position(|(s, _)| *s == whole);
    let full = match full_i {
        Some(i) if spans[i].1 > spans[i].0 => trace(&read_win(&mut r.idx, si, spans[i].0 as u64, spans[i].1 as u64)?, rt.len()),
        _ => vec![0.0; rt.len()],
    };
    let mut rows = Vec::new();
    for (i, &(a, b)) in spans.iter().enumerate().filter(|(_, s)| s.1 > s.0) {
        let w = read_win(&mut r.idx, si, a as u64, b as u64)?;
        rows.push(row(&subs[i].0, subs[i].1, &w, &rt, &full, Some(i) == full_i));
    }
    let mut merged: Vec<(usize, usize)> = spans.iter().filter(|s| s.1 > s.0).cloned().collect();
    crate::sortc::slice_ord(&mut merged);
    let mut comb = vec![0.0; rt.len()];
    let mut done = 0;
    for (a, b) in merged {                                   // the union of the windows: a peak inside two of them counts once
        let from = a.max(done);
        if from < b {
            let w = read_win(&mut r.idx, si, from as u64, b as u64)?;
            for i in 0..w.pos.len() {
                comb[w.pos[i] as usize] += w.inten[i];
            }
        }
        done = done.max(b);
    }
    crate::sortc::slice_by(&mut rows, |x, y| y.0.total_cmp(&x.0));
    let r4 = |v: &[f64], nd: usize| floats(&v.iter().map(|&x| round(x, nd)).collect::<Vec<_>>());
    let n_hit = rows.len();
    let o = Obj::new().i("k", k).n("ppm", ppm).n("min_mz", min_mz).i("z", z as i64).put("rt", r4(&rt, 4)).put("y", r4(&comb, 1)).put("full", r4(&full, 1));
    let o = o.i("n_sub", subs.len() as i64).i("n_hit", n_hit as i64).put("rows", Value::Arr(rows.into_iter().take(MAX_ROWS).map(|r| r.1).collect()));
    Ok(Some(crate::jsonb::write(&o.done())))
}
