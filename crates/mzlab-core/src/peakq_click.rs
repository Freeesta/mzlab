//! One click on a low-resolution trace: the peak whose apex is nearest the click (not the tallest one in reach), its edges from `lr_edges_detail`,
//! the evidence (points, area, height, noise, S/N) and a guard against junk windows. A refused click can be forced; the user decides.
//! Used by `lr_click` of mzlab-wasm (page and desktop) and `mzlab-cli peak edges --set click=1`.

use crate::json::Out;
use crate::peakq_lr::{lr_edges_detail, near_idx, residual_shrink, sg_filter, LrParams};
use crate::sortc;

/// The guard of a click and the search reach; every number is logged in the output (`params`) and set from the CLI (`--set`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClickGuard {
    /// Fewest points a window may have: below five points a peak has no shape to measure an area on.
    pub min_points: usize,
    /// Smallest S/N (apex height over the edge line / noise sigma of the raw trace); 3 is the usual limit of detection.
    pub min_sn: f64,
    /// How far (min) from the click an apex is looked for.
    pub reach: f64,
    /// Keep the window although the guard refused it (the reasons stay in `refused`).
    pub force: bool,
}

impl Default for ClickGuard {
    fn default() -> Self {
        ClickGuard {
            min_points: 5,
            min_sn: 3.0,
            reach: 0.25,
            force: false,
        }
    }
}

impl ClickGuard {
    /// The four numbers of the wasm interface: min_points, min_sn, reach, force (0 or 1).
    pub fn from_slice(p: &[f64]) -> Option<ClickGuard> {
        let [min_points, min_sn, reach, force] = <[f64; 4]>::try_from(p.get(..4)?).ok()?;
        Some(ClickGuard {
            min_points: min_points.max(0.0) as usize,
            min_sn,
            reach,
            force: force != 0.0,
        })
    }
}

/// What a click found. Without an apex in reach the window is the one of the plain click at that time, refused with `no_peak` among the reasons.
#[derive(Clone, Debug)]
pub struct Click {
    pub apex: Option<f64>,
    pub edges: Option<(f64, f64)>,
    pub points: usize,
    pub area: f64,
    pub height: f64,
    pub noise: f64,
    pub sn: f64,
    /// Why the guard refused (`no_peak`, `no_edges`, `points`, `area`, `sn`); with `force` the reasons that were overridden.
    pub refused: Vec<&'static str>,
    pub forced: bool,
    pub guard: ClickGuard,
}

impl Click {
    /// The window is kept: it exists, and nothing was refused or the user forced it. Without edges there is nothing to keep, forced or not.
    pub fn ok(&self) -> bool {
        self.edges.is_some() && (self.refused.is_empty() || self.forced)
    }

    /// The one JSON text of site, app and CLI (keys never move; `null` for what was not measured).
    pub fn to_json(&self) -> String {
        let mut o = Out(String::new());
        o.raw("{\"ok\":")
            .raw(if self.ok() { "true" } else { "false" })
            .raw(",\"edges\":");
        match self.edges {
            Some((a, b)) => o.raw("[").num(a).raw(",").num(b).raw("]"),
            None => o.raw("null"),
        };
        o.raw(",\"apex\":");
        match self.apex {
            Some(a) => o.num(a),
            None => o.raw("null"),
        };
        o.raw(",\"points\":").raw(&self.points.to_string());
        for (k, v) in [
            ("area", self.area),
            ("height", self.height),
            ("noise", self.noise),
            ("sn", self.sn),
        ] {
            o.raw(",\"").raw(k).raw("\":").num(v);
        }
        o.raw(",\"refused\":[");
        for (i, r) in self.refused.iter().enumerate() {
            if i > 0 {
                o.raw(",");
            }
            o.str(r);
        }
        let g = &self.guard;
        o.raw("],\"forced\":")
            .raw(if self.forced { "true" } else { "false" });
        o.raw(",\"params\":{\"min_points\":")
            .raw(&g.min_points.to_string());
        o.raw(",\"min_sn\":")
            .num(g.min_sn)
            .raw(",\"reach\":")
            .num(g.reach);
        o.raw(",\"force\":")
            .raw(if g.force { "true" } else { "false" })
            .raw("}}");
        o.0
    }
}

fn median_abs(v: impl Iterator<Item = f64>) -> f64 {
    let mut a: Vec<f64> = v.map(f64::abs).collect();
    if a.is_empty() {
        return 0.0;
    }
    sortc::by(&mut a, |x, y| x.total_cmp(y));
    a[a.len() >> 1]
}

