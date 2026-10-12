//! Shoulders (conservative): in a region with a single apex, the minimum of the second derivative at least 0.6 w from the apex marks a candidate; the
//! main Gaussian (width w) is subtracted and, if the residual stands out (Z >= k + 1) and holds at least 10 % of the area of the peak, it is a second
//! centroid flagged as a shoulder. One per side, no iteration. Below 0.6 w nothing is separated.

use super::detect::Group;
use super::{Buf, Centroid, Mode, Params, SHOULDER};

/// Second derivative of the raw signal at `j` (quadratic fit over `l` points): c_k = 2 (k^2 - mu) / sum (k^2 - mu)^2.
fn second(buf: &Buf, l: usize, j: usize) -> f64 {
    let m = ((l - 1) / 2) as f64;
    let mu = m * (m + 1.0) / 3.0;
    let den: f64 = (0..l).map(|i| ((i as f64 - m).powi(2) - mu).powi(2)).sum();
    let mut acc = 0.0;
    for i in 0..l {
        let t = j as isize + i as isize - (l as isize - 1) / 2;
        if t >= 0 && (t as usize) < buf.v.len() {
            acc += 2.0 * ((i as f64 - m).powi(2) - mu) / den * buf.v[t as usize];
        }
    }
    acc
}

/// The residual window around `j` after the main Gaussian: (positive residual, its weighted m/z, highest value, background sum).
fn residual(buf: &Buf, p: &Params, j: usize, x0: f64, h: f64) -> Option<(f64, f64, f64, f64)> {
    let hw = (0.5 * p.wb() as f64).ceil() as usize;
    let (lo, hi) = (j.saturating_sub(hw), (j + hw).min(buf.v.len() - 1));
    let sg = p.w / 2.355;
    let (mut a, mut am, mut top, mut f) = (0.0, 0.0, 0.0f64, 0.0);
    for i in lo..=hi {
        let r = buf.v[i] - buf.b[i] - h * (-(buf.x[i] - x0).powi(2) / (2.0 * sg * sg)).exp();
        if r > 0.0 {
            a += r;
            am += r * buf.x[i];
            top = top.max(r);
        }
        f += buf.b[i];
    }
    (a > 0.0).then(|| (a, am / a, top, f))
}

/// The shoulders of the peak of group `g` (at most one on each side).
pub(super) fn find(buf: &Buf, p: &Params, sigma: f64, g: &Group, x0: f64, area: f64) -> Vec<Centroid> {
    let l = p.sg_len();
    let dist = (0.6 * p.wb() as f64).ceil() as usize;
    let h = buf.s[g.apex] - buf.b[g.apex];
    let mut out = Vec::new();
    for side in [-1isize, 1] {
        let mut best: Option<(f64, usize)> = None;
        let mut d = dist;
        loop {
            let j = g.apex as isize + side * d as isize;
            if j < g.lo as isize || j > g.hi as isize {
                break;
            }
            let v = second(buf, l, j as usize);
            if v < 0.0 && best.is_none_or(|(b, _)| v < b) {
                best = Some((v, j as usize));
            }
            d += 1;
        }
        let Some((_, j)) = best else { continue };
        let Some((a, mz, top, f)) = residual(buf, p, j, x0, h) else { continue };
        let ok = match p.mode {
            Mode::Counts => {
                let (ac, fc) = (a / p.unit, f / p.unit);
                ac / (ac + 2.0 * fc).sqrt() >= p.k + 1.0
            }
            Mode::Continuous => top >= (p.k + 1.0) * sigma,
        };
        if ok && a >= 0.10 * area && (mz - x0).abs() >= 0.5 * p.w {
            out.push(Centroid { mz, value: if p.area { a } else { top }, area: a, height: top, flags: SHOULDER });
        }
    }
    out
}
