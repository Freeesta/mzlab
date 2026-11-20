//! Study item 9: the S/N of the `sn_min` gate of the line-residual route names its estimator (a name only; the help of `peak edges` is pinned by `every_help_and_usage_error_is_unchanged`).

use mzlab_core::peakq_lr::SNR_ESTIMATOR;

#[test]
fn the_estimator_is_named() {
    assert!(SNR_ESTIMATOR.contains("1.4826 MAD"), "{SNR_ESTIMATOR}");
    assert!(SNR_ESTIMATOR.contains("sqrt(1 - c0)"), "{SNR_ESTIMATOR}");
    assert!(SNR_ESTIMATOR.contains("n - 1"), "{SNR_ESTIMATOR}");
}
