//! The retention-time shift between two DAD matrices, estimated (`dad ... --shift auto`, `/api/pdasub` with `auto`): the total traces of A and B (mean over
//! the wavelengths) lose their slow baseline, and the shift is the lag at which they correlate best, refined to a fraction of a step. The numbers behind it
//! (correlation at the lag and at zero, the search range, the window used) are returned so the user can accept or reject it; nothing is applied silently.

use super::dad::DadData;
use std::fmt::Write;

/// The largest shift searched, in minutes (either sign): the drift between two runs of one method, not a different method.
const MAX_SHIFT_MIN: f64 = 0.5;
/// Half-width of the window of the baseline (a rolling minimum) removed from each trace, in minutes: wider than a peak.
const BASELINE_HALF_MIN: f64 = 0.5;
/// Padding around a requested time window, in minutes: the shift is estimated where the spectrum is read, with this much context.
pub const FOCUS_PAD_MIN: f64 = 1.0;
/// The fewest grid points a window must hold to be correlated; with fewer, the whole run is used (`basis: "run"`).
const MIN_POINTS: usize = 40;
/// A best correlation below this is reported as weak: the two runs share little to align on.
const WEAK_CORR: f64 = 0.5;
/// A shift below this (0.006 s) is rounding of the grids, not a drift: it is reported as 0.
const NEGLIGIBLE_MIN: f64 = 1e-4;
/// The most grid points of a correlation (the step is widened above it).
const MAX_POINTS: usize = 20_000;

/// What the estimate found and on what it stands.
#[derive(Debug, Clone, PartialEq)]
pub struct Align {
    /// B is read at `t - shift_min` (a positive shift moves B later), the sign of `SubOpts::shift_min`.
    pub shift_min: f64,
    /// Pearson correlation of the baseline-free traces at the shift found, and at no shift.
    pub corr: f64,
    pub corr_at_zero: f64,
    /// The shifts searched: `-search_min ..= search_min`.
    pub search_min: f64,
    /// `window` (the requested time window with its padding) or `run` (the whole overlap).
    pub basis: &'static str,
    pub window_min: (f64, f64),
    pub points: usize,
    pub step_min: f64,
    /// The codes of what is doubtful: `shift_auto_flat` (no peak to align on: the shift is 0), `shift_auto_limit` (the best lag is at the edge of the search), `shift_auto_weak` (the best correlation is below 0.5).
    pub warnings: Vec<&'static str>,
}

fn row_means(d: &DadData) -> Vec<f64> {
    let n = d.channels().max(1) as f64;
    (0..d.points())
        .map(|i| {
            d.spectrum_at(i)
                .map_or(0.0, |r| r.iter().map(|&v| f64::from(v)).sum::<f64>() / n)
        })
        .collect()
}

/// `y` at `t` by linear interpolation on the ascending `x`; `None` outside the axis.
fn interp(x: &[f64], y: &[f64], t: f64) -> Option<f64> {
    let (lo, hi) = (*x.first()?, *x.last()?);
    if !t.is_finite() || t < lo || t > hi {
        return None;
    }
    let i = x
        .partition_point(|&v| v <= t)
        .saturating_sub(1)
        .min(x.len().saturating_sub(2));
    let (x0, x1) = (*x.get(i)?, *x.get(i + 1)?);
    let (y0, y1) = (*y.get(i)?, *y.get(i + 1)?);
    Some(if x1 > x0 {
        y0 + (y1 - y0) * (t - x0) / (x1 - x0)
    } else {
        y0
    })
}

fn median_step(t: &[f64]) -> f64 {
    let mut d: Vec<f64> = t
        .windows(2)
        .filter_map(|w| Some(w.get(1)? - w.first()?))
        .filter(|x| *x > 0.0)
        .collect();
    crate::sortc::slice_by(&mut d, f64::total_cmp);
    d.get(d.len() / 2).copied().unwrap_or(0.0)
}

