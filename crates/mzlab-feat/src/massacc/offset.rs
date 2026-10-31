//! Model `offset`: one error in ppm for the whole file, whatever the m/z and the RT. Used by `fit::Model`.

/// The terms of the model at a point: the constant only.
pub(super) fn basis(_mz: f64, _rt: f64) -> [f64; 3] {
    [1.0, 0.0, 0.0]
}
