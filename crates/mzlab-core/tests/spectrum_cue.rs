//! The peak-width cue of the profile-label check (`spectrum_type::SHORT_POINTS`, `SHORT_SHARE`), measured on synthetic scans: zero-suppressed profiles whose
//! peaks keep only three points, full-grid profiles, and centroids under a profile label. Each file is 8 scans; `Runs` is the one detector of `raw info`,
//! the label check and `--spectrum auto`. `cargo test -p mzlab-core --test spectrum_cue -- --nocapture` prints the table behind the cut.
use mzlab_core::spectrum_type::{run_points, short_run_points, Runs, FINE_SHARE, SPARSE_BELOW};

/// xorshift64*: the same files on every machine.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        (self.0.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64 / (1u64 << 53) as f64
    }
    fn between(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.next()
    }
}

/// Ascending m/z (host-only test code: the `.wasm` never links it).
#[allow(clippy::disallowed_methods)]
fn sorted(mut v: Vec<f64>) -> Vec<f64> {
    v.sort_by(f64::total_cmp);
    v
}

const LO: f64 = 100.0;
const HI: f64 = 400.0;

/// One scan of a profile: `n` Gaussian ions of FWHM `fwhm` sampled every `step` Da; `suppressed` drops the points under 1 % of the maximum, `hits` adds
/// that share of isolated single-bin hits.
fn profile(r: &mut Rng, n: usize, step: f64, fwhm: f64, suppressed: bool, hits: f64) -> Vec<f64> {
    let sigma = fwhm / 2.3548;
    let ions: Vec<(f64, f64)> = (0..n)
        .map(|_| (r.between(LO, HI), r.between(0.5, 1.0)))
        .collect();
    let off = r.between(0.0, step);
    let grid: Vec<f64> = (0..((HI - LO) / step) as usize)
        .map(|g| LO + off + step * g as f64)
        .collect();
    let y: Vec<f64> = grid
        .iter()
        .map(|&m| {
            1e-9 + ions
                .iter()
                .map(|&(c, a)| a * (-0.5 * ((m - c) / sigma).powi(2)).exp())
                .sum::<f64>()
        })
        .collect();
    let top = y.iter().cloned().fold(0.0, f64::max);
    let mut mz: Vec<f64> = grid
        .iter()
        .zip(&y)
        .filter(|&(_, &v)| !suppressed || v > 0.01 * top)
        .map(|(&m, _)| m)
        .collect();
    for _ in 0..(hits * mz.len() as f64) as usize {
        mz.push(r.between(LO, HI));
    }
    sorted(mz)
}

fn ions(r: &mut Rng, n: usize) -> Vec<f64> {
    sorted((0..n).map(|_| r.between(LO, HI)).collect())
}

fn file(mut scan: impl FnMut() -> Vec<f64>) -> Runs {
    let mut runs = Runs::default();
    (0..8).for_each(|_| runs.add(&scan()));
    runs
}

/// What `auto` decided before the cue: sparse below `SPARSE_BELOW` in runs of four, unless a fine grid.
fn sparse_before(r: &Runs) -> bool {
    let fine = r.fine_share().is_some_and(|f| f >= FINE_SHARE);
    r.runs_share().is_some_and(|s| s < SPARSE_BELOW) && !fine
}

struct Class {
    name: &'static str,
    /// The truth: the points are a profile (not sparse).
    profile: bool,
    files: Vec<Runs>,
}

fn classes() -> Vec<Class> {
    let mut r = Rng(0x9E37_79B9_7F4A_7C15);
    let mut zero = Vec::new();
    for step in [0.4, 0.5, 0.6] {
        for fwhm in [0.6, 0.7, 0.8] {
            for n in [5, 12, 40] {
                for hits in [0.0, 0.05, 0.1] {
                    zero.push(file(|| profile(&mut r, n, step, fwhm, true, hits)));
                }
            }
        }
    }
    let mut full = Vec::new();
    for step in [0.1, 0.2, 0.5] {
        for fwhm in [0.4, 0.7] {
            for n in [5, 12, 40] {
                full.push(file(|| profile(&mut r, n, step, fwhm, false, 0.0)));
            }
        }
    }
    let mut counts = Vec::new();
    for n in [5, 12, 40, 100, 200] {
        for _ in 0..6 {
            counts.push(file(|| ions(&mut r, n)));
        }
    }
    let mut isotopes = Vec::new();
    for n in [12, 40, 100] {
        isotopes.push(file(|| {
            let b = ions(&mut r, n);
            sorted(
                b.iter()
                    .flat_map(|&m| [m, m + 1.0034, m + 2.0067])
                    .collect(),
            )
        }));
    }
    let sticks = vec![file(|| {
        (0..300)
            .map(|i| LO + f64::from(i) + r.between(-0.02, 0.02))
            .collect()
    })];
    vec![
        Class {
            name: "zero-suppressed profile, 3-4 points per peak",
            profile: true,
            files: zero,
        },
        Class {
            name: "full-grid profile",
            profile: true,
            files: full,
        },
        Class {
            name: "centroids under a profile label (random ions, 5-200 per scan)",
            profile: false,
            files: counts,
        },
        Class {
            name: "centroids with isotope trios 1 Da apart",
            profile: false,
            files: isotopes,
        },
        Class {
            name: "centroids on a 1 Da stick lattice",
            profile: false,
            files: sticks,
        },
    ]
}

