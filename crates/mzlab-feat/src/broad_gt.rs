//! Common gradient mode G(t) of a file for the broad-band pass: the gradient drift shared by many ion traces reads as a wide band on each of them.
//! `TraceBank` keeps a uniform sample of the persistent traces, `CommonMode` is their pointwise median per block, and `fit_background` removes
//! [1, t, t^2, G(t)] from one trace by asymmetric least squares. Used by `broad::detect_broad` and by the collectors of `trace` and `untargeted`.
//! Sums run in a fixed order and no transcendental is used: native and WASM give the same bits.

use crate::sortc::SortC;
use std::fmt;
use std::sync::Arc;

use mzlab_core::error::Result;

use crate::broad::decimate;
use crate::trace::{filled, reserve};

/// A trace is persistent when it has at least this many scans or covers this fraction of the run.
const MIN_SCANS: usize = 400;
const MIN_COVER: f64 = 0.3;
/// Most traces kept by the bank (a uniform sample of the persistent ones).
const MAX_TRACES: usize = 512;
/// Weight of the positive and of the negative residuals of the asymmetric fit.
const W_POS: f64 = 0.05;
const W_NEG: f64 = 0.95;
const ITERATIONS: usize = 8;
/// Fewer traces than this on a block give no common mode there.
const MIN_ROWS: usize = 3;
/// Blocks of the running median that cleans G of narrow peaks.
const SMOOTH: usize = 9;

/// G(t) on every scan of the run (interpolated from the blocks), unit median of the normalised traces.
#[derive(Clone, PartialEq)]
pub struct CommonMode {
    /// One value per scan of the run.
    pub g: Vec<f64>,
}

impl fmt::Debug for CommonMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CommonMode({})", self.g.len())
    }
}

/// Persistent traces decimated by `k`, normalised on the fly; `add` keeps every `stride`-th and doubles the stride when full.
pub struct TraceBank {
    n_scans: usize,
    k: usize,
    rows: Vec<Vec<f32>>,
    seen: usize,
    stride: usize,
}

impl TraceBank {
    pub fn new(n_scans: usize, k: usize) -> Self {
        TraceBank {
            n_scans,
            k: k.max(1),
            rows: Vec::new(),
            seen: 0,
            stride: 1,
        }
    }

    /// Offers the trace `y` starting at scan `scan0`; only persistent ones (see `MIN_SCANS`) count. Missing scans of the run are `NaN` blocks.
    pub fn add(&mut self, scan0: usize, y: &[f64]) -> Result<()> {
        if self.offer(scan0, y.len()) {
            self.push_row(scan0, y)?;
        }
        Ok(())
    }

    /// Counts a trace of `len` scans from `scan0` as `add` would: true when the caller must build it and hand it to [`push_row`](Self::push_row), false when
    /// the bank leaves it out (not persistent, outside the run, or skipped by the stride), so the caller can spare the work of building it.
    pub fn offer(&mut self, scan0: usize, len: usize) -> bool {
        if len < MIN_SCANS && (len as f64) < MIN_COVER * self.n_scans as f64 {
            return false;
        }
        if scan0 + len > self.n_scans {
            return false;
        }
        self.seen += 1;
        (self.seen - 1).is_multiple_of(self.stride)
    }

    /// Stores the trace that [`offer`](Self::offer) asked for.
    pub fn push_row(&mut self, scan0: usize, y: &[f64]) -> Result<()> {
        let Some(row) = normalised_row(scan0, y, self.n_scans, self.k)? else {
            return Ok(());
        };
        if self.rows.len() == MAX_TRACES {
            let mut keep = 0usize;
            self.rows.retain(|_| {
                keep += 1;
                !keep.is_multiple_of(2)
            });
            self.stride *= 2;
        }
        reserve(&mut self.rows, 1)?;
        self.rows.push(row);
        Ok(())
    }

    /// G per block = pointwise median of the normalised rows, then linear interpolation to every scan. `None` without enough traces.
    pub fn finish(&self) -> Result<Option<CommonMode>> {
        if self.rows.len() < MIN_ROWS {
            return Ok(None);
        }
        let gb = block_medians(&self.rows, self.n_scans.div_ceil(self.k))?;
        // a narrow peak that the traces happen to share must not enter G: running median over `SMOOTH` blocks
        let gb = smooth(&gb)?;
        interpolate(&gb, self.k, self.n_scans)
    }
}