/// The trace without its lower envelope (a rolling minimum over `±half` points): what is left is the peaks.
fn without_baseline(y: &[f64], half: usize) -> Vec<f64> {
    let n = y.len();
    y.iter()
        .enumerate()
        .map(|(i, v)| {
            let (a, b) = (i.saturating_sub(half), (i + half + 1).min(n));
            v - y
                .get(a..b)
                .map_or(0.0, |w| w.iter().copied().fold(f64::INFINITY, f64::min))
        })
        .collect()
}

/// Pearson correlation of `a` and `b` over the pairs where `b` is known; `None` with too few pairs or a flat series.
fn pearson(a: &[f64], b: &[Option<f64>]) -> Option<f64> {
    let pairs: Vec<(f64, f64)> = a
        .iter()
        .zip(b)
        .filter_map(|(x, y)| Some((*x, (*y)?)))
        .collect();
    if pairs.len() < MIN_POINTS || pairs.len() * 2 < a.len() {
        return None;
    }
    let n = pairs.len() as f64;
    let (ma, mb) = (
        pairs.iter().map(|p| p.0).sum::<f64>() / n,
        pairs.iter().map(|p| p.1).sum::<f64>() / n,
    );
    let (mut sab, mut saa, mut sbb) = (0.0, 0.0, 0.0);
    for (x, y) in pairs {
        sab += (x - ma) * (y - mb);
        saa += (x - ma) * (x - ma);
        sbb += (y - mb) * (y - mb);
    }
    (saa > 1e-12 && sbb > 1e-12).then(|| sab / (saa * sbb).sqrt())
}

/// The grid step, the basis (`window` or `run`) and the time window the correlation is made on: the focus when it holds enough points, else the overlap.
#[allow(clippy::cast_possible_truncation)] // grid counts, small and non-negative
fn plan(a: &DadData, b: &DadData, focus: Option<(f64, f64)>) -> (f64, &'static str, (f64, f64)) {
    let edge = |d: &DadData| {
        (
            d.time_min.first().copied().unwrap_or(0.0),
            d.time_min.last().copied().unwrap_or(0.0),
        )
    };
    let (ea, eb) = (edge(a), edge(b));
    let overlap = (ea.0.max(eb.0), ea.1.min(eb.1));
    let step = median_step(&a.time_min)
        .min(median_step(&b.time_min))
        .max(1e-6);
    let step = step.max((overlap.1 - overlap.0) / MAX_POINTS as f64);
    if let Some((f0, f1)) = focus {
        let w = (f0.max(overlap.0), f1.min(overlap.1));
        if w.1 > w.0 && ((w.1 - w.0) / step) as usize + 1 >= MIN_POINTS {
            return (step, "window", w);
        }
    }
    (step, "run", overlap)
}

/// The Pearson correlation of the baseline-free traces at every lag of `-max_lag..=max_lag` steps; B is read at `t - lag * step`.
fn correlate(
    a: &DadData,
    b: &DadData,
    t: &[f64],
    step: f64,
    max_lag: i64,
) -> Vec<(i64, Option<f64>)> {
    // the baseline is removed on each run's own rows
    #[allow(clippy::cast_possible_truncation)]
    let peaks = |d: &DadData| {
        let half = (BASELINE_HALF_MIN / median_step(&d.time_min).max(1e-6)).ceil() as usize;
        without_baseline(&row_means(d), half)
    };
    let (ba, bb) = (peaks(a), peaks(b));
    let ga: Vec<f64> = t
        .iter()
        .map(|&x| interp(&a.time_min, &ba, x).unwrap_or(0.0))
        .collect();
    let at = |lag: i64| {
        let gb: Vec<Option<f64>> = t
            .iter()
            .map(|&x| interp(&b.time_min, &bb, x - lag as f64 * step))
            .collect();
        pearson(&ga, &gb)
    };
    (-max_lag..=max_lag).map(|k| (k, at(k))).collect()
}

