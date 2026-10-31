//! Model `linear_mz_rt`: `linear_mz` plus a term in the RT (`c2 * (rt - RT_PIVOT)`, minutes), the drift of the calibration along the run (temperature, charging). Used by `fit::Model`.

use super::fit::{MZ_PIVOT, MZ_SCALE, RT_PIVOT};

/// The terms of the model at a point: the constant, the scaled m/z and the RT.
pub(super) fn basis(mz: f64, rt: f64) -> [f64; 3] {
    [1.0, (mz - MZ_PIVOT) / MZ_SCALE, rt - RT_PIVOT]
}
