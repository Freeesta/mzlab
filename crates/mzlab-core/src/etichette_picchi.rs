//! Labels of the main peaks of a chromatogram (RT + the quantity that defines the peak, as in Xcalibur): which peaks deserve a label,
//! and the main peak of the spectrum taken at an apex (base peak m/z of an MS scan, absorbance maximum of a UV spectrum).
//! Used by the chromatogram drawing through `mzlab-wasm`; the peaks come from `peaks::detect_peaks`, no second detector.

use crate::peaks::{detect_full, Params};

/// A peak chosen for a label: index of the apex in the trace, its RT, the height of the peak and its S/N.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Label {
    pub apex_i: usize,
    pub rt: f64,
    pub height: f64,
    pub snr: f64,
}

/// Minutes at the start of a run (solvent front) where no peak is labelled, at most `SKIP_INI_FRAC` of the run.
pub const SKIP_INI_MIN: f64 = 3.0;
/// Minutes at the end of a run (wash, re-equilibration) where no peak is labelled, at most `SKIP_FIN_FRAC` of the run.
pub const SKIP_FIN_MIN: f64 = 5.0;
/// The fractions keep a useful zone in short runs (MRM, demos of a few minutes).
pub const SKIP_INI_FRAC: f64 = 0.15;
pub const SKIP_FIN_FRAC: f64 = 0.25;

/// Two apexes of one label are two peaks only if the smoothed, baseline-corrected trace between them falls under this fraction of the lower apex.
/// The detector's own merge works in samples, so a broad peak sampled finely keeps two apexes (jitter, shoulder) that must not get two labels.
/// A valley is also not real when the lower apex rises above it by less than `snr_min` times the noise (the S/N a label itself must have): on a noisy top the ripple is deeper than 15 % of the apex.
/// Two peaks closer than `MIN_SEP_FWHM` times the FWHM of the taller one cannot make a valley (two equal Gaussians are bimodal only beyond 0.85 FWHM), so they are one peak.
pub const VALLEY_FRAC: f64 = 0.85;
pub const MIN_SEP_FWHM: f64 = 0.5;
/// A peak needs a prominence over its surroundings of at least this many noise sigmas on the width-following smoothing.
pub const PROM_SIGMA: f64 = 3.0;

/// The raw trace `y` (prominence and valleys do not depend on the baseline, which the detector may bend under a broad peak) smoothed (quadratic
/// Savitzky-Golay) with a half window of 30 % of the half-height width of the tallest peak, and that half window in points.
/// The width is measured on a light smoothing (three points each side) above the 10th percentile of the trace, so a high baseline does not widen it
/// and two close peaks are not smoothed into one hump; the returned trace has that level subtracted, so valley ratios and widths are measured on the peaks.
fn wide_smooth(rt: &[f64], y: &[f64], found: &[crate::peaks::Peak]) -> (usize, Vec<f64>) {
    let coarse = crate::peakq_lr::sg_filter(y, 3, false);
    let mut sorted = coarse.clone();
    crate::sortc::slice_by(&mut sorted, |a, b| crate::sortc::asc_nan_last(*a, *b));
    let base = sorted[sorted.len() / 10];
    let above: Vec<f64> = coarse.iter().map(|v| v - base).collect();
    let apex = found.first().map_or(0, |p| p.apex_i).min(above.len() - 1);
    let w = crate::signal::fwhm_walk(rt, &above, apex);
    let step = (rt[rt.len() - 1] - rt[0]) / (rt.len() - 1) as f64;
    let half = if step > 0.0 {
        ((0.3 * w / step) as usize).clamp(2, 100)
    } else {
        2
    };
    let sm = crate::peakq_lr::sg_filter(y, half, false)
        .into_iter()
        .map(|v| v - base)
        .collect();
    (half, sm)
}

/// The highest smoothed point within `half` points of the detector's apex: the true apex of a ripple of the unsmoothed trace.
fn refine_apex(sm: &[f64], apex: usize, half: usize) -> usize {
    let (a, b) = (apex.saturating_sub(half), (apex + half).min(sm.len() - 1));
    (a..=b).fold(apex, |best, i| if sm[i] > sm[best] { i } else { best })
}

/// Height of the apex over the higher of the two lowest points met walking down each side until a point above the apex (or the trace end).
fn prominence(sm: &[f64], a: usize) -> f64 {
    let side = |it: &mut dyn Iterator<Item = usize>| {
        let mut m = sm[a];
        for i in it {
            if sm[i] > sm[a] {
                break;
            }
            m = m.min(sm[i]);
        }
        m
    };
    let l = side(&mut (0..a).rev());
    let r = side(&mut (a + 1..sm.len()));
    sm[a] - l.max(r)
}

