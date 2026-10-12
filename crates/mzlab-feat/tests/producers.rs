//! Per-row producers: the share of MS2 peaks explained by the parent (equal or shifted by the mass difference).

use mzlab_feat::evid::producers::{network_share, Peaks};

fn peaks(mz: &[f64]) -> Peaks {
    Peaks {
        mz: mz.to_vec(),
        it: vec![1.0; mz.len()],
    }
}

#[test]
fn equal_and_shifted_fragments_are_explained_and_the_others_are_not() {
    let parent = peaks(&[100.0, 150.0, 200.0]);
    let delta = 15.9949;
    let child = peaks(&[100.0, 150.0 + delta, 77.0, 301.0]);
    assert_eq!(network_share(&child, &parent, delta, 0.01), Some(0.5));
    assert_eq!(
        network_share(&peaks(&[1.0]), &Peaks::default(), delta, 0.01),
        None
    );
}

#[test]
fn weak_peaks_are_not_counted() {
    let parent = peaks(&[100.0]);
    let child = Peaks {
        mz: vec![100.0, 55.0],
        it: vec![1.0, 0.01],
    };
    assert_eq!(network_share(&child, &parent, 0.0, 0.01), Some(1.0));
}

use mzlab_core::composition::isotope_ratios;
use mzlab_feat::evid::producers::{isotope_score, FeatIndex};
use mzlab_feat::table::FileFeatures;

fn file(rows: &[(f64, f32, f64)]) -> FileFeatures {
    let mut f = FileFeatures::default();
    for &(mz, rt, a) in rows {
        f.mz.push(mz);
        f.rt.push(rt);
        f.area_win.push(a);
    }
    f
}

#[test]
fn isotope_pattern_agrees_with_the_formula_or_not() {
    // C13H24N4O3S, [M+H]+ = C13H25N4O3S
    let ratio = isotope_ratios(&[
        ("C".to_string(), 13),
        ("H".to_string(), 25),
        ("N".to_string(), 4),
        ("O".to_string(), 3),
        ("S".to_string(), 1),
    ])
    .0;
    let mz = 317.1642;
    let good = file(&[(mz, 5.0, 1e6), (mz + 1.003355, 5.0, 1e6 * ratio)]);
    let bad = file(&[(mz, 5.0, 1e6), (mz + 1.003355, 5.0, 1e6 * ratio * 3.0)]);
    let alone = file(&[(mz, 5.0, 1e6)]);
    let score = |f: &FileFeatures| isotope_score(&[FeatIndex::new(f)], mz, 5.0, "C13H24N4O3S", 1);
    assert!(score(&good).unwrap() > 0.99);
    assert_eq!(score(&bad), Some(0.0));
    assert_eq!(score(&alone), Some(0.0));
    assert_eq!(score(&file(&[])), None);
}
