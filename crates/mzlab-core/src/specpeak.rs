//! `/api/specpeak` (`spectrum get --peak auto`): the apex of the product-ion chromatogram of a level (and precursor), the window of `factor` x FWHM around it, the
//! two side windows next to the peak, and the window's summed spectrum minus the mean spectrum of the sides, per nominal mass. Every choice is in the answer:
//! the numbers behind the apex, the window, the sides, the background level and the rule of the verdict. Without an apex no spectrum is made (`no_apex`).
//! Read from the index only, in the lazy `specpeak` `.wasm`, so the site, the app and the CLI answer the same bytes.

use crate::error::{Error, Result};
use crate::idxread::ReadAt;
use crate::iteminfo::hr_profile;
use crate::json::{round, Out};
use crate::rawxic::{tallest, GAP_MIN, MIN_POINTS, MIN_SN, SIDE_MIN, SMOOTH_HALF, VALLEY_PCT};
use crate::routes::{Query, Routes};
use crate::specpeaks::nominal;
use crate::specsum::fill_of;

#[path = "specpeak_out.rs"]
mod out;
use out::{background_json, emit, head, without, Found};

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

/// One number of the request in `(lo, hi]` (`open`) or `[lo, hi]`, or `d` when it is absent.
#[inline(never)]
fn param(q: &Query, name: &str, d: f64, (lo, hi): (f64, f64), open: bool) -> Result<f64> {
    let v = q
        .num(name)
        .map_err(|_| bad(name, q.get(name).unwrap_or("")))?;
    match v {
        None => Ok(d),
        Some(v) if v.is_finite() && v <= hi && (if open { v > lo } else { v >= lo }) => Ok(v),
        Some(_) => Err(bad(name, q.get(name).unwrap_or(""))),
    }
}

