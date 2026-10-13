//! Estimates of the unit axis and of the run that the panel of the Dig proposes: half window of m/z, tolerance of RT, end of the void front.

use crate::params::median;

/// Bounds of the unit-axis window of m/z (half width), Da, and of the tolerance of RT, minutes.
pub const DA_MIN: f64 = 0.1;
pub const DA_MAX: f64 = 0.5;
pub const RT_TOL_MIN: f64 = 0.05;
pub const RT_TOL_MAX: f64 = 1.0;
/// Void front: the TIC of the first fifth of the run peaks over `DEAD_PEAK` times its median and comes back under `DEAD_BACK` times it.
pub const DEAD_PEAK: f64 = 3.0;
pub const DEAD_BACK: f64 = 1.5;

/// Half window of m/z on the unit axis: three robust sigma of the offsets (Da) of the peaks from the mean m/z of their trace, within the bounds.
/// Returns `(Da, sigma)`.
pub fn da_from_offsets(offsets: &mut [f64]) -> Option<(f64, f64)> {
    if offsets.len() < 20 {
        return None;
    }
    let m = median(offsets);
    let mut dev: Vec<f64> = offsets.iter().map(|e| (e - m).abs()).collect();
    let sigma = 1.4826 * median(&mut dev);
    Some(((3.0 * sigma).clamp(DA_MIN, DA_MAX), sigma))
}

/// End of the void front, minutes: the RT where the TIC of the first fifth of the run, after its maximum, comes back under `DEAD_BACK` times the
/// median TIC. `None` when that maximum is not `DEAD_PEAK` times the median or the front does not end within the first third of the run.
pub fn dead_time(rt: &[f64], tic: &[f64]) -> Option<f64> {
    let n = rt.len();
    if n < 20 || tic.len() != n {
        return None;
    }
    let med = median(&mut tic.to_vec());
    let (first, last) = (rt[0], rt[n - 1]);
    let fifth = first + (last - first) * 0.2;
    let apex = (0..n)
        .filter(|&i| rt[i] <= fifth)
        .max_by(|&a, &b| tic[a].total_cmp(&tic[b]))?;
    if med <= 0.0 || tic[apex] < DEAD_PEAK * med {
        return None;
    }
    let end = (apex..n).find(|&i| tic[i] < DEAD_BACK * med)?;
    (rt[end] <= first + (last - first) / 3.0).then_some(rt[end])
}