/// The pointwise median of the rows per block (`NaN` where fewer than `MIN_ROWS` rows have a value).
fn block_medians(rows: &[Vec<f32>], m: usize) -> Result<Vec<f64>> {
    let mut gb = filled(m, f64::NAN)?;
    let mut col = Vec::new();
    reserve(&mut col, rows.len())?;
    for (b, out) in gb.iter_mut().enumerate() {
        col.clear();
        col.extend(
            rows.iter()
                .map(|r| r[b])
                .filter(|v| !v.is_nan())
                .map(f64::from),
        );
        if col.len() >= MIN_ROWS {
            *out = median(&mut col);
        }
    }
    Ok(gb)
}

/// The trace `y` (from scan `scan0`) as block means over the run (`NaN` where it has no scan) divided by their median; `None` when that is not positive.
fn normalised_row(scan0: usize, y: &[f64], n_scans: usize, k: usize) -> Result<Option<Vec<f32>>> {
    let m = n_scans.div_ceil(k);
    let mut row = filled(m, f32::NAN)?;
    let mut cover = filled(m, 0usize)?;
    let mut acc = filled(m, 0.0f64)?;
    for (i, &v) in y.iter().enumerate() {
        let b = (scan0 + i) / k;
        acc[b] += v;
        cover[b] += 1;
    }
    let mut vals = Vec::new();
    reserve(&mut vals, m)?;
    for ((r, &c), &a) in row.iter_mut().zip(&cover).zip(&acc) {
        if c > 0 {
            let v = a / c as f64;
            *r = v as f32;
            vals.push(v);
        }
    }
    let scale = median(&mut vals);
    if scale.is_nan() || scale <= 0.0 {
        return Ok(None);
    }
    for v in row.iter_mut().filter(|v| !v.is_nan()) {
        *v = (*v as f64 / scale) as f32;
    }
    Ok(Some(row))
}

/// Linear interpolation of the block values `gb` (`NaN` = unknown) between the block centres to every scan; `None` with fewer than two known blocks.
fn interpolate(gb: &[f64], k: usize, n_scans: usize) -> Result<Option<CommonMode>> {
    let known: Vec<usize> = (0..gb.len()).filter(|&b| !gb[b].is_nan()).collect();
    if known.len() < 2 {
        return Ok(None);
    }
    let mut g = filled(n_scans, 0.0f64)?;
    let centre = |b: usize| (b * k) as f64 + (k as f64 - 1.0) / 2.0;
    for (i, out) in g.iter_mut().enumerate() {
        let at = known.partition_point(|&b| centre(b) <= i as f64);
        *out = match (at.checked_sub(1), known.get(at)) {
            (Some(l), Some(&r)) => {
                let (a, z) = (known[l], r);
                gb[a] + (gb[z] - gb[a]) * (i as f64 - centre(a)) / (centre(z) - centre(a))
            }
            (Some(l), None) => gb[known[l]],
            (None, Some(&r)) => gb[r],
            (None, None) => 0.0,
        };
    }
    Ok(Some(CommonMode { g }))
}

/// Running median of `SMOOTH` blocks (shorter at the ends, `NaN` blocks skipped and left `NaN`).
fn smooth(gb: &[f64]) -> Result<Vec<f64>> {
    let mut out = filled(gb.len(), f64::NAN)?;
    let mut win = Vec::new();
    reserve(&mut win, SMOOTH)?;
    for (i, o) in out.iter_mut().enumerate() {
        if gb[i].is_nan() {
            continue;
        }
        win.clear();
        let (a, z) = (
            i.saturating_sub(SMOOTH / 2),
            (i + SMOOTH / 2 + 1).min(gb.len()),
        );
        win.extend(gb[a..z].iter().copied().filter(|v| !v.is_nan()));
        *o = median(&mut win);
    }
    Ok(out)
}

/// Wraps a finished bank for `BroadParams::common`.
pub fn share(cm: Option<CommonMode>) -> Option<Arc<CommonMode>> {
    cm.map(Arc::new)
}

pub(crate) fn median(v: &mut [f64]) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sortc_by(|a, b| a.total_cmp(b));
    let m = v.len() / 2;
    if v.len() % 2 == 1 {
        v[m]
    } else {
        (v[m - 1] + v[m]) / 2.0
    }
}

/// Basis row [1, t, t^2, g] of a sample, `t` in [-1, 1] over the trace and `g` scaled to unit mean.
fn basis(t: f64, g: f64) -> [f64; 4] {
    [1.0, t, t * t, g]
}

