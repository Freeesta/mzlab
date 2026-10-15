//! The MS2 similarity of a row with the parent at unit resolution (`lr::ms2sim`): synthetic spectra, no data files.

use mzlab_feat::legacy::lr::ms2sim::{consensus, TOL_LR};
use mzlab_feat::legacy::similarity::{pair_components, pair_components_tol, prepare};

const PARENT: f64 = 300.0;
const PRODUCT: f64 = 316.0;

/// Parent fragments at P-a and P-b; the +16 product keeps the first (shifted by 16) and the second (equal).
fn parent() -> Vec<(f64, f64)> {
    vec![(200.0, 100.0), (150.0, 60.0), (121.0, 30.0)]
}

fn product(jitter: f64) -> Vec<(f64, f64)> {
    vec![
        (216.0 + jitter, 100.0),
        (150.0 - jitter, 55.0),
        (131.0, 20.0),
    ]
}

#[test]
fn equal_and_shifted_fragments() {
    let a = consensus(&parent(), PARENT).unwrap();
    let b = consensus(&product(0.0), PRODUCT).unwrap();
    let c = pair_components_tol(&a, PARENT, &b, PRODUCT, TOL_LR);
    assert_eq!(c.n[0], 1, "equal: {:?}", c.n);
    assert_eq!(c.n[1], 1, "shifted: {:?}", c.n);
    assert!(c.cos[0] > 0.0 && c.cos[1] > 0.0);
    assert!(c.cos_total() > 0.7, "total {}", c.cos_total());
}

#[test]
fn noise_inside_the_window_changes_nothing() {
    let a = consensus(&parent(), PARENT).unwrap();
    let r = |j| {
        let b = consensus(&product(j), PRODUCT).unwrap();
        pair_components_tol(&a, PARENT, &b, PRODUCT, TOL_LR)
    };
    let (c0, c1) = (r(0.0), r(0.3));
    assert_eq!(c0.n, c1.n);
    assert!((c0.cos_total() - c1.cos_total()).abs() < 1e-9);
    // the hundredth-of-a-Dalton tolerance of the high-resolution matching would lose the shifted fragment
    let b = consensus(&product(0.3), PRODUCT).unwrap();
    assert!(pair_components(&a, PARENT, &b, PRODUCT).cos_total() < c1.cos_total());
}

#[test]
fn close_peaks_are_merged_into_one() {
    let s = consensus(&[(200.0, 100.0), (200.3, 50.0), (150.0, 40.0)], PARENT).unwrap();
    assert_eq!(s.0.len(), 2);
    assert!((s.0[1] - 200.1).abs() < 0.05);
}

#[test]
fn no_fragments_no_spectrum() {
    assert!(consensus(&[], PARENT).is_none());
    // only the precursor region left: nothing to compare
    assert!(consensus(&[(299.0, 100.0)], PARENT).is_none());
}

#[test]
fn hr_wrapper_keeps_its_tolerance() {
    let (m, i) = (vec![100.0, 150.0], vec![1.0, 2.0]);
    let a = prepare(&m, &i, PARENT);
    let c = pair_components(&a, PARENT, &a, PARENT);
    let d = pair_components_tol(
        &a,
        PARENT,
        &a,
        PARENT,
        mzlab_feat::legacy::similarity::TOL_HR,
    );
    assert_eq!(c.n, d.n);
    assert_eq!(c.cos, d.cos);
}
