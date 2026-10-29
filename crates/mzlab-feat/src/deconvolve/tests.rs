//! Bit-for-bit comparison of the deconvolution with the verbatim pre-optimisation copy in `old.rs`.

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
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

fn bits(v: &[f64]) -> Vec<u64> {
    v.iter().map(|x| x.to_bits()).collect()
}

/// Random chimeric-like matrix: `k` Gaussian elution profiles times sparse spectra, plus noise and zeros.
fn random_matrix(rng: &mut Rng, n: usize, m: usize, k: usize) -> Vec<Vec<f64>> {
    let spectra: Vec<Vec<f64>> = (0..k)
        .map(|_| {
            (0..m)
                .map(|_| {
                    if rng.unit() < 0.4 {
                        rng.unit() * 100.0
                    } else {
                        0.0
                    }
                })
                .collect()
        })
        .collect();
    let centers: Vec<f64> = (0..k).map(|_| rng.unit() * n as f64).collect();
    (0..n)
        .map(|i| {
            (0..m)
                .map(|j| {
                    let mut v = 0.0;
                    for (c, sp) in centers.iter().zip(&spectra) {
                        let z = (i as f64 - c) / (1.0 + n as f64 / 8.0);
                        v += sp[j] * (-z * z).exp();
                    }
                    if rng.unit() < 0.15 {
                        v = 0.0;
                    }
                    v + if rng.unit() < 0.3 {
                        rng.unit() * 0.5
                    } else {
                        0.0
                    }
                })
                .collect()
        })
        .collect()
}

fn flat(d: &[Vec<f64>]) -> Vec<f64> {
    d.iter().flatten().copied().collect()
}

#[test]
fn mcr_als_bit_identical_on_1000_matrices() {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    for case in 0..1000 {
        let (n, m, k) = (6 + rng.below(30), 4 + rng.below(25), 2 + rng.below(2));
        let d = random_matrix(&mut rng, n, m, k);
        let unimodal = case % 2 == 0;
        let s0 = (case % 3 == 0).then(|| {
            (0..m)
                .map(|_| (0..k).map(|_| rng.unit()).collect::<Vec<f64>>())
                .collect::<Vec<_>>()
        });
        let (oc, os, ol) = old::mcr::mcr_als(&d, k, unimodal, s0.clone());
        let (nc, ns, nl) = mcr_als(&flat(&d), n, m, k, unimodal, s0.as_deref().map(flat));
        assert_eq!(bits(&flat(&oc)), bits(&nc), "case {case}: C differs");
        assert_eq!(bits(&flat(&os)), bits(&ns), "case {case}: S differs");
        assert_eq!(ol.to_bits(), nl.to_bits(), "case {case}: LOF differs");
    }
}

#[test]
fn interp_linear_matches_the_linear_scan() {
    let mut rng = Rng(7);
    for case in 0..2000 {
        let n = rng.below(9);
        let mut xp: Vec<f64> = (0..n).map(|_| rng.below(6) as f64).collect();
        if case % 4 != 0 {
            xp.sort_by(f64::total_cmp);
        }
        if case % 50 == 0 && n > 3 {
            // Only interior NaNs: at the ends the old scan indexes past the array.
            xp[1 + rng.below(n - 2)] = f64::NAN;
        }
        let fp: Vec<f64> = (0..n + rng.below(2)).map(|_| rng.unit() * 10.0).collect();
        let mut x: Vec<f64> = (0..12).map(|_| rng.below(14) as f64 / 2.0 - 0.5).collect();
        if n > 1 {
            x.push(f64::NAN);
        }
        let want = old::mcr::interp_linear(&x, &xp, &fp);
        assert_eq!(
            bits(&want),
            bits(&interp_linear(&x, &xp, &fp)),
            "case {case}"
        );
    }
    assert!(interp_linear(&[1.0], &[], &[]).iter().all(|&v| v == 0.0));
    assert!(interp_linear(&[], &[1.0], &[2.0]).is_empty());
}

/// Random list of scans: peaks near a few fragment masses, duplicates inside a scan, zeros and negatives.
fn random_scans(
    rng: &mut Rng,
    n: usize,
    with_negative: bool,
) -> (Vec<Vec<f64>>, Vec<Vec<f64>>, Vec<f64>) {
    let masses: Vec<f64> = (0..3 + rng.below(8))
        .map(|_| 50.0 + rng.unit() * 400.0)
        .collect();
    let (mut mzs, mut ints) = (Vec::new(), Vec::new());
    for _ in 0..n {
        let (mut mz, mut it) = (Vec::new(), Vec::new());
        for &mass in &masses {
            for _ in 0..1 + rng.below(2) {
                if rng.unit() < 0.7 {
                    mz.push(mass * (1.0 + (rng.unit() - 0.5) * 2e-5));
                    let v = rng.unit() * 1000.0;
                    it.push(match rng.below(8) {
                        0 => 0.0,
                        1 if with_negative => -v,
                        _ => v,
                    });
                }
            }
        }
        mzs.push(mz);
        ints.push(it);
    }
    let rts = (0..n)
        .map(|i| (i as f64) * 0.02 + rng.unit() * 0.005)
        .collect();
    (mzs, ints, rts)
}

#[test]
fn fragment_matrix_and_consensus_match_the_old_ones() {
    let mut rng = Rng(11);
    for case in 0..600 {
        let n = rng.below(14);
        let (mzs, ints, _) = random_scans(&mut rng, n, case % 3 == 0);
        let (ppm, min_scans) = (8.0 + (case % 5) as f64, rng.below(5));
        for row_norm in [true, false] {
            let want = old::fragment_matrix(&mzs, &ints, ppm, min_scans, row_norm);
            assert_eq!(
                format!("{want:?}"),
                format!(
                    "{:?}",
                    fragment_matrix(&mzs, &ints, ppm, min_scans, row_norm)
                ),
                "case {case}"
            );
        }
        let want = old::ms2_consensus(&mzs, &ints, ppm);
        assert_eq!(
            format!("{want:?}"),
            format!("{:?}", ms2_consensus(&mzs, &ints, ppm)),
            "case {case}"
        );
    }
}

#[test]
fn deconvolve_isomers_matches_the_old_one() {
    let mut rng = Rng(23);
    for case in 0..300 {
        let n = 4 + rng.below(25);
        let (mzs, ints, mut rts) = random_scans(&mut rng, n, false);
        if case % 2 == 0 {
            rts.reverse();
        }
        let xr: Vec<f64> = (0..40).map(|i| i as f64 * 0.025).collect();
        let xy: Vec<f64> = xr
            .iter()
            .map(|&t| {
                (-((t - 0.2) / 0.05).powi(2)).exp() + 0.8 * (-((t - 0.45) / 0.05).powi(2)).exp()
            })
            .collect();
        let xic = (case % 3 != 0).then_some((xr.as_slice(), xy.as_slice()));
        let params = DeconvolveParams {
            min_scans: 4 + case % 3,
            ..DeconvolveParams::default()
        };
        let want = old::deconvolve_isomers(&mzs, &ints, &rts, xic, &params);
        let got = deconvolve_isomers(&mzs, &ints, &rts, xic, &params);
        assert_eq!(format!("{want:?}"), format!("{got:?}"), "case {case}");
    }
}
