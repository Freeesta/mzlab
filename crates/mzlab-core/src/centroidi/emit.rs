//! From regions to centroids: the threshold of the mode, the m/z, the value, the flags.

use super::detect::Group;
use super::shoulder;
use super::{Buf, Centroid, Mode, Params, ASYMMETRIC, SATURATED, TRUNCATED};

/// Accepts a region: counts mode needs Z = (A - F) / sqrt(A + F) >= k, at least 3 counts above the background and two non-zero bins; continuous mode
/// needs the smoothed apex k sigma over the background.
fn passes(buf: &Buf, p: &Params, sigma: f64, g: &Group) -> bool {
    let (area, back) = sums(buf, p, g);
    let nz = buf.v[g.lo..=g.hi].iter().filter(|&&v| v > 0.0).count();
    match p.mode {
        Mode::Counts => {
            let z = if area + back > 0.0 {
                (area - back) / (area + back).sqrt()
            } else {
                0.0
            };
            z >= p.k && area - back >= 3.0 && nz >= 2
        }
        Mode::Continuous => buf.s[g.apex] - buf.b[g.apex] >= p.k * sigma,
    }
}

/// Raw sum and background sum of the region (counts mode: in counts, otherwise in intensity).
fn sums(buf: &Buf, p: &Params, g: &Group) -> (f64, f64) {
    let u = if p.mode == Mode::Counts { p.unit } else { 1.0 };
    (
        buf.v[g.lo..=g.hi].iter().sum::<f64>() / u,
        buf.b[g.lo..=g.hi].iter().sum::<f64>() / u,
    )
}

/// Apex by a parabola through the three smoothed points around the maximum.
pub(super) fn apex_x(buf: &Buf, j: usize) -> f64 {
    if j == 0 || j + 1 >= buf.s.len() {
        return buf.x[j];
    }
    let den = buf.s[j - 1] - 2.0 * buf.s[j] + buf.s[j + 1];
    if den >= 0.0 {
        return buf.x[j];
    }
    let d = (0.5 * (buf.s[j - 1] - buf.s[j + 1]) / den).clamp(-1.0, 1.0);
    buf.x[j]
        + d * if d > 0.0 {
            buf.x[j + 1] - buf.x[j]
        } else {
            buf.x[j] - buf.x[j - 1]
        }
}

/// Weighted centroid of (y - background)+ over the symmetric window [c - w/2, c + w/2], bins cut by the border count in proportion; `skip` leaves out
/// the bins at the saturation level.
pub(super) fn centroid_at(
    buf: &Buf,
    p: &Params,
    c: f64,
    skip: Option<f64>,
    half: f64,
) -> Option<f64> {
    let (lo, hi) = (c - half, c + half);
    let (mut sw, mut swm) = (0.0, 0.0);
    for i in 0..buf.x.len() {
        let (a, b) = (buf.x[i] - 0.5 * p.step, buf.x[i] + 0.5 * p.step);
        let frac = ((b.min(hi) - a.max(lo)) / p.step).clamp(0.0, 1.0);
        if frac <= 0.0 || skip.is_some_and(|t| buf.v[i] >= t) {
            continue;
        }
        let wt = (buf.v[i] - buf.b[i]).max(0.0) * frac;
        sw += wt;
        swm += wt * buf.x[i];
    }
    (sw > 0.0).then(|| swm / sw)
}

