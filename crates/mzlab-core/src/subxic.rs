//! `api/subxic` over the index of a high-resolution survey: the XIC that sums every sub-formula of an ion and the measures of each one
//! (`App.subxic` and `chem.subformulas.measure` of Python). A search tool: masses and what the file shows at them, no labels.
//! `Ok(None)` = Python's (profile or unit-resolution files, a request Python must refuse).

use crate::elements::fmt;
use crate::error::Result;
use crate::idxread::ReadAt;
use crate::iteminfo::group_is_hr;
use crate::json::{round, Value};
use crate::jsonb::{floats, opt, text, Obj};
use crate::peaktable::PeakTable;
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
    let (sa, sb) = (
        (sq(&da) / (n - 1) as f64).sqrt(),
        (sq(&db) / (n - 1) as f64).sqrt(),
    );
    Some((cov / sa / sb).clamp(-1.0, 1.0))
}

/// Summed intensity of the peaks `a..b` in every scan, added in table order (`np.bincount`).
fn trace(t: &PeakTable, a: usize, b: usize, n: usize) -> Vec<f64> {
    let mut y = vec![0.0; n];
    for i in a..b {
        if (t.pos[i] as usize) < n {
            y[t.pos[i] as usize] += t.inten[i];
        }
    }
    y
}

/// One row of the table: the sub-formula `name` at `mz`, from the peaks `w` of its window.
fn row(
    name: &str,
    mz: f64,
    (t, a, b): (&PeakTable, usize, usize),
    rt: &[f64],
    full: &[f64],
    is_full: bool,
) -> (f64, Value) {
    let y = trace(t, a, b, rt.len());
    let apex = y
        .iter()
        .enumerate()
        .fold(0, |b, (i, v)| if *v > y[b] { i } else { b });
    let wsum = psum(&t.inten[a..b]);
    let err = (wsum > 0.0).then(|| {
        (psum(&(a..b).map(|i| t.mz[i] * t.inten[i]).collect::<Vec<_>>()) / wsum - mz) / mz * 1e6
    });
    let r = if is_full { None } else { pearson(&y, full) };
    let o = Obj::new()
        .put("formula", text(name))
        .n("mz", round(mz, 5))
        .put("ppm", opt(err.map(|e| round(e, 2))));
    let o = o
        .n("int", round(y[apex], 1))
        .n("rt", round(rt[apex], 4))
        .put("r", opt(r.map(|r| round(r, 3))));
    (round(y[apex], 1), o.done())
}

/// Sub-formulas of `formula` measured in the survey of `r`, as the JSON of `/api/subxic`.
pub fn subxic<R: ReadAt>(r: &mut Routes<R>, k: i64, q: &Query) -> Result<Option<String>> {
    let (Some(formula), Some(ppm)) = (
        q.some("f"),
        q.get("ppm")
            .map_or(Some(5.0), |s| s.trim().parse::<f64>().ok()),
    ) else {
        return Ok(None);
    };
    let (Some(min_mz), Some(z)) = (
        q.get("min")
            .map_or(Some(100.0), |s| s.trim().parse::<f64>().ok()),
        q.get("z").map_or(Some(1), |s| s.trim().parse::<i32>().ok()),
    ) else {
        return Ok(None);
    };
    let survey = r.idx.scans.iter().filter(|s| s.level == 1);
    if !(0.0 < ppm && ppm <= 100.0)
        || survey.clone().any(|s| s.profile)
        || !group_is_hr(&r.idx.scans, true)
    {
        return Ok(None);
    }
    let (Some(si), Ok((f, subs))) = (r.idx.section_of(1, None), subformulas(formula, z, min_mz))
    else {
        return Ok(None);
    };
    if subs.is_empty() {
        return Ok(None);
    }
    let Some(t) = r.idx.load_table(si)? else {
        return Ok(None);
    };
    let rt = r.idx.arrays(si)?.rt.clone();
    let spans: Vec<(usize, usize)> = subs
        .iter()
        .map(|(_, mz)| t.window(mz - mz * ppm * 1e-6, mz + mz * ppm * 1e-6))
        .collect();
    let whole = fmt(&f);
    let full_i = subs.iter().position(|(s, _)| *s == whole);
    let full = match full_i {
        Some(i) if spans[i].1 > spans[i].0 => trace(&t, spans[i].0, spans[i].1, rt.len()),
        _ => vec![0.0; rt.len()],
    };
    let mut rows = Vec::new();
    for (i, &(a, b)) in spans.iter().enumerate().filter(|(_, s)| s.1 > s.0) {
        rows.push(row(
            &subs[i].0,
            subs[i].1,
            (&t, a, b),
            &rt,
            &full,
            Some(i) == full_i,
        ));
    }
    let mut merged: Vec<(usize, usize)> = spans.iter().filter(|s| s.1 > s.0).cloned().collect();
    merged.sort();
    let mut comb = vec![0.0; rt.len()];
    let mut done = 0;
    for (a, b) in merged {
        // the union of the windows: a peak inside two of them counts once
        for i in a.max(done)..b {
            comb[t.pos[i] as usize] += t.inten[i];
        }
        done = done.max(b);
    }
    rows.sort_by(|x, y| y.0.total_cmp(&x.0));
    let r4 = |v: &[f64], nd: usize| floats(&v.iter().map(|&x| round(x, nd)).collect::<Vec<_>>());
    let n_hit = rows.len();
    let o = Obj::new()
        .i("k", k)
        .n("ppm", ppm)
        .n("min_mz", min_mz)
        .i("z", z as i64)
        .put("rt", r4(&rt, 4))
        .put("y", r4(&comb, 1))
        .put("full", r4(&full, 1));
    let o = o
        .i("n_sub", subs.len() as i64)
        .i("n_hit", n_hit as i64)
        .put(
            "rows",
            Value::Arr(rows.into_iter().take(MAX_ROWS).map(|r| r.1).collect()),
        );
    Ok(Some(crate::jsonb::write(&o.done())))
}
