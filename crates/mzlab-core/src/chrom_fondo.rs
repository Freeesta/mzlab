//! Automatic background of a chromatogram: the blank trace when one is loaded, otherwise the steadiest flat window of the run, plus a local baseline per peak.
//! Pure functions on (time, intensity) arrays; the page only draws what they return and lets the user move the window.
use crate::budget::try_vec;
use crate::signal::median;

/// Knobs of the automatic background. `Default` is what the page uses until the user changes something.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FondoParams {
    /// Window width as a fraction of the run (points), default 5 %.
    pub win_frac: f64,
    /// Smallest window in points, default 10.
    pub min_pts: usize,
    /// Weight of the MAD in the window score `median + k * MAD`, default 3.
    pub mad_k: f64,
    /// Local baseline: a walk away from the apex stops when the trace has climbed this fraction of the peak height above its lowest point, default 0.1.
    pub side_tol: f64,
}

impl Default for FondoParams {
    fn default() -> Self {
        Self {
            win_frac: 0.05,
            min_pts: 10,
            mad_k: 3.0,
            side_tol: 0.1,
        }
    }
}

/// Where a background comes from (the page words it as «blank», «flat window», «local»).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    Blank,
    FlatWindow,
    Local,
}

/// A whole-trace background: one level per point, its origin, and (flat window only) the window as point indices `[start, end)`.
#[derive(Clone, Debug, PartialEq)]
pub struct Background {
    pub origin: Origin,
    pub level: Vec<f64>,
    pub window: Option<(usize, usize)>,
}

/// The straight baseline of one peak and the baseline-corrected height and area (intensity x time units).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LocalBackground {
    pub left: usize,
    pub right: usize,
    pub height: f64,
    pub area: f64,
}

fn valid(t: &[f64], y: &[f64]) -> bool {
    t.len() == y.len() && t.len() >= 2 && t.windows(2).all(|w| w[1] > w[0])
}

/// Linear interpolation of the blank (`bt`, `by`) at the times `t`; outside the blank's range the nearest end is kept.
pub fn blank_on(t: &[f64], bt: &[f64], by: &[f64]) -> Option<Vec<f64>> {
    if !valid(bt, by) {
        return None;
    }
    let mut out = try_vec(t.len()).ok()?;
    let mut j = 0;
    for &x in t {
        while j + 2 < bt.len() && bt.get(j + 1).is_some_and(|&b| b < x) {
            j += 1;
        }
        let (t0, t1) = (*bt.get(j)?, *bt.get(j + 1)?);
        let (y0, y1) = (*by.get(j)?, *by.get(j + 1)?);
        let f = ((x - t0) / (t1 - t0)).clamp(0.0, 1.0);
        out.push(y0 + f * (y1 - y0));
    }
    Some(out)
}

fn mad(v: &[f64], med: f64) -> f64 {
    let dev: Vec<f64> = v.iter().map(|x| (x - med).abs()).collect();
    1.4826 * median(&dev)
}

/// The window of `w` points with the lowest `median + k * MAD`: low and steady, so a ramp or a peak scores badly.
/// Returns `(start, end, median)`; `None` when the trace is shorter than the window.
pub fn flat_window(y: &[f64], p: &FondoParams) -> Option<(usize, usize, f64)> {
    let n = y.len();
    let w = ((n as f64 * p.win_frac).round() as usize).max(p.min_pts);
    if w > n || w == 0 {
        return None;
    }
    let step = (w / 4).max(1);
    let mut best: Option<(f64, usize, f64)> = None;
    let mut s = 0;
    loop {
        let seg = y.get(s..s + w)?;
        let m = median(seg);
        let score = m + p.mad_k * mad(seg, m);
        if best.is_none_or(|b| score < b.0) {
            best = Some((score, s, m));
        }
        if s + w >= n {
            break;
        }
        s = (s + step).min(n - w);
    }
    best.map(|(_, s, m)| (s, s + w, m))
}

/// The automatic background of a chromatogram: the blank interpolated on `t` when given, otherwise a constant at the median of the steadiest flat window.
pub fn auto_background(
    t: &[f64],
    y: &[f64],
    blank: Option<(&[f64], &[f64])>,
    p: &FondoParams,
) -> Option<Background> {
    if !valid(t, y) {
        return None;
    }
    if let Some((bt, by)) = blank {
        let level = blank_on(t, bt, by)?;
        return Some(Background {
            origin: Origin::Blank,
            level,
            window: None,
        });
    }
    let (s, e, m) = flat_window(y, p)?;
    let mut level = try_vec(y.len()).ok()?;
    level.resize(y.len(), m);
    Some(Background {
        origin: Origin::FlatWindow,
        level,
        window: Some((s, e)),
    })
}

/// The same flat-window background for a window the user placed (`start..end` in points, at least two): the level is that window's median.
pub fn background_at(y: &[f64], start: usize, end: usize) -> Option<Background> {
    let seg = y.get(start..end).filter(|s| s.len() >= 2)?;
    let mut level = try_vec(y.len()).ok()?;
    level.resize(y.len(), median(seg));
    Some(Background {
        origin: Origin::FlatWindow,
        level,
        window: Some((start, end)),
    })
}

/// Walks from the apex towards `dir` (-1 / +1) to the local minimum: it stops once the trace has risen more than `tol * rise` above the lowest point seen.
fn side_min(y: &[f64], apex: usize, dir: isize, tol: f64) -> usize {
    let top = y.get(apex).copied().unwrap_or(0.0);
    let (mut best, mut i) = (apex, apex as isize);
    loop {
        i += dir;
        let Some(&v) = usize::try_from(i).ok().and_then(|k| y.get(k)) else {
            return best;
        };
        let low = y.get(best).copied().unwrap_or(0.0);
        if v < low {
            best = i as usize;
        } else if v > low + tol * (top - low).max(0.0) {
            return best;
        }
    }
}

/// The local background of the peak with apex `apex`: the straight line between the local minima on both sides, and the height and area above it.
pub fn local_background(
    t: &[f64],
    y: &[f64],
    apex: usize,
    p: &FondoParams,
) -> Option<LocalBackground> {
    if !valid(t, y) || apex >= y.len() {
        return None;
    }
    let (l, r) = (
        side_min(y, apex, -1, p.side_tol),
        side_min(y, apex, 1, p.side_tol),
    );
    if l == apex || r == apex {
        return None;
    }
    let (tl, tr) = (*t.get(l)?, *t.get(r)?);
    let (yl, yr) = (*y.get(l)?, *y.get(r)?);
    let line = |x: f64| yl + (yr - yl) * (x - tl) / (tr - tl);
    let height = *y.get(apex)? - line(*t.get(apex)?);
    let mut area = 0.0;
    for k in l..r {
        let (a, b) = (t.get(k).zip(y.get(k)), t.get(k + 1).zip(y.get(k + 1)));
        let (Some((&ta, &ya)), Some((&tb, &yb))) = (a, b) else {
            return None;
        };
        let (ha, hb) = ((ya - line(ta)).max(0.0), (yb - line(tb)).max(0.0));
        area += (ha + hb) / 2.0 * (tb - ta);
    }
    Some(LocalBackground {
        left: l,
        right: r,
        height,
        area,
    })
}
