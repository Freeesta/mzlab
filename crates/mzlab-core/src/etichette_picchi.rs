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
pub const VALLEY_FRAC: f64 = 0.85;

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
    let (found, ys) = (det.peaks, det.ys);
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
    let mut cands: Vec<&crate::peaks::Peak> = Vec::new();
    for p in found.iter().filter(|p| (lo..=hi).contains(&rt[p.apex_i])) {
        // a taller peak already kept without a real valley between: same peak
        let same = cands.iter().any(|q| {
            let (a, b) = (p.apex_i.min(q.apex_i), p.apex_i.max(q.apex_i));
            let valley = ys[a..=b].iter().copied().fold(f64::INFINITY, f64::min);
            valley >= VALLEY_FRAC * ys[p.apex_i].min(ys[q.apex_i])
        });
        if !same {
            cands.push(p);
        }
    }
    // the threshold is relative to the tallest candidate: a huge solvent peak must not hide the real ones
    let top = cands.iter().map(|p| p.height).fold(0.0, f64::max);
    let mut out: Vec<Label> = cands
        .iter()
        .filter(|p| p.snr >= snr_min && p.height >= rel_min * top)
        .take(max_n)
        .map(|p| Label {
            apex_i: p.apex_i,
            rt: rt[p.apex_i],
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
