//! The MRM pipeline (`mrm.rs`) on sparse counts like those of a triple quadrupole: a point every 0.06 s for 20 minutes, values in steps of one count
//! (40 counts per second), a background of a few counts per second so that most points are zero. Six calibration levels and three samples, one
//! precursor with a quantifier and a qualifier (40 %). Every transition must give one row, detected at every level with an area that grows with the
//! concentration, and coherent cells in the samples. Synthetic data only.

use mzlab_feat::mrm::{align, detect, fill_file, MrmParams, Srm};
use mzlab_feat::table::{CELL_ABSENT, CELL_DETECTED, CELL_FILLED};

/// Six calibration levels then three samples.
const CONC: [f64; 9] = [0.1, 0.5, 2.0, 5.0, 10.0, 20.0, 8.0, 1.0, 0.2];
/// One count of the detector, counts per second.
const QUANTUM: f64 = 40.0;
/// Background, counts per second, and apex of the quantifier per unit of concentration.
const BACKGROUND: f64 = 4.0;
const APEX: f64 = 20000.0;

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 11) as f64 + 0.5) / (1u64 << 53) as f64
    }

    /// A Poisson draw of mean `l` (Knuth below 30, the normal approximation above).
    fn poisson(&mut self, l: f64) -> f64 {
        if l > 30.0 {
            let g = (-2.0 * self.next().ln()).sqrt() * (std::f64::consts::TAU * self.next()).cos();
            return (l + l.sqrt() * g).round().max(0.0);
        }
        let (lim, mut p, mut k) = ((-l).exp(), 1.0, 0.0);
        loop {
            p *= self.next();
            if p <= lim {
                return k;
            }
            k += 1.0;
        }
    }
}

/// The two SRM chromatograms of file `k`: counts in steps of `QUANTUM` around a peak at 10 min (sigma 3 s).
fn chromatograms(k: usize) -> Vec<Srm> {
    let mut rng = Lcg(11 + k as u64);
    let t: Vec<f64> = (0..20000).map(|i| 0.001 * i as f64).collect();
    [(250.1, 160.0, 0.4), (250.1, 208.1, 1.0)]
        .iter()
        .map(|&(q1, q3, part)| {
            let y = t
                .iter()
                .map(|&x| {
                    let rate = BACKGROUND
                        + part * APEX * CONC[k] * (-0.5 * ((x - 10.0) / 0.05).powi(2)).exp();
                    QUANTUM * rng.poisson(rate / QUANTUM)
                })
                .collect();
            Srm {
                q1,
                q3,
                name: String::new(),
                t: t.clone(),
                y,
            }
        })
        .collect()
}

#[test]
fn sparse_counts_give_one_row_per_transition() {
    let files: Vec<Vec<Srm>> = (0..CONC.len()).map(chromatograms).collect();
    let zeros = files[0][0].y.iter().filter(|&&v| v == 0.0).count() as f64 / 20000.0;
    assert!(zeros >= 0.8, "the chromatograms are sparse ({zeros})");
    let p = MrmParams::default();
    let feats: Vec<_> = files.iter().map(|f| detect(f, &p)).collect();
    let refs: Vec<_> = feats.iter().collect();
    let mut t = align(&refs, &p);
    assert_eq!(t.len(), 2, "one row per transition, no row of noise");
    let nf = t.n_files;
    for r in 0..2 {
        assert!((t.rt[r] - 10.0).abs() < 0.02, "row {r} at {}", t.rt[r]);
        assert!((0..6).all(|k| t.state[r * nf + k] == CELL_DETECTED));
        assert!(
            (0..5).all(|k| t.area[r * nf + k + 1] > t.area[r * nf + k]),
            "the area grows with the level: {:?}",
            &t.area[r * nf..r * nf + 6]
        );
    }
    for (k, f) in files.iter().enumerate().skip(6) {
        fill_file(&mut t, k, f, &p);
    }
    for (s, a) in t.state.iter().zip(&t.area) {
        let ok = match *s {
            CELL_DETECTED | CELL_FILLED => a.is_finite() && *a > 0.0,
            CELL_ABSENT => *a == 0.0,
            _ => false,
        };
        assert!(ok, "state {s} with area {a}");
    }
}