/// Width at half height (in `rt` units) of the peak whose apex is `apex` on the trace `ys`, walking down each side to the first point under half the apex.
fn fwhm(rt: &[f64], ys: &[f64], apex: usize) -> f64 {
    crate::signal::fwhm_walk(rt, ys, apex)
}

/// At most `max_n` peaks with S/N ≥ `snr_min` and height ≥ `rel_min` times the tallest candidate, by increasing RT.
/// With `skip_ends` (`rt` in minutes) the peaks in the first and last minutes of the run are not candidates; labels only: the detector and everything else still see them.
/// A spectrum (nm axis) is labelled with `skip_ends` false.
pub fn select(
    rt: &[f64],
    y: &[f64],
    max_n: usize,
    rel_min: f64,
    snr_min: f64,
    skip_ends: bool,
) -> Vec<Label> {
    let n = rt.len().min(y.len());
    if n < 3 || max_n == 0 {
        return Vec::new();
    }
    // the peaks come by decreasing height; `ys` is the trace they were found on, the one the valleys are measured on
    let det = detect_full(&rt[..n], &y[..n], Params::default());
    let (found, ys, noise) = (det.peaks, det.ys, det.noise);
    let (t0, t1) = (rt[0], rt[n - 1]);
    let span = t1 - t0;
    let (lo, hi) = if skip_ends {
        (
            t0 + SKIP_INI_MIN.min(SKIP_INI_FRAC * span),
            t1 - SKIP_FIN_MIN.min(SKIP_FIN_FRAC * span),
        )
    } else {
        (t0, t1)
    };
    // Apexes and valleys are judged on a trace smoothed with a window that follows the peak width (the detector's own 5-point smoothing leaves the ripple
    // of a finely sampled MRM peak); the integration never sees it, it only decides which peaks are real and where their apex is.
    let wide = wide_smooth(&rt[..n], &y[..n], &found);
    let half = wide.0;
    let sm = &wide.1;
    let mut cands: Vec<(&crate::peaks::Peak, usize)> = Vec::new();
    for p in found.iter().filter(|p| (lo..=hi).contains(&rt[p.apex_i])) {
        let a = refine_apex(sm, p.apex_i, half);
        if prominence(sm, a) < PROM_SIGMA * noise {
            continue;
        }
        // a taller peak already kept without a real valley between: same peak
        let same = cands.iter().any(|&(_, qa)| {
            let (i, j) = (a.min(qa), a.max(qa));
            let valley = sm[i..=j].iter().copied().fold(f64::INFINITY, f64::min);
            let lower = sm[a].min(sm[qa]);
            let taller = if sm[a] >= sm[qa] { a } else { qa };
            i == j
                || valley >= VALLEY_FRAC * lower
                || lower - valley <= snr_min * noise
                || (rt[j] - rt[i]) < MIN_SEP_FWHM * fwhm(&rt[..n], sm, taller)
        });
        if !same {
            cands.push((p, a));
        }
    }
    // the threshold is relative to the tallest candidate: a huge solvent peak must not hide the real ones
    let top = cands.iter().map(|(p, _)| p.height).fold(0.0, f64::max);
    let mut out: Vec<Label> = cands
        .iter()
        .filter(|(p, _)| p.snr >= snr_min && p.height >= rel_min * top)
        .take(max_n)
        .map(|&(p, a)| Label {
            apex_i: a,
            rt: rt[a],
            height: p.height,
            snr: p.snr,
        })
        .collect();
    crate::sortc::slice_by_key(&mut out, |l| l.apex_i);
    out
}

/// The tallest point of a spectrum (`x` = m/z or nm): `(x, y)`; a UV curve is refined with the parabola through the apex and its two neighbours.
/// `None` when there is no positive point.
pub fn main_peak(x: &[f64], y: &[f64], refine: bool) -> Option<(f64, f64)> {
    let n = x.len().min(y.len());
    let mut best: Option<usize> = None;
    for i in 0..n {
        if y[i] > 0.0 && best.is_none_or(|b| y[i] > y[b]) {
            best = Some(i);
        }
    }
    let i = best?;
    if refine && i > 0 && i + 1 < n {
        let (y0, y1, y2) = (y[i - 1], y[i], y[i + 1]);
        let d = y0 - 2.0 * y1 + y2;
        if d < 0.0 {
            let t = (0.5 * (y0 - y2) / d).clamp(-1.0, 1.0);
            let step = if t < 0.0 {
                x[i] - x[i - 1]
            } else {
                x[i + 1] - x[i]
            };
            return Some((x[i] + t * step, y1 - 0.25 * (y0 - y2) * t));
        }
    }
    Some((x[i], y[i]))
}
