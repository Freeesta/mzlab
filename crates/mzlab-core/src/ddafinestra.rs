//! `/api/dda_window?k&r0&r1[&hr=0]`: the MS2 started from the survey scans of a time window, grouped by precursor within the tolerance of the
//! MS2 profile (ppm of a high-resolution profile, else 0.5 Da, as `tolOf` in dda.js). The most intense precursor first, members in order of
//! acquisition. Reference: `tools/dda_finestra_riferimento.py`; parity test: `tests/hr_parity.rs`. A reader that gives no precursor intensity (Thermo
//! .raw) has it filled from the source (`dda_window_src`, every surface): the highest point of the parent survey scan in its isolation window.
//! `mz[&tol]` keeps only the MS2 of that precursor (tol in Da, else the profile's): the whole-file picks of dda.js read only their parents.

use crate::error::Result;
use crate::idxread::ReadAt;
use crate::iteminfo::hr_profile;
use crate::json::{round, Out};
use crate::routes::{Query, Routes};
use crate::sorgente::Scansioni;
use crate::sortc;

/// Parent survey scans read at most for one window: past them the missing intensities stay null (a wide average stays fast).
const MAX_PARENTS: usize = 400;

/// One precursor of the window: its m/z (the lowest of the group) and the MS2 scans (index in the file).
struct Group {
    prec: f64,
    sid: Vec<usize>,
}

impl<R: ReadAt> Routes<R> {
    pub(crate) fn dda_window(&self, q: &Query) -> Result<Option<String>> {
        let (Some(a), Some(b)) = (q.num("r0")?, q.num("r1")?) else {
            return Ok(None);
        };
        Ok(Some(
            window_with(
                &self.idx.scans,
                a.min(b),
                a.max(b),
                q.get("hr") != Some("0"),
                near(q)?,
                None,
            )
            .unwrap_or_default(),
        ))
    }

    /// The same answer, the missing precursor intensities filled from the parent survey scans of `src`.
    pub(crate) fn dda_window_src<M: Scansioni + ?Sized>(
        &self,
        q: &Query,
        src: &mut M,
    ) -> Result<Option<String>> {
        let (Some(a), Some(b)) = (q.num("r0")?, q.num("r1")?) else {
            return Ok(None);
        };
        let scans = &self.idx.scans;
        let mut fill = |p: usize, w: &[(f64, f64)]| -> Result<Vec<Option<f64>>> {
            let (mz, y) = src.spettro(&scans[p])?;
            Ok(w.iter()
                .map(|&(m, t)| apex(&mz, &y, m - t, m + t))
                .collect())
        };
        window_with(
            scans,
            a.min(b),
            a.max(b),
            q.get("hr") != Some("0"),
            near(q)?,
            Some(&mut fill),
        )
        .map(Some)
    }
}

/// The precursor `mz` asked for and its tolerance `tol` (Da), when given.
fn near(q: &Query) -> Result<Option<(f64, Option<f64>)>> {
    let tol = q.num("tol")?.filter(|t| *t > 0.0);
    Ok(q.num("mz")?.map(|m| (m, tol)))
}

/// Half-width of the search around a precursor: its isolation window, at most 0.5 Da (a filter gives the m/z to 2 decimals, a ppm tolerance would
/// miss its peak).
fn reach(s: &crate::idxread::Scan) -> f64 {
    s.iso.map_or(0.5, |(a, b)| ((b - a).abs() / 2.0).min(0.5))
}

/// The highest intensity of the points within [lo, hi], `None` without a positive one (a reader may not sort its points).
fn apex(mz: &[f64], y: &[f64], lo: f64, hi: f64) -> Option<f64> {
    let top = mz
        .iter()
        .zip(y)
        .filter(|(m, _)| lo <= **m && **m <= hi)
        .map(|(_, v)| *v)
        .fold(0.0f64, f64::max);
    (top > 0.0).then_some(top)
}

type Fill<'a> = dyn FnMut(usize, &[(f64, f64)]) -> Result<Vec<Option<f64>>> + 'a;

/// The answer for the scans of a file (also the empty answer of a file without scans).
pub fn window_json(scans: &[crate::idxread::Scan], lo: f64, hi: f64, hr: bool) -> String {
    window_with(scans, lo, hi, hr, None, None).unwrap_or_default()
}

