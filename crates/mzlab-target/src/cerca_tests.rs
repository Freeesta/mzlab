//! Tests of the sibling module (loaded with `#[path]`, so they see its private items).

use super::*;
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn unit(&mut self) -> f64 {
        self.next() as f64 / (1u64 << 31) as f64
    }
}

fn spec(mz: Vec<f64>, intensity: Vec<f32>) -> Spectrum {
    Spectrum {
        index: 0,
        id: String::new(),
        level: 1,
        rt: 0.0,
        polarity: 1,
        profile: false,
        filter: String::new(),
        precursor: None,
        mz,
        intensity,
        wide: None,
    }
}

fn linear(s: &Spectrum, lo: f64, hi: f64) -> Vec<(f64, f32)> {
    s.mz.iter()
        .zip(&s.intensity)
        .filter(|(&m, _)| m >= lo && m <= hi)
        .map(|(&m, &i)| (m, i))
        .collect()
}

#[test]
fn window_equals_the_linear_scan_sorted_or_not() {
    let mut r = Lcg(7);
    for round in 0..400 {
        let n = (r.next() % 40) as usize;
        // few distinct values: many ties and peaks exactly on the window edges
        let mut mz: Vec<f64> = (0..n)
            .map(|_| 100.0 + (r.next() % 12) as f64 * 0.25)
            .collect();
        if round % 2 == 0 {
            mz.sort_by(|a, b| a.total_cmp(b));
        }
        if round % 7 == 0 && n > 0 {
            mz[n / 2] = f64::NAN;
        }
        let intensity: Vec<f32> = (0..n).map(|_| (r.next() % 50) as f32).collect();
        let s = spec(mz, intensity);
        let asc = is_ascending(&s);
        assert_eq!(asc, s.mz.windows(2).all(|w| w[0] <= w[1]));
        for _ in 0..10 {
            let lo = 99.5 + r.unit() * 3.5;
            let hi = lo + r.unit() * 1.5;
            let (lo, hi) = if round % 3 == 0 {
                (100.5, 101.25)
            } else {
                (lo, hi)
            };
            let got: Vec<(f64, f32)> = window(&s, asc, lo, hi).collect();
            let want = linear(&s, lo, hi);
            assert_eq!(got.len(), want.len());
            for (g, w) in got.iter().zip(&want) {
                assert_eq!((g.0.to_bits(), g.1), (w.0.to_bits(), w.1));
            }
        }
    }
}

#[test]
fn window_survives_short_intensity_array() {
    let s = spec(vec![1.0, 2.0, 3.0], vec![5.0]);
    assert_eq!(window(&s, true, 0.0, 9.0).count(), 1);
    assert_eq!(window(&s, false, 0.0, 9.0).count(), 1);
}

fn noise_reference(y: &[f64]) -> f64 {
    if y.is_empty() {
        return 1.0;
    }
    let med = |v: &mut Vec<f64>| {
        v.sort_by(|a, b| a.total_cmp(b));
        let mid = v.len() / 2;
        if v.len() % 2 == 0 && v.len() > 1 {
            (v[mid - 1] + v[mid]) * 0.5
        } else {
            v[mid]
        }
    };
    let m = med(&mut y.to_vec());
    let mut dev: Vec<f64> = y.iter().map(|v| (v - m).abs()).collect();
    let n = med(&mut dev) * 1.4826;
    if n > 1e-6 {
        n
    } else {
        1.0
    }
}

#[test]
fn noise_matches_the_sorted_median_bit_for_bit() {
    let mut r = Lcg(11);
    let mut buf = Vec::new();
    for round in 0..500 {
        let n = (r.next() % 60) as usize;
        let mut y: Vec<f64> = (0..n)
            .map(|_| match round % 4 {
                0 => (r.next() % 5) as f64,
                1 => r.unit() * 1e6,
                2 => 0.0,
                _ => (r.next() % 3) as f64 * 1000.5,
            })
            .collect();
        if round % 11 == 0 && n > 2 {
            y[1] = f64::NAN;
        }
        if round % 13 == 0 && n > 2 {
            y[0] = f64::INFINITY;
        }
        let want = noise_reference(&y);
        let got = estimate_noise(&y, &mut buf);
        assert_eq!(got.to_bits(), want.to_bits(), "round {round} {y:?}");
    }
}

#[test]
fn flat_trace_noise_falls_back_to_one() {
    let mut buf = Vec::new();
    assert_eq!(estimate_noise(&[0.0; 9], &mut buf), 1.0);
    assert_eq!(estimate_noise(&[], &mut buf), 1.0);
}

fn gauss(rt: f64, c: f64, h: f64, w: f64) -> f64 {
    h * (-0.5 * ((rt - c) / w).powi(2)).exp()
}

#[test]
fn two_coeluting_peaks_with_a_shallow_valley_are_both_found() {
    // valley at ~60 % of the lower apex, far above the 5 % tail cut: the first peak's right edge stops at the valley minimum
    let rt: Vec<f64> = (0..120).map(|k| k as f64 * 0.01).collect();
    let y: Vec<f64> = rt
        .iter()
        .map(|&t| gauss(t, 0.45, 10_000.0, 0.06) + gauss(t, 0.72, 7_000.0, 0.06))
        .collect();
    let peaks = detect_chrom_peaks(&rt, &y, 3.0, 1.0, 10.0);
    let apexes: Vec<f64> = peaks.iter().map(|p| rt[p.apex_idx]).collect();
    assert_eq!(apexes.len(), 2, "{apexes:?}");
    assert!((apexes[0] - 0.45).abs() < 0.011 && (apexes[1] - 0.72).abs() < 0.011);
    // the second peak starts right where the first one ends (the valley minimum)
    assert_eq!(peaks[0].rt_end, peaks[1].rt_start);
}

#[test]
fn second_apex_right_after_the_valley_and_after_a_tail_cut_is_found() {
    // apex, drop below 5 % of the height, immediately a new apex
    let rt: Vec<f64> = (0..12).map(|k| k as f64).collect();
    let y = [
        0.0, 100.0, 200.0, 1.0, 50.0, 90.0, 40.0, 0.0, 0.0, 0.0, 0.0, 0.0,
    ];
    let peaks = detect_chrom_peaks(&rt, &y, 1.0, 0.0, 1.0);
    let apexes: Vec<usize> = peaks.iter().map(|p| p.apex_idx).collect();
    assert_eq!(apexes, vec![2, 5]);
    // a rise right after the cut point: apex at the very next sample
    let y = [
        0.0, 100.0, 200.0, 1.0, 80.0, 30.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
    ];
    let apexes: Vec<usize> = detect_chrom_peaks(&rt, &y, 1.0, 0.0, 1.0)
        .iter()
        .map(|p| p.apex_idx)
        .collect();
    assert_eq!(apexes, vec![2, 4]);
}

#[test]
fn fallback_isotope_peaks_prefer_the_strongest_then_the_closest() {
    let obs = [
        (100.0, 5.0),
        (100.0002, 50.0),
        (100.0001, 50.0),
        (101.0, 9.0),
    ];
    let ok = |m: f64| (m - 100.0).abs() <= 0.001;
    assert_eq!(strongest(&obs, 100.0, ok), Some((100.0001, 50.0)));
    assert_eq!(strongest(&obs, 100.0, |m| m > 200.0), None);
}