/// The lag of the best correlation, placed between two steps by a parabola through it and its neighbours: `(lags, correlation)`.
fn best_lag(r: &[(i64, Option<f64>)]) -> Option<(f64, f64)> {
    let (k, c) = r.iter().filter_map(|(k, c)| Some((*k, (*c)?))).fold(
        None::<(i64, f64)>,
        |m, p| match m {
            Some(q) if q.1 >= p.1 => Some(q),
            _ => Some(p),
        },
    )?;
    let around = |j: i64| r.iter().find(|(l, _)| *l == j).and_then(|(_, c)| *c);
    let frac = match (around(k - 1), around(k + 1)) {
        (Some(l), Some(u)) if (l - 2.0 * c + u).abs() > 1e-12 => {
            (0.5 * (l - u) / (l - 2.0 * c + u)).clamp(-0.5, 0.5)
        }
        _ => 0.0,
    };
    Some((k as f64 + frac, c))
}

/// The shift of B that best matches A. `focus` is the time window the spectrum or trace is read in (with its padding): the shift is estimated there when it
/// holds enough points, else on the whole overlap. A flat trace (no peak) gives shift 0 with `shift_auto_flat`; the result is never an error.
#[allow(clippy::cast_possible_truncation)] // grid counts, small and non-negative
pub fn dad_align(a: &DadData, b: &DadData, focus: Option<(f64, f64)>) -> Align {
    let (step, basis, win) = plan(a, b, focus);
    let t: Vec<f64> = (0..)
        .map(|k| win.0 + k as f64 * step)
        .take_while(|t| *t <= win.1 + 1e-9)
        .collect();
    let flat = |warnings: Vec<&'static str>| Align {
        shift_min: 0.0,
        corr: 0.0,
        corr_at_zero: 0.0,
        search_min: 0.0,
        basis,
        window_min: win,
        points: 0,
        step_min: step,
        warnings,
    };
    if t.len() < MIN_POINTS {
        return flat(vec!["shift_auto_flat"]);
    }
    let max_lag = (MAX_SHIFT_MIN.min((win.1 - win.0) / 4.0) / step).floor() as i64;
    let r = correlate(a, b, &t, step, max_lag);
    let Some((lag, c)) = best_lag(&r) else {
        return flat(vec!["shift_auto_flat"]);
    };
    let shift = lag * step;
    let edge = lag.abs() > max_lag as f64 - 0.5;
    let warnings = [
        (edge, "shift_auto_limit"),
        (c < WEAK_CORR, "shift_auto_weak"),
    ]
    .into_iter()
    .filter_map(|(on, w)| on.then_some(w))
    .collect();
    Align {
        shift_min: if shift.abs() < NEGLIGIBLE_MIN {
            0.0
        } else {
            shift
        },
        corr: c,
        corr_at_zero: r
            .iter()
            .find(|(l, _)| *l == 0)
            .and_then(|(_, c)| *c)
            .unwrap_or(0.0),
        search_min: max_lag as f64 * step,
        basis,
        window_min: win,
        points: t.len(),
        step_min: step,
        warnings,
    }
}

fn num(x: f64) -> String {
    let r = (x * 1e6).round() / 1e6;
    if r.is_finite() {
        format!("{r}")
    } else {
        "null".into()
    }
}

impl Align {
    /// The estimate as JSON with stable keys (the `align` member of the `sub` report).
    pub fn to_json(&self) -> String {
        let mut s = String::new();
        let _ = write!(
            s,
            "{{\"method\":\"twc_xcorr\",\"shift_min\":{},\"corr\":{},\"corr_at_zero\":{},\"search_min\":{},\"basis\":\"{}\",\"window_min\":[{},{}],\"points\":{},\"step_min\":{},\"warnings\":[{}]}}",
            num(self.shift_min), num(self.corr), num(self.corr_at_zero), num(self.search_min), self.basis,
            num(self.window_min.0), num(self.window_min.1), self.points, num(self.step_min),
            self.warnings.iter().map(|w| format!("\"{w}\"")).collect::<Vec<_>>().join(",")
        );
        s
    }
}

#[cfg(test)]
#[path = "dad_align_tests.rs"]
mod tests;