/// Apex (index) nearest the click `x` among the local maxima of the lightly smoothed trace within `reach`, with a prominence of at least
/// 3 noise sigmas over the lower of its two valleys (so a noise wiggle is no apex). `None` when there is none.
fn nearest_apex(xs: &[f64], ys: &[f64], x: f64, reach: f64, pk: &LrParams) -> Option<usize> {
    let (a, b) = (near_idx(xs, x - pk.win), near_idx(xs, x + pk.win));
    let dt = (xs[b] - xs[a]) / ((b - a).max(1) as f64);
    let m = ((0.03 / dt.max(1e-9) / 2.0 + 0.5).floor().max(2.0)) as usize;
    let co = sg_filter(&ys[a..=b], m, false);
    // sigma of the raw trace: the residual of the smoothing over its shrink factor (the scale `lr_edges` uses for its S/N)
    let sigma =
        1.4826 * median_abs(ys[a..=b].iter().zip(&co).map(|(y, c)| y - c)) / residual_shrink(m);
    let mut best: Option<(f64, usize)> = None;
    for j in 1..co.len().saturating_sub(1) {
        let i = a + j;
        if !(co[j] > co[j - 1] && co[j] >= co[j + 1]) || (xs[i] - x).abs() > reach {
            continue;
        }
        let side = |dir: i64| {
            let (mut k, mut lo) = (j as i64, co[j]);
            while k + dir >= 0 && (k + dir) < co.len() as i64 && co[(k + dir) as usize] <= co[j] {
                k += dir;
                lo = lo.min(co[k as usize]);
            }
            lo
        };
        if co[j] - side(-1).max(side(1)) < 3.0 * sigma {
            continue;
        }
        let d = (xs[i] - x).abs();
        if best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, i));
        }
    }
    best.map(|(_, i)| i)
}

/// Points, area (counts x s), height over the edge line and S/N of the window [e0, e1]: the line joins the means of 3 points at each end (never below zero),
/// as `integ()` of the page.
fn measure(xs: &[f64], ys: &[f64], e0: f64, e1: f64, noise: f64) -> (usize, f64, f64, f64) {
    let n = xs.len();
    let (i0, i1) = (near_idx(xs, e0), near_idx(xs, e1));
    let end = |i: usize| {
        let (lo, hi) = (i.saturating_sub(1), (i + 1).min(n - 1));
        (ys[lo..=hi].iter().sum::<f64>() / (hi - lo + 1) as f64).max(0.0)
    };
    let (ya, yb) = (end(i0), end(i1));
    let span = (xs[i1] - xs[i0]).max(1e-12);
    let mut top = f64::NEG_INFINITY;
    let mut raw = 0.0;
    for j in i0..=i1 {
        let line = ya + (yb - ya) * (xs[j] - xs[i0]) / span;
        if ys[j] - line > top {
            top = ys[j] - line;
        }
        if j > i0 {
            raw += (ys[j] + ys[j - 1]) / 2.0 * (xs[j] - xs[j - 1]) * 60.0;
        }
    }
    let area = (raw - (ya + yb) / 2.0 * span * 60.0).max(0.0);
    let height = top.max(0.0);
    (
        i1 + 1 - i0,
        area,
        height,
        if noise > 0.0 { height / noise } else { 0.0 },
    )
}

/// The click at `x` on the trace: the window of the peak whose apex is nearest, its evidence and the verdict of the guard.
pub fn lr_click(xs: &[f64], ys: &[f64], x: f64, mrm: bool, pk: &LrParams, g: &ClickGuard) -> Click {
    let n = xs.len().min(ys.len());
    let (xs, ys) = (&xs[..n], &ys[..n]);
    let mut c = Click {
        apex: None,
        edges: None,
        points: 0,
        area: 0.0,
        height: 0.0,
        noise: 0.0,
        sn: 0.0,
        refused: Vec::new(),
        forced: false,
        guard: *g,
    };
    if n < 9 {
        c.refused.push("no_peak");
        return c;
    }
    let apex = nearest_apex(xs, ys, x, g.reach, pk);
    if apex.is_none() {
        c.refused.push("no_peak");
    }
    // the edges are searched from the apex alone (a forced click without one: at the click, as the plain `peak edges`)
    let at = apex.map_or(x, |i| xs[i]);
    let ia = near_idx(xs, at);
    let (lo, hi) = (ia.saturating_sub(50), (n - 1).min(ia + 50));
    let dt = (xs[hi] - xs[lo]) / ((hi - lo).max(1) as f64);
    let search = LrParams {
        sn_min: 0.0,
        apex_win: if apex.is_some() {
            1.5 * dt.abs()
        } else {
            pk.apex_win
        },
        ..*pk
    };
    let Some(e) = lr_edges_detail(xs, ys, at, 0.0, mrm, &search) else {
        c.refused.push("no_edges");
        return c;
    };
    c.apex = apex.map(|i| xs[i]);
    c.edges = Some((e.e0, e.e1));
    let m = measure(xs, ys, e.e0, e.e1, e.noise);
    (c.points, c.area, c.height, c.sn) = (m.0, m.1, m.2, m.3);
    c.noise = e.noise;
    let span = (xs[near_idx(xs, e.e1)] - xs[near_idx(xs, e.e0)]).max(1e-12);
    let min_sn = g.min_sn.max(pk.sn_min);
    if c.points < g.min_points {
        c.refused.push("points");
    }
    if c.area <= e.noise * span * 60.0 {
        c.refused.push("area");
    }
    if c.sn < min_sn {
        c.refused.push("sn");
    }
    c.forced = g.force && !c.refused.is_empty();
    c
}