/// The counts behind the cut: files read as sparse (centroids) before the cue and with it, per class. A false negative is a profile read as sparse (it keeps
/// its profile points instead of being centroided); a false positive is centroids read as a profile (centroided again, 60-70 % of the run).
#[test]
fn the_cue_frees_zero_suppressed_profiles_and_keeps_every_centroid_file_sparse() {
    let (mut fn_before, mut fn_after, mut fp_before, mut fp_after) = (0, 0, 0, 0);
    for c in classes() {
        let before = c.files.iter().filter(|f| sparse_before(f)).count();
        let after = c.files.iter().filter(|f| f.sparse()).count();
        let (lo, hi) = c.files.iter().fold((1.0f64, 0.0f64), |(lo, hi), f| {
            let s = f.short_share().unwrap();
            (lo.min(s), hi.max(s))
        });
        println!(
            "{:64} files {:3}  sparse before {:3}  with cue {:3}  short-run share {lo:.3}..{hi:.3}",
            c.name,
            c.files.len(),
            before,
            after
        );
        if c.profile {
            (fn_before, fn_after) = (fn_before + before, fn_after + after);
        } else {
            let n = c.files.len();
            (fp_before, fp_after) = (fp_before + n - before, fp_after + n - after);
        }
    }
    println!("false negatives (profile read as sparse): {fn_before} before, {fn_after} with the cue; false positives: {fp_before} before, {fp_after} with the cue");
    // the dense random files already read as a fine grid (counts closer than 50 mDa by chance), with or without the cue: the cue adds no false positive
    assert_eq!(fp_after, fp_before);
    assert_eq!(
        fn_after, 0,
        "every zero-suppressed or full-grid profile of the battery is a profile"
    );
    assert!(fn_before >= 30, "the battery holds the case the cue is for");
}

/// The limit, said as a test: three ions that happen to be 0.5 Da apart, scan after scan, are the three points of a zero-suppressed peak (the points
/// alone cannot tell them apart); the notice carries the numbers and `--centroid off` / `file convert --spectrum centroid` decide.
#[test]
fn ions_on_a_half_dalton_lattice_read_as_a_profile_and_the_share_says_so() {
    let mut r = Rng(7);
    let f = file(|| {
        let b = ions(&mut r, 20);
        sorted(b.iter().flat_map(|&m| [m, m + 0.5, m + 1.0]).collect())
    });
    assert!(!f.sparse());
    assert!(f.short_share().unwrap() > 0.6 && f.runs_share().unwrap() < 0.2);
}

/// The unit case: ten peaks of three points 0.5 Da apart have no run of four but are all short runs; ions 7.3 Da apart have neither.
#[test]
fn three_point_peaks_are_short_runs_only_and_isolated_ions_are_neither() {
    let peaks: Vec<f64> = (0..10)
        .flat_map(|p| (0..3).map(move |i| 100.0 + f64::from(p) * 7.3 + f64::from(i) * 0.5))
        .collect();
    assert_eq!(run_points(&peaks), (0, 30));
    assert_eq!(short_run_points(&peaks), (30, 30));
    let mut r = Runs::default();
    r.add(&peaks);
    assert!(!r.sparse() && r.short_share() == Some(1.0));
    let ions: Vec<f64> = (0..30).map(|i| 100.0 + f64::from(i) * 7.3).collect();
    assert_eq!(short_run_points(&ions).0, 0);
    let mut r = Runs::default();
    r.add(&ions);
    assert!(r.sparse() && r.short_share() == Some(0.0));
}