/// `factor`, `side_min`, `gap_min`, `min_sn`, `min_points` of a request; a value out of its range is refused.
fn set_of(q: &Query) -> Result<Set> {
    let n = param(q, "min_points", MIN_POINTS as f64, (1.0, 1e6), false)?;
    if n.fract() != 0.0 {
        return Err(bad("min_points", q.get("min_points").unwrap_or("")));
    }
    Ok(Set {
        factor: param(q, "factor", FACTOR, (0.0, 100.0), true)?,
        side: param(q, "side_min", SIDE_MIN, (0.0, 1e4), true)?,
        gap: param(q, "gap_min", GAP_MIN, (0.0, 1e4), false)?,
        min_sn: param(q, "min_sn", MIN_SN, (0.0, 1e6), false)?,
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
    /// It reads the scans of the level and the precursor only (no scan filter), so the module does not carry the filter code of `/api/spectrum`.
    fn nominal_sum(
        &mut self,
        level: u8,
        (lo, hi): (f64, f64),
        precursor: Option<f64>,
    ) -> Result<(Vec<f64>, Vec<f64>, usize)> {
        let Some(si) = self.idx.section_of(level, self.pol) else {
            return Ok((Vec::new(), Vec::new(), 0));
        };
        let want = if level > 1 { precursor } else { None };
        let (ar, scans) = self.idx.arrays_scans(si)?;
        let mut ok = vec![false; ar.rt.len()];
        let mut n = 0;
        for (j, hit) in ok.iter_mut().enumerate() {
            let s = &scans[ar.scan_ids[j] as usize];
            if ar.rt[j] >= lo
                && ar.rt[j] <= hi
                && want.is_none_or(|p| (s.prec(-1.0) - p).abs() <= 0.6)
            {
                *hit = true;
                n += 1;
            }
        }
        if n == 0 {
            return Ok((Vec::new(), Vec::new(), 0));
        }
        // cells of `floor(mz / BIN)` come in order because the peaks do; each cell keeps its intensity-weighted m/z
        let (mut cell, mut sy, mut smy) = (i64::MIN, 0.0f64, 0.0f64);
        let (mut cm, mut cy) = (Vec::new(), Vec::new());
        let total = self.idx.sections[si].n_peaks;
        self.idx.each_peak(si, 0, total, |pos, m, v| {
            if !ok[pos] {
                return;
            }
            let c = (m / BIN).floor() as i64;
            if c != cell {
                if cell != i64::MIN {
                    cm.push(smy / sy.max(1e-12));
                    cy.push(sy);
                }
                (cell, sy, smy) = (c, 0.0, 0.0);
            }
            sy += v;
            smy += v * m;
        })?;
        if cell != i64::MIN {
            cm.push(smy / sy.max(1e-12));
            cy.push(sy);
        }
        let (mz, y) = per_nominal(&cm, &cy);
        Ok((mz, y, n))
    }

    /// `/api/specpeak?rt0=A&rt1=B[&level=N][&precursor=P][&factor=2&side_min=0.3&gap_min=0.05&min_sn=3&min_points=3][&fill_ms=T]`: `rt0`..`rt1` is where the
    /// apex is searched; `None` when the request has no such range.
    pub fn spec_peak_route(&mut self, q: &Query) -> Result<Option<String>> {
        let (Some(rt0), Some(rt1)) = (q.num("rt0")?, q.num("rt1")?) else {
            return Ok(None);
        };
        let (level, fill, precursor, p) = (q.level(), fill_of(q)?, q.num("precursor")?, set_of(q)?);
        let win = (rt0.min(rt1), rt0.max(rt1));
        let (rt, y) = self.level_trace(level, precursor)?;
        let in_range = scans_in(&rt, win);
        let mut o = Out::default();
        head(&mut o, level, win, &p, (rt.len(), in_range));
        if in_range == 0 {
            let rule = format!(
                "no scan of MS level {level} between {} and {} min",
                win.0, win.1
            );
            return Ok(Some(without(o, "no_data", &rule, "null", in_range)));
        }
        let (hit, _, _, edge) = tallest(&rt, &y, win.0, win.1, SMOOTH_HALF, VALLEY_PCT);
        let Some(hit) = hit else {
            let rule = "no peak in the search range of the chromatogram -> no_apex";
            return Ok(Some(without(o, "no_apex", rule, "null", in_range)));
        };
        let apex_json = format!(
            "{{\"rt_min\":{},\"height\":{},\"sn\":{},\"points\":{},\"start_min\":{},\"end_min\":{}}}",
            hit.rt, hit.height, hit.sn, hit.points, hit.start_rt, hit.end_rt
        );
        if let Some(rule) = misses(&hit, &p) {
            return Ok(Some(without(o, "no_apex", &rule, &apex_json, in_range)));
        }
        let g = geometry(&rt, &y, &hit, edge, &p);
        let side_l = side_of(&rt, &y, g.left);
        let side_r = side_of(&rt, &y, g.right);
        let n_side = side_l.0 + side_r.0;
        let background = background_json(n_side, side_l.1 + side_r.1);
        let sum = self.sum_evidence(level, g.window, precursor, fill, &background)?;
        // a profile or high-resolution level: the window and the sides are given, no spectrum (the nominal-mass subtraction does not describe its lines)
        let scans = &self.idx.scans;
        let is_profile = scans.iter().filter(|s| s.level == level).any(|s| s.profile);
        let hr = hr_profile(scans, level == 1, true).on;
        let has_spectrum = !(is_profile || hr);
        let mut flags = g.flags.clone();
        if n_side == 0 {
            flags.push("no_background");
        } else if side_l.0 == 0 || side_r.0 == 0 {
            flags.push("one_side");
        }
        let mut rule = g.rule(&hit, &p);
        let (mut mz, mut yy, mut n_win) = (Vec::new(), Vec::new(), scans_in(&rt, g.window));
        if has_spectrum {
            (mz, yy, n_win) = self.net_spectrum(level, &g, precursor, n_side)?;
        } else {
            rule += &format!("; the level is {}: no spectrum made, take the window with spectrum get --rt A,B --sum", if is_profile { "profile" } else { "high resolution" });
        }
        let f = Found {
            g: &g,
            apex_json: &apex_json,
            sides: (side_l, side_r),
            background: &background,
            sum: &sum,
            flags: &flags,
            rule: &rule,
            has_spectrum,
            n_win,
            lines: (&mz, &yy),
        };
        Ok(Some(emit(o, &f)))
    }

    /// The window's sum minus the mean of both sides scaled to the window's scans, per nominal mass, kept where the net (rounded as shown) is above zero;
    /// with the scans of the window.
    fn net_spectrum(
        &mut self,
        level: u8,
        g: &Geometry,
        precursor: Option<f64>,
        n_side: usize,
    ) -> Result<(Vec<f64>, Vec<f64>, usize)> {
        let (wm, wy, n) = self.nominal_sum(level, g.window, precursor)?;
        let mut sides = Vec::new();
        for w in [g.left, g.right] {
            let (sm, sy, _) = self.nominal_sum(level, w, precursor)?;
            sides.push(sm.iter().map(|&m| nominal(m)).zip(sy).collect::<Vec<_>>());
        }
        let scale = if n_side > 0 {
            n as f64 / n_side as f64
        } else {
            0.0
        };
        let (mut mz, mut yy) = (Vec::new(), Vec::new());
        for (m, v) in wm.iter().zip(&wy) {
            // rounded as it is shown: a mass whose net is a rounding error of the flat background is not a line
            let net = round(
                v - (at(&sides[0], nominal(*m)) + at(&sides[1], nominal(*m))) * scale,
                1,
            );
            if net > 0.0 {
                mz.push(*m);
                yy.push(net);
            }
        }
        Ok((mz, yy, n))
    }
}

/// The cells of a spectrum (in order of m/z) merged per nominal mass: intensities added, m/z weighted by them.
fn per_nominal(mz: &[f64], y: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let (mut om, mut oy) = (Vec::<f64>::new(), Vec::<f64>::new());
    let (mut key, mut sx) = (i64::MIN, 0.0f64);
    for (&m, &v) in mz.iter().zip(y) {
        let k = nominal(m);
        if k != key {
            if let (Some(t), Some(last)) = (oy.last(), om.last_mut()) {
                *last = sx / t.max(1e-12);
            }
            (key, sx) = (k, 0.0);
            om.push(m);
            oy.push(0.0);
        }
        sx += m * v;
        if let Some(t) = oy.last_mut() {
            *t += v;
        }
    }
    if let (Some(t), Some(last)) = (oy.last(), om.last_mut()) {
        *last = sx / t.max(1e-12);
    }
    (om, oy)
}

/// What a sorted list of (nominal mass, intensity) holds at `k`.
fn at(side: &[(i64, f64)], k: i64) -> f64 {
    side.binary_search_by_key(&k, |p| p.0)
        .map_or(0.0, |i| side[i].1)
}

/// The rule that refuses a candidate apex under `min_sn` or `min_points`, `None` when it meets both.
fn misses(hit: &crate::rawxic::Hit, p: &Set) -> Option<String> {
    let (ok_sn, ok_pt) = (hit.sn >= p.min_sn, hit.points >= p.min_points);
    if ok_sn && ok_pt {
        return None;
    }
    let cmp = |ok: bool| if ok { ">=" } else { "<" };
    Some(format!(
        "candidate at {} min: S/N {} {} min_sn {}; points {} {} min_points {} -> no_apex",
        hit.rt,
        hit.sn,
        cmp(ok_sn),
        p.min_sn,
        hit.points,
        cmp(ok_pt),
        p.min_points
    ))
}

/// Where the window and the sides lie.
struct Geometry {
    fwhm: f64,
    half: f64,
    edges: (f64, f64),
    window: (f64, f64),
    anchored: (f64, f64),
    left: (f64, f64),
    right: (f64, f64),
    flags: Vec<&'static str>,
}

impl Geometry {
    /// The rule of a `peak`, every number next to its threshold.
    fn rule(&self, hit: &crate::rawxic::Hit, p: &Set) -> String {
        format!(
            "apex {} min: S/N {} >= min_sn {}; points {} >= min_points {} -> peak; FWHM {} min ({} .. {}); window {} x FWHM = {} .. {} min",
            hit.rt, hit.sn, p.min_sn, hit.points, p.min_points, rd(self.fwhm), rd(self.edges.0), rd(self.edges.1), p.factor, rd(self.window.0), rd(self.window.1)
        )
    }
}

/// The FWHM at the apex of `hit`, the window of `factor` x FWHM around it, and the sides: they start outside the peak the integrator found as well as outside the
/// window, so a tail never counts as background.
fn geometry(rt: &[f64], y: &[f64], hit: &crate::rawxic::Hit, edge: bool, p: &Set) -> Geometry {
    // the width at half height above the baseline the integrator drew under the apex
    let at = rt.iter().position(|&t| t == hit.rt).unwrap_or(0);
    let half = y[at] - hit.height / 2.0;
    let ((l, cut_l), (r, cut_r)) = half_width(rt, y, at, half);
    let fwhm = r - l;
    let mut flags: Vec<&'static str> = Vec::new();
    if edge {
        flags.push("edge");
    }
    if cut_l || cut_r {
        flags.push("fwhm_cut");
    }
    let h = p.factor * fwhm / 2.0;
    let window = (hit.rt - h, hit.rt + h);
    let (a0, a1) = (window.0.min(hit.start_rt), window.1.max(hit.end_rt));
    Geometry {
        fwhm,
        half,
        edges: (l, r),
        window,
        anchored: (a0, a1),
        left: (a0 - p.gap - p.side, a0 - p.gap),
        right: (a1 + p.gap, a1 + p.gap + p.side),
        flags,
    }
}

/// The scans of a trace inside `s` and their summed intensity.
fn side_of(rt: &[f64], y: &[f64], s: (f64, f64)) -> (usize, f64) {
    let inside = (0..rt.len()).filter(|&i| rt[i] >= s.0 && rt[i] <= s.1);
    inside.fold((0, 0.0), |(n, t), i| (n + 1, t + y[i]))
}

/// The scans of a trace inside the window.
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
