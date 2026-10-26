//! Formula candidates for the rows formed and not explained (`tp/candidati.rs`): the true formula of a three-step product among the first three, no
//! candidate with an element the parent lacks, nothing at unit resolution, and the status columns left as they were.

use mzlab_core::elements::{ion_mz, mass, parse_formula};
use mzlab_feat::tp::{candidates, element_fit, TpBlock, ST_UNEXPLAINED};

const PARENT: &str = "C14H18N2O3S";

fn mz_of(f: &str) -> f64 {
    ion_mz(mass(&parse_formula(f).unwrap()), "[M+H]+").unwrap()
}

/// A synthetic product: the parent plus two oxygens minus CH2 (three steps of a rule list that has no such combination).
const PRODUCT: &str = "C13H16N2O5S";

/// The MS2 of the product: peaks on some of its own sub-formulas.
fn ms2_of(f: &str) -> (Vec<f64>, Vec<f64>) {
    let (_, subs) = mzlab_core::subformulas::subformulas(&format!("{f}H"), 1, 50.0).unwrap();
    let mz: Vec<f64> = subs.iter().map(|s| s.1).step_by(40).take(5).collect();
    let it = vec![100.0; mz.len()];
    (mz, it)
}

#[test]
fn true_formula_is_among_the_first_three() {
    let mz = mz_of(PRODUCT);
    let (pm, pi) = ms2_of(PRODUCT);
    for spec in [None, Some((&pm[..], &pi[..]))] {
        let c = candidates(PARENT, mz, 1, spec, None);
        assert!(!c.is_empty() && c.len() <= 3);
        assert!(c.iter().any(|x| x.neutral_formula == PRODUCT), "{c:?}");
        if spec.is_some() {
            let t = c.iter().find(|x| x.neutral_formula == PRODUCT).unwrap();
            assert!(t.ms2_explained.unwrap() > 0.99, "{t:?}");
        }
        for x in &c {
            assert!(x.isotope_fit.is_none());
            assert!((x.total - (x.element_fit + x.ms2_explained.unwrap_or(0.0))).abs() < 1e-3);
        }
    }
}

#[test]
fn an_element_the_parent_lacks_gives_no_candidate() {
    // A chlorinated mass: no formula with the parent's elements sits within 5 ppm.
    let mz = mz_of("C14H17ClN2O3S");
    assert!(candidates(PARENT, mz, 1, None, None).is_empty());
}

#[test]
fn unit_resolution_and_the_status_columns() {
    let mut b = TpBlock {
        status: vec![0, ST_UNEXPLAINED, 4],
        notes: vec![0, 1, 0],
        via: vec![1, 0, 0],
        kinship: vec![3, 2, 0],
        ..TpBlock::default()
    };
    let before = b.clone();
    b.add_fit_candidates(PARENT, 1, true, |_| (mz_of(PRODUCT), None, None));
    assert!(b.fit_candidates.is_empty());
    b.add_fit_candidates(PARENT, 1, false, |_| (mz_of(PRODUCT), None, None));
    assert_eq!(b.fit_candidates.len(), 1);
    assert_eq!(b.fit_candidates[0].0, 1);
    assert_eq!(
        (&b.status, &b.notes, &b.via, &b.kinship),
        (&before.status, &before.notes, &before.via, &before.kinship)
    );
    assert!(b.fit_of(0).is_empty() && !b.fit_of(1).is_empty());
}

#[test]
fn element_fit_is_the_larger_of_the_two_shares() {
    let p = parse_formula("C10H10O2").unwrap();
    let c = parse_formula("C5H5O").unwrap();
    assert!((element_fit(&p, &c) - 1.0).abs() < 1e-12);
    assert_eq!(element_fit(&p, &parse_formula("Cl").unwrap()), 0.0);
}

