//! The reused buffers and the one-pass noise estimate against the verbatim previous code, bit for bit.
use super::*;

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// A series of `n` scans: a few Gaussian peaks of random width and height over sparse noise, with runs of zeros.
fn series(rng: &mut Rng, n: usize) -> Vec<f64> {
    let peaks: Vec<(f64, f64, f64)> = (0..rng.next() % 4)
        .map(|_| {
            (
                rng.unit() * n as f64,
                1.0 + rng.unit() * 6.0,
                1e4 + rng.unit() * 1e6,
            )
        })
        .collect();
    (0..n)
        .map(|i| {
            let mut v: f64 = peaks
                .iter()
                .map(|&(c, s, h)| h * (-0.5 * ((i as f64 - c) / s).powi(2)).exp())
                .sum();
            if rng.unit() < 0.3 {
                v += rng.unit() * 3e3;
            }
            if v < 2e3 && rng.unit() < 0.5 {
                v = 0.0;
            }
            v
        })
        .collect()
}

#[test]
fn peaks_are_bit_identical_to_the_version_that_allocated_per_series() {
    let mut rng = Rng(0xBEEF);
    let mut work = Scratch::default();
    for case in 0..3000 {
        let n = [3, 9, 20, 60, 150, 400][case % 6];
        let y = series(&mut rng, n);
        let dt = 0.5 + rng.unit() * 2.0;
        let wavelets: Vec<Wavelet> = scales((5.0, 30.0), dt)
            .into_iter()
            .map(Wavelet::new)
            .collect();
        let (snr, floor) = [(10.0, 2e4), (3.0, 1e3), (0.0, 0.0)][case % 3];
        // the reused buffers hold the previous series when this one starts
        let want = before::peaks(&y, &wavelets, snr, floor).unwrap();
        let got = peaks(&y, &wavelets, snr, floor, &mut work).unwrap();
        assert_eq!(want, got, "case {case}");
        let mut r = Vec::new();
        response(&y, &wavelets[case % wavelets.len()], &mut r).unwrap();
        let want_r = before::response(&y, &wavelets[case % wavelets.len()]).unwrap();
        assert_eq!(
            want_r.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            r.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            "response, case {case}"
        );
    }
}