/// Solves the 4x4 system `a x = b` (partial pivoting); `None` when singular.
fn solve4(mut a: [[f64; 4]; 4], mut b: [f64; 4]) -> Option<[f64; 4]> {
    for c in 0..4 {
        let p = (c..4).max_by(|&i, &j| a[i][c].abs().total_cmp(&a[j][c].abs()))?;
        if a[p][c].abs() < 1e-12 {
            return None;
        }
        a.swap(c, p);
        b.swap(c, p);
        for r in c + 1..4 {
            let f = a[r][c] / a[c][c];
            let pivot = a[c];
            for (v, pv) in a[r][c..].iter_mut().zip(&pivot[c..]) {
                *v -= f * pv;
            }
            b[r] -= f * b[c];
        }
    }
    let mut x = [0.0; 4];
    for c in (0..4).rev() {
        let tail: f64 = a[c][c + 1..]
            .iter()
            .zip(&x[c + 1..])
            .map(|(u, v)| u * v)
            .sum();
        x[c] = (b[c] - tail) / a[c][c];
    }
    Some(x)
}

/// The background of a trace: asymmetric least squares (weight 0.05 on positive residuals, 0.95 on negative ones, 8 iterations) on
/// [1, t, t^2, G(t)], fitted on the decimated series `yd` at times `td` (in [-1, 1], see `unit_time`) with `gd` (G on the same blocks). Returns the 4 coefficients, `None` when the system is singular.
pub fn fit_coefficients(td: &[f64], yd: &[f64], gd: &[f64]) -> Option<[f64; 4]> {
    let n = yd.len().min(gd.len()).min(td.len());
    if n < 8 {
        return None;
    }
    let gm = gd[..n].iter().sum::<f64>() / n as f64;
    if gm.is_nan() || gm <= 0.0 {
        return None;
    }
    let rows: Vec<[f64; 4]> = (0..n).map(|i| basis(td[i], gd[i] / gm)).collect();
    let mut w = vec![1.0f64; n];
    let mut x = [0.0f64; 4];
    for _ in 0..ITERATIONS {
        let mut a = [[0.0f64; 4]; 4];
        let mut b = [0.0f64; 4];
        for (i, r) in rows.iter().enumerate() {
            for (p, (bp, ap)) in b.iter_mut().zip(a.iter_mut()).enumerate() {
                *bp += w[i] * r[p] * yd[i];
                for (apq, rq) in ap.iter_mut().zip(r) {
                    *apq += w[i] * r[p] * rq;
                }
            }
        }
        x = solve4(a, b)?;
        for (i, r) in rows.iter().enumerate() {
            let fit: f64 = (0..4).map(|p| x[p] * r[p]).sum();
            w[i] = if yd[i] > fit { W_POS } else { W_NEG };
        }
    }
    // coefficient of G refers to g / mean: fold the scale back so that `background` takes the raw G
    x[3] /= gm;
    Some(x)
}

/// Background at the time `t` (in [-1, 1]) with raw G value `g`, for coefficients from `fit_coefficients`.
pub fn background(x: &[f64; 4], t: f64, g: f64) -> f64 {
    x[0] + x[1] * t + x[2] * t * t + x[3] * g
}

/// Maps the times `rt` onto [-1, 1] over `[first, last]` (a constant series maps to 0).
pub fn unit_time(rt: f64, first: f64, last: f64) -> f64 {
    if last > first {
        2.0 * (rt - first) / (last - first) - 1.0
    } else {
        0.0
    }
}

/// Subtracts the background fitted on the decimated `y_res` from every sample of it. `scan0` is the scan of the run that `y_res[0]` belongs to; a trace
/// that runs past the end of `cm`, or with a singular fit, is left as it is.
pub fn detrend(
    rt_s: &[f64],
    mz: &[f64],
    y_res: &mut [f64],
    k: usize,
    scan0: usize,
    cm: &CommonMode,
) -> Result<()> {
    let len = y_res.len();
    let Some(g) = cm.g.get(scan0..scan0 + len) else {
        return Ok(());
    };
    let (first, last) = (rt_s[0], rt_s[len - 1]);
    let (rd, yd, _) = decimate(rt_s, mz, y_res, k)?;
    let (_, gd, _) = decimate(rt_s, mz, g, k)?;
    let td: Vec<f64> = rd.iter().map(|&t| unit_time(t, first, last)).collect();
    let Some(x) = fit_coefficients(&td, &yd, &gd) else {
        return Ok(());
    };
    for ((v, &t), &gv) in y_res.iter_mut().zip(rt_s).zip(g) {
        *v -= background(&x, unit_time(t, first, last), gv);
    }
    Ok(())
}
