//! `/api/dda_window?k&r0&r1[&hr=0]`: the MS2 started from the survey scans of a time window, grouped by precursor within the tolerance of the
//! MS2 profile (ppm of a high-resolution profile, else 0.5 Da, as `tolOf` in dda.js). The most intense precursor first, members in order of
//! acquisition. Reference: `mzlab/dda_finestra.py`; parity test: `tests/hr_parity.rs`.

use crate::error::Result;
use crate::idxread::ReadAt;
use crate::iteminfo::hr_profile;
use crate::json::{round, Out};
use crate::routes::{Query, Routes};
use crate::sortc;

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
        Ok(Some(window_json(
            &self.idx.scans,
            a.min(b),
            a.max(b),
            q.get("hr") != Some("0"),
        )))
    }
}

/// The answer for the scans of a file (also the empty answer of a file without scans).
pub fn window_json(scans: &[crate::idxread::Scan], lo: f64, hi: f64, hr: bool) -> String {
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
    let top = |g: &Group| {
        g.sid
            .iter()
            .filter_map(|&i| scans[i].pint)
            .fold(-1.0f64, f64::max)
    };
    sortc::any_by(&mut groups, |x, y| {
        sortc::desc_nan_last(top(x), top(y)).then(sortc::asc_nan_last(x.prec, y.prec))
    });
    let mut o = Out::default();
    o.raw("{\"ms1\":")
        .int(ms1.iter().filter(|&&b| b).count() as i64)
        .raw(",\"n\":")
        .int(kids.len() as i64)
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
            o.raw(if k > 0 { "," } else { "" }).opt(scans[i].pint);
        }
        o.raw("]}");
    }
    o.raw("]}");
    o.0
}
