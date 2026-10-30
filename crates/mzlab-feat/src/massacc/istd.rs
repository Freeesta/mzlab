//! What the trace of an internal standard (or a standard of the mix) says besides its mass error: where it elutes, how much of it there is, how wide the peak is. Numbers only; the
//! batch part (the spread of the area over the files) is `area_rsd`.

use super::measure::Trace;

/// The retention and the size of one standard in one file.
#[derive(Clone, Debug, PartialEq)]
pub struct Istd {
    pub r: usize,
    /// RT of the most intense scan, minutes.
    pub rt: f64,
    /// Peak area of the summed window over the scans above the intensity floor, counts x seconds (trapezoids).
    pub area: f64,
    /// Full width at half maximum, minutes; `None` when the trace does not fall to half on both sides.
    pub fwhm: Option<f64>,
}

/// The RT where the line through two points (rt, y) reaches `level`.
fn cross(a: (f64, f64), b: (f64, f64), level: f64) -> f64 {
    if (b.1 - a.1).abs() < f64::MIN_POSITIVE {
        return a.0;
    }
    a.0 + (level - a.1) / (b.1 - a.1) * (b.0 - a.0)
}

pub fn of_trace(t: &Trace) -> Option<Istd> {
    let top = (0..t.sum.len()).max_by(|&i, &j| t.sum[i].total_cmp(&t.sum[j]).then(j.cmp(&i)))?;
    let area =
        t.rt.windows(2)
            .zip(t.sum.windows(2))
            .map(|(x, y)| (x[1] - x[0]) * 60.0 * (y[0] + y[1]) / 2.0)
            .sum();
    let half = t.sum[top] / 2.0;
    let left = (0..top)
        .rev()
        .find(|&i| t.sum[i] <= half)
        .map(|i| cross((t.rt[i], t.sum[i]), (t.rt[i + 1], t.sum[i + 1]), half));
    let right = (top + 1..t.sum.len())
        .find(|&i| t.sum[i] <= half)
        .map(|i| cross((t.rt[i - 1], t.sum[i - 1]), (t.rt[i], t.sum[i]), half));
    Some(Istd {
        r: t.r,
        rt: t.rt[top],
        area,
        fwhm: left.zip(right).map(|(a, b)| b - a),
    })
}

/// Relative standard deviation (sample) of `v`; `None` below three values or with a zero mean.
pub fn area_rsd(v: &[f64]) -> Option<f64> {
    let n = v.len();
    let mean = v.iter().sum::<f64>() / n.max(1) as f64;
    if n < 3 || mean <= 0.0 {
        return None;
    }
    let var = v.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / (n - 1) as f64;
    Some(var.sqrt() / mean)
}
