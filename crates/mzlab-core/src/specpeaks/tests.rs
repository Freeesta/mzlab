use super::*;

/// `merge_unit` as it was, with a fresh vector for every run of equal nominal mass.
fn old_merge_unit(mz: &[f64], y: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let mut o: Vec<usize> = (0..mz.len()).collect();
    o.sort_by(|a, b| mz[*a].total_cmp(&mz[*b]));
    let (mzs, ys): (Vec<f64>, Vec<f64>) = o.iter().map(|&i| (mz[i], y[i])).unzip();
    let nom: Vec<i64> = mzs.iter().map(|&m| nominal(m)).collect();
    let (starts, _g) = groups(&nom);
    let ra = |v: &[f64]| {
        if v.len() > 1 {
            v[0] + crate::stats::psum(&v[1..])
        } else {
            v[0]
        }
    };
    let mut sy = Vec::with_capacity(starts.len());
    let mut sx = Vec::with_capacity(starts.len());
    for (k, &a) in starts.iter().enumerate() {
        let b = starts.get(k + 1).copied().unwrap_or(mzs.len());
        sy.push(ra(&ys[a..b]));
        sx.push(ra(&(a..b).map(|i| mzs[i] * ys[i]).collect::<Vec<_>>()));
    }
    let smz = sx.iter().zip(&sy).map(|(a, b)| a / b.max(1e-12)).collect();
    (smz, sy)
}

#[test]
fn merge_unit_has_the_same_bits_with_a_reused_buffer() {
    let mut s = 5u64;
    let mut next = || {
        s ^= s >> 12;
        s ^= s << 25;
        s ^= s >> 27;
        (s.wrapping_mul(0x2545F4914F6CDD1D) >> 11) as f64 / (1u64 << 53) as f64
    };
    for n in (2..40).chain([200, 1000]) {
        let mz: Vec<f64> = (0..n).map(|_| 50.0 + next() * 30.0).collect();
        let y: Vec<f64> = (0..n).map(|_| (next() * 5000.0).round()).collect();
        let (a, b) = (merge_unit(&mz, &y), old_merge_unit(&mz, &y));
        let bits = |v: &[f64]| v.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
        assert_eq!((bits(&a.0), bits(&a.1)), (bits(&b.0), bits(&b.1)), "n {n}");
    }
}
