//! Study item 9: the S/N of the `sn_min` gate of the line-residual route names its estimator.

use mzlab_core::peakq_lr::SNR_ESTIMATOR;

#[test]
fn the_estimator_is_named() {
    assert!(SNR_ESTIMATOR.contains("1.4826 MAD"), "{SNR_ESTIMATOR}");
    assert!(SNR_ESTIMATOR.contains("sqrt(1 - c0)"), "{SNR_ESTIMATOR}");
}
