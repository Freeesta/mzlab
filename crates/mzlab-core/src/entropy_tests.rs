//! The spectral entropy similarity and its peak pairing.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]
/// `matches` as the plain double loop over every (i, j).
fn reference(a: &[f64], ai: &[f64], b: &[f64], bi: &[f64], tol: f64) -> Vec<(usize, usize)> {
    let mut c: Vec<(usize, usize, f64)> = Vec::new();
    for i in 0..a.len() {
        for j in 0..b.len() {
            if (a[i] - b[j]).abs() <= tol {
                c.push((i, j, ai[i] * bi[j]));
            }
        }
    }
    crate::sortc::by(&mut c, |x, y| {
        y.2.partial_cmp(&x.2)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(x.0.cmp(&y.0))
            .then(x.1.cmp(&y.1))
    });
    let (mut ua, mut ub) = (vec![false; a.len()], vec![false; b.len()]);
    let mut out = Vec::new();
    for (i, j, _) in c {
        if !ua[i] && !ub[j] {
            ua[i] = true;
            ub[j] = true;
            out.push((i, j));
        }
    }
    out
}

#[test]
fn the_sorted_search_finds_the_pairs_of_the_double_loop() {
    let mut seed = 7u64;
    let mut r = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed % 20_000) as f64 / 100.0
    };
    let it = |v: &[f64]| {
        (0..v.len())
            .map(|k| ((k * 7) % 5) as f64 + 1.0)
            .collect::<Vec<_>>()
    };
    for n in [0usize, 1, 5, 60, 300] {
        let mut a: Vec<f64> = (0..n).map(|_| 50.0 + r()).collect();
        let mut b: Vec<f64> = (0..n + 3).map(|_| 50.0 + r()).collect();
        b.extend_from_slice(&a[..n / 2]);
        a.sort_by(f64::total_cmp);
        let unsorted = b.clone();
        b.sort_by(f64::total_cmp);
        for tol in [0.0, 0.01, 0.5, 3.0] {
            for bb in [&b, &unsorted] {
                let (ai, bi) = (it(&a), it(bb));
                assert_eq!(
                    super::matches(&a, &ai, bb, &bi, tol),
                    reference(&a, &ai, bb, &bi, tol),
                    "n={n} tol={tol}"
                );
            }
        }
    }
}
