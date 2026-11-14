//! MEASURE (no default is changed): can the row isotope score (`evid::producers::isotope_score`, M+1/M only) tell a halogenated or sulfur compound from an
//! isobaric CHNO formula (within 5 ppm)? Every CHNO formula in range is scored against the observed pattern of the true compound with the row score and with
//! `mzlab_core::isoscore` (M, M+1, M+2, ...). Numbers: `docs/agenti/mzfeat-prove.md` (MISURA). Synthetic, no real data.

use mzlab_core::composition::{isotope_ratios, Counts};
use mzlab_core::elements::{mass, parse_formula, C13_STEP, PROTON};
use mzlab_core::isoscore::{fine_pattern, isotope_score, merge_res, IsoOpts};
use mzlab_feat::evid::producers::{isotope_score as row_score, FeatIndex};
use mzlab_feat::table::FileFeatures;

/// True compounds: Cl2, Cl3, Br, Br2, S, S2, Cl+S and a CHNO control.
const TRUE: [&str; 8] = [
    "C12H10Cl2N2O",
    "C10H8Cl3NO2",
    "C9H9BrN2O2",
    "C8H6Br2N2O",
    "C12H15N3O3S",
    "C10H12N2O2S2",
    "C13H14ClN3O3S",
    "C15H20N4O3",
];

fn ion(neutral: &str) -> Counts {
    let mut c: Counts = parse_formula(neutral)
        .unwrap()
        .into_iter()
        .map(|(e, k)| (e, k as i64))
        .collect();
    c.iter_mut().find(|x| x.0 == "H").unwrap().1 += 1;
    c
}

fn neutral_mass(c: &str) -> f64 {
    mass(&parse_formula(c).unwrap())
}

/// CHNO neutral formulas whose [M+H]+ is within 5 ppm of the one of `target` (hydrogens chosen to the nearest mass).
fn isobaric(target: &str) -> Vec<String> {
    let t = neutral_mass(target);
    let mut out = vec![];
    for c in 3..=30u32 {
        for n in 0..=6u32 {
            for o in 0..=8u32 {
                let base = f64::from(c) * 12.0
                    + f64::from(n) * neutral_mass("N")
                    + f64::from(o) * neutral_mass("O");
                let h = ((t - base) / neutral_mass("H")).round();
                if h < 1.0 || h > f64::from(2 * c + n + 2) {
                    continue;
                }
                let f = format!("C{c}H{h}N{n}O{o}")
                    .replace("N0", "")
                    .replace("O0", "");
                if ((neutral_mass(&f) - t) / t).abs() <= 5e-6 && f != target {
                    out.push(f);
                }
            }
        }
    }
    out
}

/// The pattern the instrument shows for the true compound: its fine structure merged at the resolution of the score.
fn observed(truth: &str) -> Vec<(f64, f64)> {
    let fine = fine_pattern(&ion(truth), 1, 0.001).unwrap();
    merge_res(fine, opts().r200)
        .iter()
        .map(|p| (p.mz, p.rel))
        .collect()
}

/// A file holding the light ion and its M+1 as the true compound would show them.
fn file(truth: &str, mz: f64) -> FileFeatures {
    let m1 = isotope_ratios(&ion(truth)).0;
    let mut f = FileFeatures::default();
    for (m, a) in [(mz, 1e6), (mz + C13_STEP, 1e6 * m1)] {
        f.mz.push(m);
        f.rt.push(5.0);
        f.area_win.push(a);
    }
    f
}

/// The options of the HR target screening (`mzlab-target`).
fn opts() -> IsoOpts<'static> {
    IsoOpts {
        r200: 20000.0,
        sigma_ppm: 5.0,
        sigma_int: 0.25,
        min_rel: 1.0,
        noise_rel: 0.1,
        norm: "mono",
    }
}

#[derive(Debug)]
struct Fit {
    n: usize,
    row: (f64, f64),
    core: (f64, f64),
}

fn measure(truth: &str) -> Fit {
    let mz = neutral_mass(truth) + PROTON;
    let f = file(truth, mz);
    let idx = [FeatIndex::new(&f)];
    let obs = observed(truth);
    let o = opts();
    let mut fit = Fit {
        n: 0,
        row: (2.0, -1.0),
        core: (2.0, -1.0),
    };
    for c in isobaric(truth) {
        let (Some(r), Ok(k)) = (
            row_score(&idx, mz, 5.0, &c, 1),
            isotope_score(&ion(&c), 1, &obs, &o),
        ) else {
            continue;
        };
        fit.n += 1;
        fit.row = (fit.row.0.min(r), fit.row.1.max(r));
        fit.core = (fit.core.0.min(k.score), fit.core.1.max(k.score));
    }
    fit
}

/// Row and core score of the true formula itself (the control of the harness: both must be about 1).
fn true_scores(truth: &str) -> (f64, f64) {
    let mz = neutral_mass(truth) + PROTON;
    let f = file(truth, mz);
    let obs = observed(truth);
    let r = row_score(&[FeatIndex::new(&f)], mz, 5.0, truth, 1).unwrap();
    (
        r,
        isotope_score(&ion(truth), 1, &obs, &opts()).unwrap().score,
    )
}

/// Pinned: per true formula (isobaric CHNO formulas within 5 ppm, row score min-max, core score min-max). The row score of the isobaric formulas reaches
/// 0.99 for the S2 compound while the core score (which sees M+2) reaches 0.38 for it; Cl3 and Br2 have no isobaric CHNO formula in the searched range.
#[test]
fn row_score_is_blind_to_m2() {
    let want = [
        ("C12H10Cl2N2O", 1, (0.706, 0.706), (0.158, 0.158)),
        ("C10H8Cl3NO2", 0, (2.0, -1.0), (2.0, -1.0)),
        ("C9H9BrN2O2", 1, (0.817, 0.817), (0.309, 0.309)),
        ("C8H6Br2N2O", 0, (2.0, -1.0), (2.0, -1.0)),
        ("C12H15N3O3S", 3, (0.335, 0.721), (0.209, 0.321)),
        ("C10H12N2O2S2", 2, (0.768, 0.987), (0.291, 0.384)),
        ("C13H14ClN3O3S", 3, (0.621, 0.757), (0.161, 0.264)),
        ("C15H20N4O3", 2, (0.851, 0.941), (0.524, 0.659)),
    ];
    for (t, n, row, core) in want {
        let (r1, c1) = true_scores(t);
        assert!(
            r1 > 0.99 && c1 > 0.9,
            "harness: the true formula fits {t}: {r1} {c1}"
        );
        let m = measure(t);
        assert_eq!(m.n, n, "{t}");
        for (got, w) in [(m.row, row), (m.core, core)] {
            assert!(
                (got.0 - w.0).abs() < 0.01 && (got.1 - w.1).abs() < 0.01,
                "{t}: {got:?} vs {w:?}"
            );
        }
    }
}
