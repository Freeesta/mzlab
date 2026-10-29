use super::*;

/// `pearson` as it was, with a fresh vector for every product sum and the squared sums computed twice.
fn old_pearson(a: &[f64], b: &[f64]) -> Option<f64> {
    let n = a.len();
    if n < 3 {
        return None;
    }
    let mean = |v: &[f64]| psum(v) / n as f64;
    let centred = |v: &[f64]| -> Vec<f64> {
        let m = mean(v);
        v.iter().map(|x| x - m).collect()
    };
    let (da, db) = (centred(a), centred(b));
    let sq = |d: &[f64]| psum(&d.iter().map(|x| x * x).collect::<Vec<_>>());
    if sq(&da) / n as f64 == 0.0 || sq(&db) / n as f64 == 0.0 {
        return None;
    }
    let cov = psum(&da.iter().zip(&db).map(|(x, y)| x * y).collect::<Vec<_>>()) / (n - 1) as f64;
    let (sa, sb) = (
        (sq(&da) / (n - 1) as f64).sqrt(),
        (sq(&db) / (n - 1) as f64).sqrt(),
    );
    Some((cov / sa / sb).clamp(-1.0, 1.0))
}

#[test]
fn pearson_has_the_same_bits_with_one_scratch_buffer() {
    let mut s = 31u64;
    let mut next = || {
        s ^= s >> 12;
        s ^= s << 25;
        s ^= s >> 27;
        (s.wrapping_mul(0x2545F4914F6CDD1D) >> 40) as f64 / 17.0
    };
    // lengths around the pairwise-sum block sizes (8 and 128), plus flat traces
    for n in (0..40).chain([127, 128, 129, 130, 255, 256, 257, 1000]) {
        let a: Vec<f64> = (0..n).map(|_| next()).collect();
        let b: Vec<f64> = (0..n).map(|_| next()).collect();
        let flat = vec![3.0; n];
        for (x, y) in [(&a, &b), (&a, &a), (&a, &flat), (&flat, &b)] {
            let (new, old) = (pearson(x, y), old_pearson(x, y));
            assert_eq!(new.map(f64::to_bits), old.map(f64::to_bits), "n {n}");
        }
    }
}
