//! `/api/specpeak` (`spectrum get --peak auto`): the apex of the product-ion chromatogram of a level (and precursor), the window of `factor` x FWHM around it, the
//! two side windows next to the peak, and the window's summed spectrum minus the mean spectrum of the sides, per nominal mass. Every choice is in the answer:
//! the numbers behind the apex, the window, the sides, the background level and the rule of the verdict. Without an apex no spectrum is made (`no_apex`).
//! Read from the index only, in the lazy `rawxic` `.wasm`, so the site, the app and the CLI answer the same bytes.

use std::collections::BTreeMap;

use crate::error::{Error, Result};
use crate::idxread::ReadAt;
use crate::iteminfo::hr_profile;
use crate::json::{round, Out};
use crate::rawxic::{tallest, GAP_MIN, MIN_POINTS, MIN_SN, SIDE_MIN, SMOOTH_HALF, VALLEY_PCT};
use crate::routes::{Query, Routes};
use crate::specpeaks::{merge_unit, nominal};
use crate::specsum::fill_of;

/// The window is `FACTOR` x FWHM wide, centred on the apex (`--set factor=`).
pub const FACTOR: f64 = 2.0;
/// The bin of the nominal-mass spectrum (the one `/api/spectrum` uses).
const BIN: f64 = 0.1;
/// How the background was made, named in the answer.
const METHOD: &str = "mean spectrum of the scans of both side windows, scaled to the scans of the window, per nominal mass, lines above zero kept";

/// The thresholds and widths of a request (`--set`), with their defaults.
struct Set {
    factor: f64,
    side: f64,
    gap: f64,
    min_sn: f64,
    min_points: usize,
}

fn bad(name: &str, v: &str) -> Error {
    Error::new("err.spec.peak")
        .with("name", name.to_string())
        .with("value", v.to_string())
}

/// `factor`, `side_min`, `gap_min`, `min_sn`, `min_points` of a request; a value out of its range is refused.
fn set_of(q: &Query) -> Result<Set> {
    let get = |name: &str, d: f64, lo: f64, hi: f64, open: bool| -> Result<f64> {
        let v = q.num(name).map_err(|_| bad(name, q.get(name).unwrap_or("")))?;
        match v {
            None => Ok(d),
            Some(v) if v.is_finite() && v <= hi && (if open { v > lo } else { v >= lo }) => Ok(v),
            Some(_) => Err(bad(name, q.get(name).unwrap_or(""))),
        }
    };
    let n = get("min_points", MIN_POINTS as f64, 1.0, 1e6, false)?;
    if n.fract() != 0.0 {
        return Err(bad("min_points", q.get("min_points").unwrap_or("")));
    }
    Ok(Set {
        factor: get("factor", FACTOR, 0.0, 100.0, true)?,
        side: get("side_min", SIDE_MIN, 0.0, 1e4, true)?,
        gap: get("gap_min", GAP_MIN, 0.0, 1e4, false)?,
        min_sn: get("min_sn", MIN_SN, 0.0, 1e6, false)?,
        min_points: n as usize,
    })
}

/// The width of the peak at half of its height above the baseline, by linear interpolation on the raw trace out from the apex; `(left, right)` times and
/// whether the walk ran out of the trace before it came down (the end of the trace is then the edge).
fn half_width(rt: &[f64], y: &[f64], apex: usize, half: f64) -> ((f64, bool), (f64, bool)) {
    let cross = |dir: isize| {
        let mut i = apex as isize;
        loop {
            let j = i + dir;
            if j < 0 || j as usize >= y.len() {
                return (rt[i as usize], true);
            }
            let (a, b) = (i as usize, j as usize);
            if y[b] <= half {
                let t = (y[a] - half) / (y[a] - y[b]).max(1e-12);
                return (rt[a] + t * (rt[b] - rt[a]), false);
            }
            i = j;
        }
    };
    (cross(-1), cross(1))
}

/// A number the answer shows.
fn rd(v: f64) -> f64 {
    round(v, 4)
}

