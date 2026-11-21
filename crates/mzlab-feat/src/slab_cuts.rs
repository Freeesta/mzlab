//! The cuts of m/z that split a series into slabs (`slabs`, `spill`).

use crate::axis::half;
use crate::sortc;

/// Cuts of m/z, from `-inf` to `inf`: a slab holds about `slab_rows` features and ends in a gap wider than the ppm of the alignment.
pub(crate) fn cuts(all: &mut [f64], slab_rows: usize, ppm: f64) -> Vec<f64> {
    sortc::by(all, |a, b| a.total_cmp(b));
    let (mut cuts, mut since) = (vec![f64::NEG_INFINITY], 0usize);
    for i in 1..all.len() {
        since += 1;
        if since >= slab_rows && all[i] - all[i - 1] > half(all[i], ppm) {
            cuts.push(all[i]);
            since = 0;
        }
    }
    cuts.push(f64::INFINITY);
    cuts
}
