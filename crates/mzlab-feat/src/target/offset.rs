//! The offset of the calibration of a unit-resolution instrument: its centroids are often a few tenths of a dalton off. It is measured on the parent, the
//! strongest ion near its calculated m/z at its RT in each file, as the median of observed minus calculated, and applied to the hypotheses (`TargetParams::mz_offset`).

use crate::sortc;
use crate::table::FileFeatures;

/// Largest offset taken as a calibration error, Da; a bigger one is another ion.
pub const MAX_OFFSET: f64 = 0.6;
/// Window of RT around the RT of the parent, minutes.
const RT_WINDOW: f64 = 0.3;

/// Median offset over the files, or `None` when no file shows the parent or the offset is above `MAX_OFFSET`.
pub fn estimate(files: &[&FileFeatures], parent_mz: f64, parent_rt: Option<f64>) -> Option<f64> {
    let mut seen: Vec<f64> = Vec::new();
    for f in files {
        let a = f.mz.partition_point(|&m| m < parent_mz - MAX_OFFSET);
        let b = f.mz.partition_point(|&m| m <= parent_mz + MAX_OFFSET);
        let best = (a..b)
            .filter(|&i| parent_rt.is_none_or(|t| (f64::from(f.rt[i]) - t).abs() <= RT_WINDOW))
            .max_by(|&i, &j| f.height[i].total_cmp(&f.height[j]));
        seen.extend(best.map(|i| f.mz[i] - parent_mz));
    }
    if seen.is_empty() {
        return None;
    }
    sortc::by(&mut seen, f64::total_cmp);
    let n = seen.len();
    let med = if n % 2 == 1 {
        seen[n / 2]
    } else {
        (seen[n / 2 - 1] + seen[n / 2]) / 2.0
    };
    (med.abs() <= MAX_OFFSET).then_some(med)
}
