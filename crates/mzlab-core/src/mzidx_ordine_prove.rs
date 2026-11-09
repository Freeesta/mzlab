// The tests of mzidx_ordine.rs on buckets shaped like real HR data.
use super::*;

/// A 64-bit m/z bucket as HR data fill it (the same ions in many scans, values a few ulps apart, NaN, both zeros) is ordered as the stable comparison sort orders it.
#[test]
#[allow(clippy::disallowed_methods)] // the reference order
fn hr_bucket_is_the_stable_sort() {
    let mut x = 7u64;
    let mut next = move || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x
    };
    let ions: [f64; 4] = [445.120_31, 445.120_32, 445.5, 445.999_999_9];
    let v: Vec<Peak<f64>> = (0..5000u32)
        .map(|i| {
            let mz = match next() % 10 {
                0 => f64::NAN,
                1 => [0.0, -0.0][(next() % 2) as usize],
                2 => 445.0 + (next() % 1_000_000) as f64 * 1e-6,
                k => f64::from_bits(ions[k as usize % 4].to_bits() + next() % 3),
            };
            Peak {
                mz,
                it: f64::from(i),
                pos: (next() % 400) as u32,
            }
        })
        .collect();
    for per_scan in [false, true] {
        let (mut a, mut b, mut tmp) = (v.clone(), v.clone(), Vec::new());
        sort(&mut a, &mut tmp, per_scan).unwrap();
        b.sort_by(|x, y| {
            let o = by_mz(x, y);
            if per_scan {
                o.then(x.pos.cmp(&y.pos))
            } else {
                o
            }
        });
        assert!(a
            .iter()
            .zip(&b)
            .all(|(p, q)| p.mz.to_bits() == q.mz.to_bits() && p.it == q.it && p.pos == q.pos));
    }
}