#[test]
fn the_area_follows_the_counts() {
    // the area of the quantifier is close to the counts under the peak: apex x sigma x sqrt(2 pi), in counts x seconds
    let files = chromatograms(4);
    let f = detect(&files, &MrmParams::default());
    let i = (0..f.len()).find(|&i| f.q3[i] == 208.1).unwrap();
    let want = APEX * CONC[4] * 3.0 * (std::f64::consts::TAU).sqrt();
    assert!(
        (f.area[i] / want - 1.0).abs() < 0.05,
        "{} against {want}",
        f.area[i]
    );
}

#[test]
fn noise_alone_gives_no_peak() {
    let mut rng = Lcg(99);
    let t: Vec<f64> = (0..20000).map(|i| 0.001 * i as f64).collect();
    let y = t
        .iter()
        .map(|_| QUANTUM * rng.poisson(BACKGROUND / QUANTUM))
        .collect();
    let s = Srm {
        q1: 250.1,
        q3: 160.0,
        name: String::new(),
        t,
        y,
    };
    assert_eq!(detect(&[s], &MrmParams::default()).len(), 0);
}

/// Several compounds: (Q1, Q3, [(RT min, apex per unit of concentration, counts per second)]); the last transition has two isomers 0.5 min apart and
/// the weakest compound is 50 times below the quantifier above.
const MIX: [(f64, f64, &[(f64, f64)]); 7] = [
    (180.1, 110.0, &[(3.0, 8000.0)]),
    (180.1, 138.1, &[(3.0, 3000.0)]),
    (250.1, 208.1, &[(10.0, 20000.0)]),
    (250.1, 160.0, &[(10.0, 8000.0)]),
    (310.2, 192.1, &[(13.0, 400.0)]),
    (342.2, 170.1, &[(16.0, 5000.0), (16.5, 2500.0)]),
    (342.2, 296.2, &[(16.0, 2000.0)]),
];

/// The chromatograms of `MIX` in file `k`, with the counts and background of `chromatograms`.
fn mix(k: usize) -> Vec<Srm> {
    let mut rng = Lcg(31 + k as u64);
    let t: Vec<f64> = (0..20000).map(|i| 0.001 * i as f64).collect();
    MIX.iter()
        .map(|&(q1, q3, peaks)| {
            let y = t
                .iter()
                .map(|&x| {
                    let rate = BACKGROUND
                        + peaks
                            .iter()
                            .map(|&(rt, a)| a * CONC[k] * (-0.5 * ((x - rt) / 0.05).powi(2)).exp())
                            .sum::<f64>();
                    QUANTUM * rng.poisson(rate / QUANTUM)
                })
                .collect();
            Srm {
                q1,
                q3,
                name: String::new(),
                t: t.clone(),
                y,
            }
        })
        .collect()
}

#[test]
fn several_compounds_give_one_row_per_true_peak() {
    let p = MrmParams::default();
    let files: Vec<Vec<Srm>> = (0..CONC.len()).map(mix).collect();
    let feats: Vec<_> = files.iter().map(|f| detect(f, &p)).collect();
    let refs: Vec<_> = feats.iter().collect();
    let t = align(&refs, &p);
    let want: Vec<(f64, f64, f64)> = MIX
        .iter()
        .flat_map(|&(q1, q3, peaks)| peaks.iter().map(move |&(rt, _)| (q1, q3, rt)))
        .collect();
    assert_eq!(t.len(), want.len(), "eight true peaks, eight rows");
    let nf = t.n_files;
    for (q1, q3, rt) in want {
        let r = (0..t.len())
            .find(|&r| t.mz[r] == q1 && t.q3[r] == q3 && (t.rt[r] - rt).abs() < 0.05)
            .unwrap_or_else(|| panic!("no row for {q1} > {q3} at {rt}"));
        // every level above the lowest, with an area that grows with the concentration
        assert!(
            (1..6).all(|k| t.state[r * nf + k] == CELL_DETECTED),
            "{q1} > {q3} at {rt}"
        );
        assert!((1..5).all(|k| t.area[r * nf + k + 1] > t.area[r * nf + k]));
    }
}