#[test]
fn decoy_masses_fit_worse_than_true_products() {
    // True products: the parent changed by a few atoms; decoys: invented masses with no relation to the parent's formula.
    let trues = [
        "C13H16N2O5S",
        "C14H18N2O4S",
        "C14H18N2O5S",
        "C13H16N2O3S",
        "C14H16N2O4S",
        "C12H14N2O3S",
        "C14H20N2O4S",
        "C13H18N2O4S",
        "C14H18N2O6S",
        "C12H16N2O4S",
    ];
    let best = |mz: f64| {
        candidates(PARENT, mz, 1, None, None)
            .iter()
            .map(|c| c.element_fit)
            .fold(0.0, f64::max)
    };
    let t: f64 = trues.iter().map(|f| best(mz_of(f))).sum::<f64>() / trues.len() as f64;
    let mut seed = 12345u64;
    let mut decoys = Vec::new();
    for _ in 0..30 {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        decoys.push(150.0 + (seed >> 40) as f64 / (1u64 << 24) as f64 * 250.0);
    }
    let d: f64 = decoys.iter().map(|&m| best(m)).sum::<f64>() / decoys.len() as f64;
    eprintln!("mean best element_fit: true {t:.3}, decoy {d:.3}");
    assert!(d < t, "decoy {d} true {t}");
}

fn pattern_of(f: &str) -> Vec<(f64, f64)> {
    let p = parse_formula(f).unwrap();
    let counts: Vec<(String, i64)> = p.into_iter().map(|(s, n)| (s, n as i64)).collect();
    mzlab_core::isoscore::fine_pattern(&counts, 1, 1.0)
        .unwrap()
        .into_iter()
        .map(|p| (p.mz, p.rel))
        .collect()
}

#[test]
fn without_isotope_data_isotope_fit_is_none_and_total_is_unaltered() {
    let mz = mz_of(PRODUCT);
    let (pm, pi) = ms2_of(PRODUCT);
    let c = candidates(PARENT, mz, 1, Some((&pm, &pi)), None);
    assert!(!c.is_empty());
    for x in &c {
        assert_eq!(x.isotope_fit, None);
        assert!((x.total - (x.element_fit + x.ms2_explained.unwrap_or(0.0))).abs() < 1e-6);
    }
}

#[test]
fn isotope_data_changes_candidate_order() {
    let mz = mz_of("C14H12O5");
    let c_no_iso = candidates(PARENT, mz, 1, None, None);
    assert_eq!(c_no_iso.len(), 2);
    assert_eq!(c_no_iso[0].neutral_formula, "C14H12O5");
    assert_eq!(c_no_iso[1].neutral_formula, "C6H16N2O7S");
    assert!(c_no_iso[0].element_fit > c_no_iso[1].element_fit);

    // Observed pattern matches the second candidate (C6H16N2O7S, ion C6H17N2O7S):
    let iso = pattern_of("C6H17N2O7S");
    let c_with_iso = candidates(PARENT, mz, 1, None, Some(&iso));
    assert_eq!(c_with_iso.len(), 2);
    assert_eq!(c_with_iso[0].neutral_formula, "C6H16N2O7S");
    assert_eq!(c_with_iso[1].neutral_formula, "C14H12O5");
    assert!(c_with_iso[0].isotope_fit.unwrap() > 0.95);
    assert!(c_with_iso[0].total > c_with_iso[1].total);
}

#[test]
fn isotope_data_keeps_candidate_order() {
    let mz = mz_of("C14H12O5");
    let c_no_iso = candidates(PARENT, mz, 1, None, None);
    assert_eq!(c_no_iso.len(), 2);
    assert_eq!(c_no_iso[0].neutral_formula, "C14H12O5");
    assert_eq!(c_no_iso[1].neutral_formula, "C6H16N2O7S");

    // Observed pattern matches the first candidate (C14H12O5, ion C14H13O5):
    let iso = pattern_of("C14H13O5");
    let c_with_iso = candidates(PARENT, mz, 1, None, Some(&iso));
    assert_eq!(c_with_iso.len(), 2);
    assert_eq!(c_with_iso[0].neutral_formula, "C14H12O5");
    assert_eq!(c_with_iso[1].neutral_formula, "C6H16N2O7S");
    assert!(c_with_iso[0].isotope_fit.unwrap() > c_with_iso[1].isotope_fit.unwrap());
    assert!(c_with_iso[0].total > c_with_iso[1].total);
}