impl<R: ReadAt> Routes<R> {
    /// The chromatogram of the product-ion (or survey) scans of a level: time and summed intensity of each scan, for the precursor when the level has one.
    fn level_trace(&mut self, level: u8, precursor: Option<f64>) -> Result<(Vec<f64>, Vec<f64>)> {
        let Some(si) = self.idx.section_of(level, self.pol) else {
            return Ok((Vec::new(), Vec::new()));
        };
        let want = if level > 1 { precursor } else { None };
        let (ar, scans) = self.idx.arrays_scans(si)?;
        let (mut rt, mut y) = (Vec::new(), Vec::new());
        for j in 0..ar.rt.len() {
            let s = &scans[ar.scan_ids[j] as usize];
            if want.is_some_and(|p| (s.prec(-1.0) - p).abs() > 0.6) {
                continue;
            }
            rt.push(ar.rt[j]);
            y.push(ar.sumint[j]);
        }
        Ok((rt, y))
    }

    /// The sum (not the mean) of a window per nominal mass, and the scans it holds; the lines of one nominal mass are merged as `/api/spectrum` does.
    fn nominal_sum(&mut self, level: u8, (lo, hi): (f64, f64), precursor: Option<f64>) -> Result<(Vec<f64>, Vec<f64>, usize)> {
        let before = std::mem::replace(&mut self.msn.opts.sum, true);
        let b = self.binned(level, lo, hi, precursor, BIN, None);
        self.msn.opts.sum = before;
        Ok(match b? {
            Some(b) if !b.mz.is_empty() => {
                let (mz, y) = merge_unit(&b.mz, &b.y);
                (mz, y, b.n)
            }
            Some(b) => (Vec::new(), Vec::new(), b.n),
            None => (Vec::new(), Vec::new(), 0),
        })
    }

