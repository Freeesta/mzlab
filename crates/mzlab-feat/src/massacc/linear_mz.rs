//! Model `linear_mz`: the error in ppm is a line of the m/z (`c0 + c1 * (mz - MZ_PIVOT) / MZ_SCALE`), the drift of a mass scale that is slightly off in slope. Used by `fit::Model`.

use super::fit::{MZ_PIVOT, MZ_SCALE};

/// The terms of the model at a point: the constant and the scaled m/z.
pub(super) fn basis(mz: f64, _rt: f64) -> [f64; 3] {
    [1.0, (mz - MZ_PIVOT) / MZ_SCALE, 0.0]
}
