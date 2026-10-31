//! Multiscale Ricker response of `broad.rs`: the taps of each scale tabulated once (`Kernel`) and the candidate maxima of the response (`candidates`).

use crate::sortc::SortC;
use mzlab_core::error::Result;

use crate::centwave::ricker;
use crate::trace::{filled, reserve};

/// The taps of the Ricker wavelet of standard deviation `s` samples, computed once per scale: `w[k]` is `ricker(k / s)`, which is even, so one side serves both.
pub(super) struct Kernel {
    w: Vec<f64>,
    sqrt_s: f64,
}

impl Kernel {
    pub(super) fn new(s: f64) -> Result<Kernel> {
        let half = libm::ceil(4.0 * s) as usize;
        let mut w = Vec::new();
        reserve(&mut w, half + 1)?;
        w.extend((0..=half).map(|k| ricker(k as f64 / s)));
        Ok(Kernel {
            w,
            sqrt_s: libm::sqrt(s),
        })
    }

    /// Response of the wavelet centred on sample `t` (taps outside the series are dropped). The taps are summed from the left end to the right one, as ever.
    pub(super) fn at(&self, y: &[f64], t: usize) -> f64 {
        let half = self.w.len() - 1;
        let (i0, i1) = (t.saturating_sub(half), (t + half + 1).min(y.len()));
        let mut acc = 0.0;
        for (v, w) in y[i0..t].iter().zip(self.w[1..=t - i0].iter().rev()) {
            acc += v * w;
        }
        acc += y[t] * self.w[0];
        for (v, w) in y[t + 1..i1].iter().zip(&self.w[1..]) {
            acc += v * w;
        }
        acc / self.sqrt_s
    }
}

/// One candidate: the sample, the scale (index) and the response that found it.
pub(super) struct Cand {
    pub(super) t: usize,
    pub(super) s: usize,
    pub(super) r: f64,
}

/// The candidates: local maxima of the response at every scale, strongest first kept when two lie within the larger standard deviation.
pub(super) fn candidates(y: &[f64], sig: &[f64]) -> Result<Vec<Cand>> {
    let n = y.len();
    let mut all: Vec<Cand> = Vec::new();
    let mut r = filled(n, 0.0f64)?;
    for (si, &s) in sig.iter().enumerate() {
        let kernel = Kernel::new(s)?;
        for (t, out) in r.iter_mut().enumerate() {
            *out = kernel.at(y, t);
        }
        for (t, &v) in r.iter().enumerate() {
            let left = t == 0 || v >= r[t - 1];
            let right = t + 1 == n || v > r[t + 1];
            if v > 0.0 && left && right {
                reserve(&mut all, 1)?;
                all.push(Cand { t, s: si, r: v });
            }
        }
    }
    all.sortc_by(|a, b| b.r.total_cmp(&a.r).then(a.t.cmp(&b.t)));
    let mut kept: Vec<Cand> = Vec::new();
    for c in all {
        let near = kept
            .iter()
            .any(|k| (k.t as f64 - c.t as f64).abs() <= sig[k.s].max(sig[c.s]).ceil());
        if !near {
            reserve(&mut kept, 1)?;
            kept.push(c);
        }
    }
    kept.sortc_by_key(|c| c.t);
    Ok(kept)
}
