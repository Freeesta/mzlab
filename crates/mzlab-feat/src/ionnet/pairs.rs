//! Correlation of two rows of a per-ion table (the metaCorrelate idea of mzmine and the `groupFWHM`/`groupCorr` of CAMERA, Kuhl et al. 2011,
//! doi:10.1021/ac202450g), from the table alone: same RT, apexes within a share of the peak width in the files that detect both, and heights that
//! rise and fall together over the files (Pearson).

use super::Params;
use crate::table::{Aligned, CELL_DETECTED, CELL_FILLED, CELL_TARGETED};

/// Evidence that two rows are one compound: `r` = Pearson of the heights (NaN when not tested: too few shared files or no spread).
#[derive(Debug, Clone, Copy)]
pub struct Corr {
    pub a: usize,
    pub b: usize,
    pub r: f64,
}

fn measured(t: &Aligned, c: usize) -> bool {
    matches!(t.state[c], CELL_DETECTED | CELL_TARGETED | CELL_FILLED) && t.height[c] > 0.0
}

/// Pearson of `x` and `y`; NaN when one of them has no spread.
fn pearson(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len() as f64;
    let (mx, my) = (x.iter().sum::<f64>() / n, y.iter().sum::<f64>() / n);
    let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
    for (a, b) in x.iter().zip(y) {
        sxy += (a - mx) * (b - my);
        sxx += (a - mx) * (a - mx);
        syy += (b - my) * (b - my);
    }
    // a spread below 1e-6 of the mean is a constant level: no evidence either way
    if sxx <= (1e-6 * mx).powi(2) * n || syy <= (1e-6 * my).powi(2) * n {
        return f64::NAN;
    }
    sxy / (sxx * syy).sqrt()
}

/// The evidence of rows `a` and `b`, or `None` when they are not one compound.
pub fn corr(t: &Aligned, p: &Params, a: usize, b: usize) -> Option<Corr> {
    if (t.rt[a] - t.rt[b]).abs() > p.rt_tol {
        return None;
    }
    let nf = t.n_files;
    let (mut ha, mut hb) = (Vec::new(), Vec::new());
    let (mut both, mut agree) = (0usize, 0usize);
    for f in 0..nf {
        let (ca, cb) = (a * nf + f, b * nf + f);
        if !(measured(t, ca) && measured(t, cb)) {
            continue;
        }
        ha.push(f64::from(t.height[ca]));
        hb.push(f64::from(t.height[cb]));
        if t.state[ca] == CELL_DETECTED && t.state[cb] == CELL_DETECTED {
            both += 1;
            let w = f64::from(t.width[ca].max(t.width[cb]));
            let d = f64::from((t.apex_rt[ca] - t.apex_rt[cb]).abs());
            agree += usize::from(d <= p.apex_fwhm * w + 1e-9);
        }
    }
    if ha.is_empty() || (both > 0 && 2 * agree < both) {
        return None;
    }
    let r = if ha.len() >= p.min_corr_samples {
        pearson(&ha, &hb)
    } else {
        f64::NAN
    };
    if r < p.min_corr {
        return None;
    }
    Some(Corr { a, b, r })
}

/// Every pair of rows that correlates, `a < b`, in row order (rows visited by RT, so the cost is the RT window and not all pairs).
pub fn all(t: &Aligned, p: &Params) -> Vec<Corr> {
    let order = crate::sortc::order(t.len(), &mut |i, j| {
        crate::sortc::asc_nan_last(t.rt[i], t.rt[j]).then(i.cmp(&j))
    });
    let mut out = Vec::new();
    for (k, &i) in order.iter().enumerate() {
        let i = i as usize;
        for &j in &order[k + 1..] {
            let j = j as usize;
            if t.rt[j] - t.rt[i] > p.rt_tol {
                break;
            }
            if let Some(c) = corr(t, p, i.min(j), i.max(j)) {
                out.push(c);
            }
        }
    }
    crate::sortc::by(&mut out, |x, y| x.a.cmp(&y.a).then(x.b.cmp(&y.b)));
    out
}