    /// `/api/specpeak?rt0=A&rt1=B[&level=N][&precursor=P][&factor=2&side_min=0.3&gap_min=0.05&min_sn=3&min_points=3][&fill_ms=T]`: `rt0`..`rt1` is where the
    /// apex is searched; `None` when the request has no such range.
    pub fn spec_peak_route(&mut self, q: &Query) -> Result<Option<String>> {
        let (Some(rt0), Some(rt1)) = (q.num("rt0")?, q.num("rt1")?) else {
            return Ok(None);
        };
        let (level, fill, precursor, p) = (q.level(), fill_of(q)?, q.num("precursor")?, set_of(q)?);
        let (lo, hi) = (rt0.min(rt1), rt0.max(rt1));
        let (rt, y) = self.level_trace(level, precursor)?;
        let mut o = Out::default();
        o.raw("{\"mode\":\"peak_auto\",\"ms_level\":").int(i64::from(level));
        o.raw(&format!(
            ",\"params\":{{\"factor\":{},\"side_min\":{},\"gap_min\":{},\"min_sn\":{},\"min_points\":{}}}",
            p.factor, p.side, p.gap, p.min_sn, p.min_points
        ));
        o.raw(&format!(",\"search_min\":[{lo},{hi}]"));
        let in_range = rt.iter().filter(|&&t| t >= lo && t <= hi).count();
        o.raw(&format!(",\"trace\":{{\"n_scans\":{},\"n_scans_searched\":{in_range},\"basis\":\"summed intensity of every scan of the level{}\"}}", rt.len(), if level > 1 { " and precursor" } else { "" }));
        let none = |o: &mut Out, verdict: &str, rule: &str, apex: &str| {
            o.raw(&format!(",\"verdict\":\"{verdict}\",\"flags\":[],\"apex\":{apex},\"fwhm_min\":null,\"window_min\":null,\"sides\":null,\"background\":null,"));
            o.raw("\"spectrum\":false,\"n_scans\":").int(in_range as i64);
            o.raw(",\"total_ions\":null,\"sum\":null,\"rule\":").str(rule).raw(",\"mz\":[],\"y\":[]}");
        };
        if in_range == 0 {
            none(&mut o, "no_data", &format!("no scan of MS level {level} between {lo} and {hi} min"), "null");
            return Ok(Some(o.0));
        }
        let (hit, _, _, edge) = tallest(&rt, &y, lo, hi, SMOOTH_HALF, VALLEY_PCT);
        let Some(hit) = hit else {
            none(&mut o, "no_apex", "no peak in the search range of the chromatogram -> no_apex", "null");
            return Ok(Some(o.0));
        };
        let (ok_sn, ok_pt) = (hit.sn >= p.min_sn, hit.points >= p.min_points);
        let apex_json = format!(
            "{{\"rt_min\":{},\"height\":{},\"sn\":{},\"points\":{},\"start_min\":{},\"end_min\":{}}}",
            hit.rt, hit.height, hit.sn, hit.points, hit.start_rt, hit.end_rt
        );
        if !(ok_sn && ok_pt) {
            let rule = format!(
                "candidate at {} min: S/N {} {} min_sn {}; points {} {} min_points {} -> no_apex",
                hit.rt,
                hit.sn,
                if ok_sn { ">=" } else { "<" },
                p.min_sn,
                hit.points,
                if ok_pt { ">=" } else { "<" },
                p.min_points
            );
            none(&mut o, "no_apex", &rule, &apex_json);
            return Ok(Some(o.0));
        }
        // the width at half height above the baseline the integrator drew under the apex
        let at = rt.iter().position(|&t| t == hit.rt).unwrap_or(0);
        let half = y[at] - hit.height / 2.0;
        let ((l, cut_l), (r, cut_r)) = half_width(&rt, &y, at, half);
        let fwhm = r - l;
        let mut flags: Vec<&str> = Vec::new();
        if edge {
            flags.push("edge");
        }
        if cut_l || cut_r {
            flags.push("fwhm_cut");
        }
        let h = p.factor * fwhm / 2.0;
        let (w0, w1) = (hit.rt - h, hit.rt + h);
        // the sides start outside the peak the integrator found as well as outside the window, so a tail never counts as background
        let (a0, a1) = (w0.min(hit.start_rt), w1.max(hit.end_rt));
        let left = (a0 - p.gap - p.side, a0 - p.gap);
        let right = (a1 + p.gap, a1 + p.gap + p.side);
        let side_of = |s: (f64, f64)| {
            let (mut n, mut tot) = (0usize, 0.0);
            for i in 0..rt.len() {
                if rt[i] >= s.0 && rt[i] <= s.1 {
                    n += 1;
                    tot += y[i];
                }
            }
            (n, tot)
        };
        let (side_l, side_r) = (side_of(left), side_of(right));
        let n_side = side_l.0 + side_r.0;
        let cps = (n_side > 0).then(|| (side_l.1 + side_r.1) / n_side as f64);
        if n_side == 0 {
            flags.push("no_background");
        } else if side_l.0 == 0 || side_r.0 == 0 {
            flags.push("one_side");
        }
        let background = match cps {
            Some(c) => format!("{{\"applied\":\"sides\",\"method\":{},\"n_side_scans\":{n_side},\"per_scan_cps\":{}}}", quote(METHOD), rd(c)),
            None => "{\"applied\":\"none\",\"method\":null,\"n_side_scans\":0,\"per_scan_cps\":null}".to_string(),
        };
        let rule = format!(
            "apex {} min: S/N {} >= min_sn {}; points {} >= min_points {} -> peak; FWHM {} min ({} .. {}); window {} x FWHM = {} .. {} min",
            hit.rt, hit.sn, p.min_sn, hit.points, p.min_points, rd(fwhm), rd(l), rd(r), p.factor, rd(w0), rd(w1)
        );
        // a profile or high-resolution level: the window and the sides are given, no spectrum (the nominal-mass subtraction does not describe its lines)
        let scans = &self.idx.scans;
        let is_profile = scans.iter().filter(|s| s.level == level).any(|s| s.profile);
        let hr = hr_profile(scans, level == 1, true).on;
        let has_spectrum = !(is_profile || hr);
        let verdict = if has_spectrum { "peak" } else { "window_only" };
        o.raw(&format!(",\"verdict\":\"{verdict}\",\"flags\":[{}]", flags.iter().map(|f| format!("\"{f}\"")).collect::<Vec<_>>().join(",")));
        o.raw(&format!(",\"apex\":{apex_json},\"fwhm_min\":{},\"half_height\":{},\"window_min\":[{},{}]", rd(fwhm), rd(half), rd(w0), rd(w1)));
        let side_json = |s: (f64, f64), v: (usize, f64)| format!("{{\"window_min\":[{},{}],\"n_scans\":{}}}", rd(s.0), rd(s.1), v.0);
        o.raw(&format!(",\"sides\":{{\"left\":{},\"right\":{},\"anchored_min\":[{},{}]}}", side_json(left, side_l), side_json(right, side_r), rd(a0), rd(a1)));
        o.raw(",\"background\":").raw(&background);
        let sum = self.sum_evidence(level, (w0, w1), precursor, fill, &background)?;
        let (mut mz, mut yy, mut n_win) = (Vec::new(), Vec::new(), 0usize);
        let verdict_rule = if !has_spectrum {
            format!("{rule}; the level is {}: no spectrum made, take the window with spectrum get --rt A,B --sum", if is_profile { "profile" } else { "high resolution" })
        } else {
            let (wm, wy, n) = self.nominal_sum(level, (w0, w1), precursor)?;
            n_win = n;
            let mut bg: BTreeMap<i64, f64> = BTreeMap::new();
            for s in [left, right] {
                let (sm, sy, _) = self.nominal_sum(level, s, precursor)?;
                for (m, v) in sm.iter().zip(&sy) {
                    *bg.entry(nominal(*m)).or_insert(0.0) += v;
                }
            }
            let scale = if n_side > 0 { n as f64 / n_side as f64 } else { 0.0 };
            for (m, v) in wm.iter().zip(&wy) {
                // rounded as it is shown: a mass whose net is a rounding error of the flat background is not a line
                let net = round(v - bg.get(&nominal(*m)).copied().unwrap_or(0.0) * scale, 1);
                if net > 0.0 {
                    mz.push(*m);
                    yy.push(net);
                }
            }
            rule
        };
        o.raw(",\"spectrum\":").raw(if has_spectrum { "true" } else { "false" });
        o.raw(",\"n_scans\":").int(if has_spectrum { n_win as i64 } else { scans_in(&rt, (w0, w1)) as i64 });
        // the ions the window rests on: counts x fill time, null until the fill time is stated or known (`evidence.sum.ions`)
        o.raw(",\"total_ions\":").raw(&ions_of(&sum));
        o.raw(",\"sum\":").raw(&sum);
        // the lines last: a reader that wants the evidence alone cuts the answer at `,"mz":[`
        o.raw(",\"rule\":").str(&verdict_rule).raw(",\"mz\":[");
        for (i, v) in mz.iter().enumerate() {
            o.raw(if i > 0 { "," } else { "" }).num(round(*v, 3));
        }
        o.raw("],\"y\":[");
        for (i, v) in yy.iter().enumerate() {
            o.raw(if i > 0 { "," } else { "" }).num(round(*v, 1));
        }
        o.raw("]}");
        Ok(Some(o.0))
    }
}

fn scans_in(rt: &[f64], (lo, hi): (f64, f64)) -> usize {
    rt.iter().filter(|&&t| t >= lo && t <= hi).count()
}

/// The `ions` member of a sum's evidence, as JSON (`null` when the fill time is unknown).
fn ions_of(sum: &str) -> String {
    crate::json::parse(sum.as_bytes())
        .ok()
        .and_then(|v| v.get("ions").and_then(crate::json::Value::f64))
        .map_or("null".to_string(), |v| format!("{v}"))
}

fn quote(s: &str) -> String {
    let mut o = Out::default();
    o.str(s);
    o.0
}
