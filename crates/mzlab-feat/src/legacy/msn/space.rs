//! `hr/formula.py` beyond what `fragtree::Space` needs: candidates and counts under a sub-formula (`within`), and the m/z calibration
//! (`Calibration`, `fit_calibration`).

use crate::legacy::fragtree::Space;
use crate::sortc::SortC;

/// `FormulaSpace.window` of one m/z: (start, stop) of the sorted formulas within max(ppm, abs_da).
fn window(sp: &Space, mz: f64, ppm: f64, abs_da: f64) -> (usize, usize) {
    let tol = (mz * ppm * 1e-6).max(abs_da);
    let a = sp.mass.partition_point(|&x| x < mz - tol);
    let b = sp.mass.partition_point(|&x| x <= mz + tol);
    (a, b.max(a))
}

fn inside(g: &[i64], within: Option<&[i64]>) -> bool {
    within.is_none_or(|w| g.iter().zip(w).all(|(a, b)| a <= b))
}

/// `FormulaSpace.candidates(mz, ppm, abs_da, within)`: smallest |error| first.
pub fn candidates(
    sp: &Space,
    mz: f64,
    ppm: f64,
    abs_da: f64,
    within: Option<&[i64]>,
) -> Vec<usize> {
    let (a, b) = window(sp, mz, ppm, abs_da);
    let mut idx: Vec<usize> = (a..b).filter(|&i| inside(&sp.g[i], within)).collect();
    idx.sortc_by(|&x, &y| (sp.mass[x] - mz).abs().total_cmp(&(sp.mass[y] - mz).abs()));
    idx
}

/// `FormulaSpace.count` of one m/z.
pub fn count(sp: &Space, mz: f64, ppm: f64, abs_da: f64, within: Option<&[i64]>) -> usize {
    let (a, b) = window(sp, mz, ppm, abs_da);
    (a..b).filter(|&i| inside(&sp.g[i], within)).count()
}

/// `Calibration`: err(mz) = a + b (mz - mz0) in ppm; `apply` removes it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Calibration {
    pub a: f64,
    pub b: f64,
    pub mz0: f64,
    pub n: usize,
}

impl Default for Calibration {
    fn default() -> Self {
        Calibration {
            a: 0.0,
            b: 0.0,
            mz0: 200.0,
            n: 0,
        }
    }
}

impl Calibration {
    pub fn apply(&self, mz: f64) -> f64 {
        mz / (1.0 + (self.a + self.b * (mz - self.mz0)) * 1e-6)
    }
}

/// `np.median`.
fn median(v: &[f64]) -> f64 {
    let mut s = v.to_vec();
    s.sortc_by(f64::total_cmp);
    let n = s.len();
    if n == 0 {
        return f64::NAN;
    }
    if n % 2 == 1 {
        s[n / 2]
    } else {
        (s[n / 2 - 1] + s[n / 2]) / 2.0
    }
}

/// `np.linalg.lstsq` of y = c0 + c1 x on the points kept.
fn line(x: &[f64], y: &[f64], keep: &[bool]) -> (f64, f64) {
    let pts: Vec<(f64, f64)> = (0..x.len())
        .filter(|&i| keep[i])
        .map(|i| (x[i], y[i]))
        .collect();
    let n = pts.len() as f64;
    let xm = pts.iter().map(|p| p.0).sum::<f64>() / n;
    let ym = pts.iter().map(|p| p.1).sum::<f64>() / n;
    let sxx: f64 = pts.iter().map(|p| (p.0 - xm) * (p.0 - xm)).sum();
    let sxy: f64 = pts.iter().map(|p| (p.0 - xm) * (p.1 - ym)).sum();
    let c1 = if sxx > 0.0 { sxy / sxx } else { 0.0 };
    (ym - c1 * xm, c1)
}

/// `fit_calibration(mz, space, within, window_ppm, min_points=8, mode)`: the median ppm error of the peaks with exactly one formula, or a line of
/// m/z when it removes more than 20 % of the spread (two robust passes).
pub fn fit(
    mz: &[f64],
    sp: &Space,
    within: Option<&[i64]>,
    window_ppm: f64,
    mode: &str,
) -> Calibration {
    const MIN_POINTS: usize = 8;
    let sel: Vec<f64> = mz
        .iter()
        .copied()
        .filter(|&m| count(sp, m, window_ppm, 0.0, within) == 1)
        .collect();
    if sel.is_empty() {
        return Calibration::default();
    }
    let err: Vec<f64> = sel
        .iter()
        .map(|&m| {
            let c = candidates(sp, m, window_ppm, 0.0, within)[0];
            (m - sp.mass[c]) / sp.mass[c] * 1e6
        })
        .collect();
    let mz0 = median(&sel);
    let cst = Calibration {
        a: median(&err),
        b: 0.0,
        mz0,
        n: sel.len(),
    };
    let ptp = sel.iter().copied().fold(f64::NEG_INFINITY, f64::max)
        - sel.iter().copied().fold(f64::INFINITY, f64::min);
    if mode == "constant" || sel.len() < MIN_POINTS || ptp < 60.0 {
        return cst;
    }
    let x: Vec<f64> = sel.iter().map(|m| m - mz0).collect();
    let mut keep = vec![true; x.len()];
    let mut coef = (0.0, 0.0);
    for _ in 0..2 {
        coef = line(&x, &err, &keep);
        let res: Vec<f64> = x
            .iter()
            .zip(&err)
            .map(|(xi, yi)| yi - (coef.0 + coef.1 * xi))
            .collect();
        let kept: Vec<f64> = (0..res.len())
            .filter(|&i| keep[i])
            .map(|i| res[i])
            .collect();
        let med = median(&kept);
        let dev: Vec<f64> = kept.iter().map(|r| (r - med).abs()).collect();
        let mad = 1.4826 * median(&dev) + 1e-9;
        keep = res.iter().map(|r| r.abs() <= 3.0 * mad).collect();
        if keep.iter().filter(|&&k| k).count() < MIN_POINTS {
            return cst;
        }
    }
    let lin = Calibration {
        a: coef.0,
        b: coef.1,
        mz0,
        n: keep.iter().filter(|&&k| k).count(),
    };
    let sc: Vec<f64> = err.iter().map(|y| (y - cst.a).abs()).collect();
    let sl: Vec<f64> = x
        .iter()
        .zip(&err)
        .map(|(xi, yi)| (yi - (coef.0 + coef.1 * xi)).abs())
        .collect();
    if mode == "linear" || median(&sl) < 0.8 * median(&sc) {
        lin
    } else {
        cst
    }
}