/// `fill(parent, [(m/z, tolerance)])` gives the intensities of the MS2 whose scan has none, one parent at a time (one spectrum held at once);
/// without a source they stay as the index has them. `near = (m/z, tol)` keeps only the MS2 of that precursor.
fn window_with(
    scans: &[crate::idxread::Scan],
    lo: f64,
    hi: f64,
    hr: bool,
    near: Option<(f64, Option<f64>)>,
    fill: Option<&mut Fill>,
) -> Result<String> {
    let p = hr_profile(scans, false, hr);
    let d = p.dec + 1;
    let ppm = if p.on { p.ppm } else { 0.0 };
    let tol = |m: f64| if ppm > 0.0 { m * ppm * 1e-6 } else { 0.5 };
    let ms1: Vec<bool> = scans
        .iter()
        .map(|s| s.level == 1 && lo <= s.rt && s.rt <= hi)
        .collect();
    let in_win =
        |p: Option<i64>| p.is_some_and(|p| p >= 0 && (p as usize) < scans.len() && ms1[p as usize]);
    let key = |i: usize| {
        let s = &scans[i];
        s.precursor
            .filter(|p| *p != 0.0)
            .or(s.iso.map(|(a, b)| (a + b) / 2.0))
            .map(|v| round(v, d))
    };
    let mut kids: Vec<(f64, usize)> = (0..scans.len())
        .filter(|&i| scans[i].level > 1 && in_win(scans[i].parent))
        .filter_map(|i| key(i).map(|m| (m, i)))
        .filter(|&(m, _)| near.is_none_or(|(c, t)| (m - c).abs() <= t.unwrap_or_else(|| tol(c))))
        .collect();
    sortc::by(&mut kids, |x, y| {
        sortc::asc_nan_last(x.0, y.0).then(x.1.cmp(&y.1))
    });
    let mut groups: Vec<Group> = Vec::new();
    for &(m, i) in &kids {
        match groups.last_mut() {
            Some(g) if m - g.prec <= tol(g.prec) => g.sid.push(i),
            _ => groups.push(Group {
                prec: m,
                sid: vec![i],
            }),
        }
    }
    for g in &mut groups {
        sortc::ord(&mut g.sid);
    }
    let (pint, cut) = match fill {
        Some(f) => filled(scans, &kids, f)?,
        None => (scans.iter().map(|s| s.pint).collect(), false),
    };
    let top = |g: &Group| {
        g.sid
            .iter()
            .filter_map(|&i| pint[i])
            .fold(-1.0f64, f64::max)
    };
    sortc::any_by(&mut groups, |x, y| {
        sortc::desc_nan_last(top(x), top(y)).then(sortc::asc_nan_last(x.prec, y.prec))
    });
    Ok(answer(
        scans,
        ms1.iter().filter(|&&b| b).count(),
        kids.len(),
        &groups,
        &pint,
        cut,
    ))
}

/// The intensity of every scan: its own, else from `fill` for the MS2 `kids` (one parent at a time, the first `MAX_PARENTS` parents);
/// `true` when parents past the limit were left unread.
fn filled(
    scans: &[crate::idxread::Scan],
    kids: &[(f64, usize)],
    fill: &mut Fill,
) -> Result<(Vec<Option<f64>>, bool)> {
    let mut pint: Vec<Option<f64>> = scans.iter().map(|s| s.pint).collect();
    let mut need: Vec<(usize, usize, f64)> = kids
        .iter()
        .filter(|&&(_, i)| scans[i].pint.is_none())
        .map(|&(m, i)| (scans[i].parent.unwrap_or(0) as usize, i, m))
        .collect();
    sortc::by(&mut need, |x, y| x.0.cmp(&y.0).then(x.1.cmp(&y.1)));
    for (n, run) in need.chunk_by(|x, y| x.0 == y.0).enumerate() {
        if n == MAX_PARENTS {
            return Ok((pint, true));
        }
        let w: Vec<(f64, f64)> = run.iter().map(|&(_, i, m)| (m, reach(&scans[i]))).collect();
        for (&(_, i, _), v) in run.iter().zip(fill(run[0].0, &w)?) {
            pint[i] = v;
        }
    }
    Ok((pint, false))
}

fn answer(
    scans: &[crate::idxread::Scan],
    ms1: usize,
    n: usize,
    groups: &[Group],
    pint: &[Option<f64>],
    cut: bool,
) -> String {
    let mut o = Out::default();
    o.raw("{\"ms1\":")
        .int(ms1 as i64)
        .raw(",\"n\":")
        .int(n as i64)
        .raw(",\"groups\":[");
    for (n, g) in groups.iter().enumerate() {
        o.raw(if n > 0 { ",{\"prec\":" } else { "{\"prec\":" })
            .num(g.prec)
            .raw(",\"sid\":[");
        for (k, &i) in g.sid.iter().enumerate() {
            o.raw(if k > 0 { "," } else { "" }).int(i as i64);
        }
        o.raw("],\"rt\":[");
        for (k, &i) in g.sid.iter().enumerate() {
            o.raw(if k > 0 { "," } else { "" })
                .num(round(scans[i].rt, 4));
        }
        o.raw("],\"pint\":[");
        for (k, &i) in g.sid.iter().enumerate() {
            o.raw(if k > 0 { "," } else { "" }).opt(pint[i]);
        }
        o.raw("]}");
    }
    o.raw("]");
    if cut {
        o.raw(",\"pintCut\":").int(MAX_PARENTS as i64); // the intensities past these parents stay null: the page says so
    }
    o.raw("}");
    o.0
}
