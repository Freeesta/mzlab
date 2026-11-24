//! The numbers behind one edge, read from the cells of the two rows: Pearson of the heights over the files that measure both, the intensity ratio of the
//! heavier to the lighter row per file (median and range), the RT offset of the rows and the median apex shift in the files that detect both.

use crate::ionnet::pairs::{measured, pearson};
use crate::table::{Aligned, CELL_DETECTED};

/// `r` is NaN when it was not tested (fewer shared files than `min_corr_samples`, or a constant level); `ratio_*` and `apex_shift` are NaN without data.
#[derive(Debug, Clone)]
pub struct Evidence {
    pub r: f64,
    pub n: usize,
    pub ratio_med: f64,
    pub ratio_lo: f64,
    pub ratio_hi: f64,
    pub rt_offset: f64,
    pub apex_shift: f64,
}

fn median(v: &mut [f64]) -> f64 {
    if v.is_empty() {
        return f64::NAN;
    }
    crate::sortc::by(v, |x, y| crate::sortc::asc_nan_last(*x, *y));
    let m = v.len() / 2;
    if v.len() % 2 == 1 {
        v[m]
    } else {
        (v[m - 1] + v[m]) / 2.0
    }
}

/// The evidence of the lighter row `a` and the heavier row `b`; the ratio is `b / a`.
pub fn of(t: &Aligned, a: usize, b: usize, min_n: usize) -> Evidence {
    let nf = t.n_files;
    let (mut ha, mut hb, mut ratio, mut apex) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for f in 0..nf {
        let (ca, cb) = (a * nf + f, b * nf + f);
        if !(measured(t, ca) && measured(t, cb)) {
            continue;
        }
        let (x, y) = (f64::from(t.height[ca]), f64::from(t.height[cb]));
        ha.push(x);
        hb.push(y);
        ratio.push(y / x);
        if t.state[ca] == CELL_DETECTED && t.state[cb] == CELL_DETECTED {
            apex.push(f64::from(t.apex_rt[cb]) - f64::from(t.apex_rt[ca]));
        }
    }
    let n = ha.len();
    let lo = ratio.iter().copied().fold(f64::NAN, f64::min);
    let hi = ratio.iter().copied().fold(f64::NAN, f64::max);
    Evidence {
        r: if n >= min_n {
            pearson(&ha, &hb)
        } else {
            f64::NAN
        },
        n,
        ratio_med: median(&mut ratio),
        ratio_lo: lo,
        ratio_hi: hi,
        rt_offset: t.rt[b] - t.rt[a],
        apex_shift: median(&mut apex),
    }
}

/// The sum of the heights of each row over the files that measure it: which of two rows is the more intense.
pub fn total_heights(t: &Aligned, a: usize, b: usize) -> (f64, f64) {
    let nf = t.n_files;
    let sum = |r: usize| {
        (0..nf)
            .filter(|f| measured(t, r * nf + f))
            .map(|f| f64::from(t.height[r * nf + f]))
            .sum::<f64>()
    };
    (sum(a), sum(b))
}
