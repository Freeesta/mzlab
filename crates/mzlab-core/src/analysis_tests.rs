//! Unit resolution, binning and min-max thinning of `analysis`.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]
use super::*;

#[test]
fn unit_resolution_is_everything_but_orbitrap_and_tof() {
    let q = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let none: [&Spectrum; 0] = [];
    assert!(is_unit_resolution(
        &q(&["quadrupole", "quadrupole", "ion trap"]),
        none
    ));
    assert!(is_unit_resolution(&[], none));
    assert!(!is_unit_resolution(&q(&["orbitrap"]), none));
    assert!(!is_unit_resolution(
        &q(&["quadrupole", "time-of-flight"]),
        none
    ));
    let s = Spectrum {
        filter: "FTMS + p ESI Full ms".into(),
        ..Default::default()
    };
    assert!(!is_unit_resolution(&[], [&s]));
}

#[test]
fn binning_merges_cells_with_weighted_mz() {
    let (m, y) = bin_centroids(&[100.01, 100.05, 100.31], &[1.0, 3.0, 2.0], 0.1);
    assert_eq!(y, vec![4.0, 2.0]);
    assert!((m[0] - (100.01 + 3.0 * 100.05) / 4.0).abs() < 1e-9);
    assert!((m[1] - 100.31).abs() < 1e-9);
}

#[test]
fn the_ascending_path_is_the_map_to_the_bit() {
    let mut seed = 99u64;
    let mut r = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let mut mz: Vec<f64> = (0..3000)
        .map(|_| 50.0 + (r() % 95_000_000) as f64 / 100_000.0)
        .collect();
    mz.sort_by(f64::total_cmp);
    let it: Vec<f32> = (0..mz.len())
        .map(|_| (r() % 100_000) as f32 * 0.37)
        .collect();
    for bin in [0.01, 0.1, 1.0] {
        let fast = bin_ascending(&mz, &it, bin).unwrap();
        let mut cells: std::collections::BTreeMap<i64, (f64, f64)> = Default::default();
        for (&m, &i) in mz.iter().zip(&it) {
            let c = cells.entry((m / bin).floor() as i64).or_insert((0.0, 0.0));
            c.0 += i as f64;
            c.1 += i as f64 * m;
        }
        let (wm, wy): (Vec<f64>, Vec<f64>) = cells
            .values()
            .map(|&(sy, smy)| (smy / sy.max(1e-12), sy))
            .unzip();
        let bits = |v: &[f64]| v.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
        assert_eq!(
            (bits(&fast.0), bits(&fast.1)),
            (bits(&wm), bits(&wy)),
            "bin {bin}"
        );
    }
    assert!(
        bin_ascending(&[2.0, 1.0], &[1.0f32, 1.0], 0.1).is_none()
            && bin_ascending(&[1.0], &[1.0f32], 0.0).is_none()
    );
}

#[test]
fn minmax_keeps_extremes_and_bounds() {
    let x: Vec<f64> = (0..1000).map(|i| i as f64).collect();
    let mut y = vec![1.0; 1000];
    y[503] = 99.0;
    y[17] = -5.0;
    let k = minmax_indices(&x, &y, 10);
    assert!(k.len() <= 20 && k.contains(&503) && k.contains(&17));
    assert!(k.windows(2).all(|w| w[0] < w[1]));
    assert_eq!(minmax_indices(&x, &y, 0).len(), 1000);
}
