// The tests of mzidx_spill.rs: what the narrower reads of `read_parts` and the rounds of `allinea` rely on.
use super::*;

/// Rounded to 32 bits, a 64-bit m/z stays in its bucket or goes to the next one, never lower: `read_parts` reads only bucket `lo - 1` more.
#[test]
fn rounding_to_f32_moves_a_peak_up_one_bucket_at_most() {
    for k in 0..NB as u64 {
        let k = k as f64;
        for d in [-3i64, -2, -1, 0, 1, 2, 3] {
            for x in [k, k + 0.5, k + 1.0] {
                let x = f64::from_bits((x.to_bits() as i64 + d).max(0) as u64);
                let (b, r) = (bucket(x), bucket(f64::from(x as f32)));
                assert!(r == b || r == b + 1, "{x}: {b} -> {r}");
            }
        }
    }
}

/// A round ends on the last partition boundary it passes, keeps its end when it has none or is the last, and gives back the cost it leaves out.
#[test]
fn a_round_ends_on_a_partition_boundary() {
    let counts: Vec<u64> = (0..NB as u64)
        .map(|b| (b % 13 + 1) * u64::from(b > 100 && b < 2000))
        .collect();
    let parts = Parts::new(&counts);
    let cost = |b: usize| (b % 7 + 1) * 10;
    for (lo, hi) in [(0, 150), (150, 400), (300, 301), (1500, 1990), (9000, NB)] {
        let used: usize = (lo..hi).map(cost).sum();
        let (h, u) = align(Some(&parts), lo, hi, used, &cost);
        assert!(h > lo && h <= hi, "{lo}..{hi} -> {h}");
        assert_eq!(u, (lo..h).map(cost).sum::<usize>());
        let inside = parts.first.iter().any(|&f| f > lo && f <= hi);
        if hi < NB && inside {
            assert!(parts.first.contains(&h), "{lo}..{hi} -> {h}");
        } else {
            assert_eq!(h, hi);
        }
    }
    assert_eq!(align(None, 10, 20, 5, &cost), (20, 5));
}
