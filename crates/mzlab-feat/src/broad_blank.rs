//! Pointwise blank subtraction for the broad-band pass: `BlankBank` holds the ion traces of the blank run decimated like the sample, `subtract`
//! removes the blank trace of the same ion from the sample trace before the wavelet. Used by `broad::detect_broad_at`; the bank is filled by
//! `trace::blank_bank`. Sums run in a fixed order: native and WASM give the same bits.

use mzlab_core::error::Result;

use crate::broad::decimate;
use crate::broad_gt::median;
use crate::trace::{filled, reserve};

/// Most traces the bank keeps (the strongest ones).
const MAX_ITEMS: usize = 4096;

/// One ion trace of the blank, decimated: block RT, block intensity and the m/z of the trace.
#[derive(Clone, PartialEq)]
struct Item {
    mz: f64,
    rt: Vec<f64>,
    y: Vec<f64>,
    top: f64,
}

/// Ion traces of the blank run, ordered by m/z once `finish` has run.
#[derive(Clone, Default, PartialEq)]
pub struct BlankBank {
    items: Vec<Item>,
}

impl std::fmt::Debug for BlankBank {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BlankBank({})", self.items.len())
    }
}

impl BlankBank {
    /// Adds the trace `y` (seconds `rt_s`, centre `mz`) decimated by `k`; keeps the strongest `MAX_ITEMS` traces.
    pub fn add(&mut self, mz: f64, rt_s: &[f64], y: &[f64], k: usize) -> Result<()> {
        let flat = filled(y.len(), mz)?;
        let (rt, y, _) = decimate(rt_s, &flat, y, k)?;
        let top = y.iter().copied().fold(0.0f64, f64::max);
        reserve(&mut self.items, 1)?;
        self.items.push(Item { mz, rt, y, top });
        if self.items.len() >= 2 * MAX_ITEMS {
            self.trim();
        }
        Ok(())
    }

    fn trim(&mut self) {
        self.items
            .sort_by(|a, b| b.top.total_cmp(&a.top).then(a.mz.total_cmp(&b.mz)));
        self.items.truncate(MAX_ITEMS);
    }

    /// Orders the traces by m/z; `None` when the blank gave none.
    pub fn finish(mut self) -> Option<Self> {
        self.trim();
        self.items
            .sort_by(|a, b| a.mz.total_cmp(&b.mz).then(b.top.total_cmp(&a.top)));
        (!self.items.is_empty()).then_some(self)
    }

    /// The trace whose m/z is nearest to `mz` within `ppm`.
    fn near(&self, mz: f64, ppm: f64) -> Option<&Item> {
        let tol = mz * ppm * 1e-6;
        let from = self.items.partition_point(|i| i.mz < mz - tol);
        let to = self.items.partition_point(|i| i.mz <= mz + tol);
        self.items[from..to]
            .iter()
            .min_by(|a, b| (a.mz - mz).abs().total_cmp(&(b.mz - mz).abs()))
    }
}

/// Linear interpolation of the decimated blank at `t`; 0 outside the run of the blank.
fn at(it: &Item, t: f64) -> f64 {
    let n = it.rt.len();
    if n == 0 || t < it.rt[0] || t > it.rt[n - 1] {
        return 0.0;
    }
    let j = it.rt.partition_point(|&r| r <= t).clamp(1, n - 1);
    if n == 1 {
        return it.y[0];
    }
    let (t0, t1) = (it.rt[j - 1], it.rt[j]);
    if t1 <= t0 {
        return it.y[j];
    }
    it.y[j - 1] + (it.y[j] - it.y[j - 1]) * (t - t0) / (t1 - t0)
}

/// `y -= scale x blank` (clipped at 0), where the blank trace is that of the ion at m/z `mz` interpolated at the time `t0 + rt_s[i]`. With `fit`
/// the scale is the median of the sample/blank ratios over the decimated blocks where both are positive, kept in [0, 1]; otherwise 1. Without a
/// blank trace of that ion `y` is left as it was.
pub fn subtract(
    y: &mut [f64],
    rt_s: &[f64],
    t0: f64,
    mz: f64,
    k: usize,
    (bank, fit, ppm): (&BlankBank, bool, f64),
) -> Result<()> {
    let Some(it) = bank.near(mz, ppm) else {
        return Ok(());
    };
    let mut b = filled(y.len(), 0.0f64)?;
    for (o, &t) in b.iter_mut().zip(rt_s) {
        *o = at(it, t0 + t);
    }
    let mut scale = 1.0;
    if fit {
        let flat = filled(y.len(), mz)?;
        let (_, yd, _) = decimate(rt_s, &flat, y, k)?;
        let (_, bd, _) = decimate(rt_s, &flat, &b, k)?;
        let mut r = Vec::new();
        for (&s, &c) in yd.iter().zip(&bd) {
            if s > 0.0 && c > 0.0 {
                reserve(&mut r, 1)?;
                r.push(s / c);
            }
        }
        if !r.is_empty() {
            scale = median(&mut r).clamp(0.0, 1.0);
        }
    }
    for (v, &c) in y.iter_mut().zip(&b) {
        *v = (*v - scale * c).max(0.0);
    }
    Ok(())
}