/// Gaussian interpolation through the three raw points around the highest one (high resolution), when all three are positive and S/N >= 10.
fn gaussian(buf: &Buf, sigma: f64, apex: usize) -> Option<f64> {
    let m = (apex.saturating_sub(1)..=(apex + 1).min(buf.v.len() - 1)).fold(apex, |m, i| {
        if buf.v[i] > buf.v[m] {
            i
        } else {
            m
        }
    });
    if m == 0
        || m + 1 >= buf.v.len()
        || buf.v[m - 1] <= 0.0
        || buf.v[m] <= 0.0
        || buf.v[m + 1] <= 0.0
        || (buf.v[m] - buf.b[m]) / sigma < 10.0
    {
        return None;
    }
    let (a, b, c) = (buf.v[m - 1].ln(), buf.v[m].ln(), buf.v[m + 1].ln());
    let den = a - 2.0 * b + c;
    if den >= 0.0 {
        return None;
    }
    let d = 0.5 * (a - c) / den;
    Some(
        buf.x[m]
            + d * if d > 0.0 {
                buf.x[m + 1] - buf.x[m]
            } else {
                buf.x[m] - buf.x[m - 1]
            },
    )
}

/// True when three or more consecutive raw bins of the region are within 1 % of the highest intensity of the file.
fn saturated(buf: &Buf, p: &Params, g: &Group) -> bool {
    if p.vmax <= 0.0 {
        return false;
    }
    let t = 0.99 * p.vmax;
    let mut run = 0;
    for j in g.lo..=g.hi {
        run = if buf.v[j] >= t { run + 1 } else { 0 };
        if run >= 3 {
            return true;
        }
    }
    false
}

/// Centre of a saturated peak: the middle of the flat top, then the weighted centroid of the unsaturated bins in a window as wide as the flat top plus w
/// (symmetric around the centre, repeated twice).
fn saturated_centre(buf: &Buf, p: &Params, g: &Group) -> Option<f64> {
    let t = 0.99 * p.vmax;
    let top: Vec<usize> = (g.lo..=g.hi).filter(|&j| buf.v[j] >= t).collect();
    let (a, b) = (*top.first()?, *top.last()?);
    let half = 0.5 * (buf.x[b] - buf.x[a]) + 0.5 * p.w;
    let mut c = 0.5 * (buf.x[a] + buf.x[b]);
    for _ in 0..2 {
        c = centroid_at(buf, p, c, Some(t), half).unwrap_or(c);
    }
    Some(c)
}

fn one(buf: &Buf, p: &Params, sigma: f64, g: &Group, out: &mut Vec<Centroid>) {
    if !passes(buf, p, sigma, g) {
        return;
    }
    let nz = buf.v[g.lo..=g.hi].iter().filter(|&&v| v > 0.0).count();
    let truncated = g.lo == 0 || g.hi + 1 == buf.v.len();
    if truncated && nz < 3 {
        return;
    }
    let mut flags = if truncated { TRUNCATED } else { 0 };
    let sat = saturated(buf, p, g);
    let x0 = apex_x(buf, g.apex);
    let mz = if sat {
        flags |= SATURATED;
        saturated_centre(buf, p, g).unwrap_or(x0)
    } else if p.hr() {
        gaussian(buf, sigma, g.apex)
            .or_else(|| centroid_at(buf, p, x0, None, 0.5 * p.w))
            .unwrap_or(x0)
    } else {
        centroid_at(buf, p, x0, None, 0.5 * p.w).unwrap_or(x0)
    };
    if (mz - x0).abs() > 0.25 * p.w {
        flags |= ASYMMETRIC;
    }
    let mut area: f64 = buf.v[g.lo..=g.hi].iter().sum();
    let height = buf.s[g.apex];
    let mut shoulders = Vec::new();
    if g.members == 1 && !sat {
        shoulders = shoulder::find(buf, p, sigma, g, x0, area);
    }
    area -= shoulders.iter().map(|s| s.area).sum::<f64>();
    out.push(Centroid {
        mz,
        value: if p.area || sat { area } else { height },
        area,
        height,
        flags,
    });
    out.extend(shoulders);
}

pub(super) fn emit(buf: &Buf, p: &Params, sigma: f64, groups: &[Group]) -> Vec<Centroid> {
    let mut out = Vec::new();
    for g in groups {
        one(buf, p, sigma, g, &mut out);
    }
    out
}
