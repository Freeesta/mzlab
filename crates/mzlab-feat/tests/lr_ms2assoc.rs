//! The rule that ties an MS2 scan to a feature (`lr::ms2assoc`) and the losses of «Fragment ions found» (`lr::ms2frag`), on tables of exact values.
//! Precursor 333.0 with an isolation window of 1.0 Da (half 0.5) unless a case says otherwise; the feature peak is 15.30..15.50 min, apex 15.39.

use mzlab_core::elements::{mass, parse_formula};
use mzlab_feat::legacy::lr::ms2assoc::{
    associate, delta, fits, overlap, tolerance, Feature, Ms2Scan,
};
use mzlab_feat::legacy::lr::ms2frag::{find, loss_formula, protonated, text, Ion, LOSS_TOL_LR};

const TABLE_TOL: f64 = 0.5;

fn feature(mz: f64) -> Feature {
    Feature {
        mz,
        apex: 15.39,
        lo: 15.30,
        hi: 15.50,
    }
}

fn scan(rt: f64, precursor: f64, intensity: f64) -> Ms2Scan {
    Ms2Scan {
        rt,
        precursor,
        half_window: Some(0.5),
        intensity,
    }
}

#[test]
fn tolerance_is_the_half_window_else_the_table_and_never_below_the_ppm_floor() {
    let t = tolerance(Some(0.5), TABLE_TOL, 333.0);
    assert_eq!((t.da, t.from_window), (0.5, true));
    let t = tolerance(Some(0.35), TABLE_TOL, 333.0);
    assert_eq!((t.da, t.from_window), (0.35, true));
    let t = tolerance(None, TABLE_TOL, 333.0);
    assert_eq!((t.da, t.from_window), (0.5, false));
    // a window of 0 or a nonsense one is not a window
    assert!(!tolerance(Some(0.0), TABLE_TOL, 333.0).from_window);
    assert!(!tolerance(Some(f64::NAN), TABLE_TOL, 333.0).from_window);
    // the floor: 10 ppm of 333.0 = 0.00333 Da; a table tolerance below it is raised to it
    assert!((tolerance(None, 0.001, 333.0).da - 0.00333).abs() < 1e-9);
}

#[test]
fn a_precursor_at_333_0_with_a_window_of_1_0_is_not_the_ion_of_a_feature_at_332_38() {
    // |333.0 - 332.38| = 0.62 Da > 0.5 Da (half of 1.0): not associated, whatever the time
    let (da, ppm) = delta(333.0, 332.38);
    assert!(
        (da - 0.62).abs() < 1e-9 && (ppm - 1865.4).abs() < 0.1,
        "{da} {ppm}"
    );
    assert!(fits(&feature(332.38), &scan(15.39, 333.0, 1.0e4), TABLE_TOL).is_none());
    assert!(associate(&feature(332.38), &[scan(15.39, 333.0, 1.0e4)], TABLE_TOL).is_none());
    // the ones of the screenshot that are close enough stay: 332.87 and 332.88 are 0.13 and 0.12 Da from 333.0
    assert!(fits(&feature(332.87), &scan(15.39, 333.0, 1.0e4), TABLE_TOL).is_some());
    assert!(fits(&feature(332.88), &scan(15.39, 333.0, 1.0e4), TABLE_TOL).is_some());
}

#[test]
fn a_feature_at_333_03_with_the_scan_inside_its_peak_is_associated_with_its_offsets() {
    let a = associate(&feature(333.03), &[scan(15.41, 333.0, 2.0e4)], TABLE_TOL).unwrap();
    assert_eq!((a.rep, a.n_in), (0, 1));
    assert!(
        (a.dmz - 0.03).abs() < 1e-9 && (a.dppm - 90.1).abs() < 0.1,
        "{} {}",
        a.dmz,
        a.dppm
    );
    assert!((a.drt - 0.02).abs() < 1e-9);
    assert_eq!((a.tol.da, a.tol.from_window), (0.5, true));
}

#[test]
fn a_scan_outside_the_peak_of_the_feature_is_not_associated_even_if_it_is_close_to_it() {
    for rt in [15.29, 15.51, 1.92, 18.60] {
        assert!(
            associate(&feature(333.03), &[scan(rt, 333.0, 2.0e4)], TABLE_TOL).is_none(),
            "{rt}"
        );
    }
    // the start and the end of the peak belong to it
    assert!(associate(&feature(333.03), &[scan(15.30, 333.0, 1.0)], TABLE_TOL).is_some());
    assert!(associate(&feature(333.03), &[scan(15.50, 333.0, 1.0)], TABLE_TOL).is_some());
}

#[test]
fn of_several_scans_in_the_peak_the_most_intense_one_represents_them_and_the_others_are_counted() {
    let scans = [
        scan(15.32, 333.0, 1.0e4),
        scan(15.40, 333.0, 5.0e4),
        scan(15.44, 333.0, 3.0e4),
        scan(16.00, 333.0, 9.0e4), // outside the peak: never the representative
        scan(15.46, 340.0, 8.0e4), // another precursor
    ];
    let a = associate(&feature(333.03), &scans, TABLE_TOL).unwrap();
    assert_eq!((a.rep, a.n_in), (1, 3));
    assert!((a.drt - 0.01).abs() < 1e-9);
    // the same intensity: the one closest to the apex (15.39)
    let tie = [
        scan(15.31, 333.0, 7.0),
        scan(15.41, 333.0, 7.0),
        scan(15.49, 333.0, 7.0),
    ];
    assert_eq!(associate(&feature(333.03), &tie, TABLE_TOL).unwrap().rep, 1);
}

#[test]
fn a_scan_without_a_window_uses_the_table_tolerance_and_says_so() {
    let s = Ms2Scan {
        half_window: None,
        ..scan(15.40, 333.45, 1.0)
    };
    let a = associate(&feature(333.03), &[s], TABLE_TOL).unwrap();
    assert_eq!((a.tol.da, a.tol.from_window), (0.5, false));
    let s = Ms2Scan {
        half_window: Some(0.35),
        ..scan(15.40, 333.45, 1.0)
    };
    assert!(
        associate(&feature(333.03), &[s], TABLE_TOL).is_none(),
        "0.42 Da > half of a 0.7 Da window"
    );
}

#[test]
fn overlap_of_the_trace_peak_and_the_feature_peak() {
    assert_eq!(overlap(15.2, 15.6, 15.3, 15.5), Some((15.3, 15.5)));
    assert_eq!(overlap(15.45, 15.9, 15.3, 15.5), Some((15.45, 15.5)));
    assert_eq!(overlap(15.6, 15.9, 15.3, 15.5), None);
}

fn flufenacet() -> &'static str {
    // C14H13F4N3O2S, a public formula; [M+H]+ 364.0737
    "C14H13F4N3O2S"
}

fn ions(v: &[(f64, f64)]) -> Vec<Ion> {
    v.iter()
        .map(|(mz, intensity)| Ion {
            mz: *mz,
            intensity: *intensity,
            formula: None,
        })
        .collect()
}

#[test]
fn the_protonated_formula_of_flufenacet_is_364_0737() {
    let p = protonated(flufenacet()).unwrap();
    assert!(
        (mass(&p) - 1.007276 - 363.0665).abs() < 0.001 + 0.0006,
        "{}",
        mass(&p)
    );
    assert!((mass(&p) - 364.0738).abs() < 0.001, "{}", mass(&p));
}

#[test]
fn losses_of_flufenacet_are_read_as_formulas_from_its_elements() {
    let p = protonated(flufenacet()).unwrap();
    // 364.0737 - 322.0267 = 42.0470 = C3H6 (the isopropyl); 20.0062 = HF; 56.0626 = C4H8; 63.9619 = SO2
    for (loss, want) in [
        (42.0470, "C3H6"),
        (20.0062, "HF"),
        (56.0626, "C4H8"),
        (63.9619, "SO2"),
        (18.0106, "H2O"),
    ] {
        assert_eq!(
            loss_formula(loss, &p, LOSS_TOL_LR).as_deref(),
            Some(want),
            "{loss}"
        );
    }
    // a loss of mass 100.0 is none of the usual ones; and a loss made of an element the molecule does not have is not offered (HCl for flufenacet)
    assert_eq!(loss_formula(100.0, &p, LOSS_TOL_LR), None);
    assert_eq!(loss_formula(35.9767, &p, LOSS_TOL_LR), None);
    // the elements are counted: C20H40 is not a loss of a C14 molecule, SO2 is not one of a molecule with one O
    let p1 = protonated("C3H8OS").unwrap();
    assert_eq!(loss_formula(63.9619, &p1, LOSS_TOL_LR), None);
}

#[test]
fn the_fragments_of_the_precursor_364_0737_with_their_losses_in_mz_and_in_formula() {
    let spectrum = ions(&[
        (364.0737, 9.0e5), // the precursor: never a fragment
        (322.0267, 8.0e4), // -C3H6
        (344.0675, 3.0e4), // -HF
        (308.0111, 1.0e4), // -C4H8
        (212.1, 5.0e3),    // loss 151.97: no usual loss
        (100.0, 100.0),    // under the threshold (0.1 %)
    ]);
    let found = find(364.0737, flufenacet(), &spectrum, 5.0, LOSS_TOL_LR);
    let got: Vec<(f64, Option<&str>)> = found
        .iter()
        .map(|f| (f.mz, f.loss_formula.as_deref()))
        .collect();
    assert_eq!(
        got,
        [
            (322.0267, Some("C3H6")),
            (344.0675, Some("HF")),
            (308.0111, Some("C4H8")),
            (212.1, None)
        ]
    );
    assert!((found[0].loss - 42.0470).abs() < 1e-9 && found[0].rel == 100.0);
    let t = text(&found);
    assert_eq!(
        t,
        "Fragment ions found: m/z 322.03 (intensity 80000, loss \u{2212}\u{394}Pm/z = \u{2212}42.0470, \u{2212}C3H6) ; \
         m/z 344.07 (intensity 30000, loss \u{2212}\u{394}Pm/z = \u{2212}20.0062, \u{2212}HF) ; \
         m/z 308.01 (intensity 10000, loss \u{2212}\u{394}Pm/z = \u{2212}56.0626, \u{2212}C4H8) ; \
         m/z 212.10 (intensity 5000, loss \u{2212}\u{394}Pm/z = \u{2212}151.9737, unassigned)"
    );
}

#[test]
fn at_most_ten_ions_the_threshold_is_relative_and_fragments_with_a_formula_get_the_exact_difference(
) {
    let many: Vec<Ion> = (0..15)
        .map(|i| Ion {
            mz: 100.0 + 10.0 * f64::from(i),
            intensity: 1000.0 - f64::from(i),
            formula: None,
        })
        .collect();
    assert_eq!(
        find(364.0737, flufenacet(), &many, 0.0, LOSS_TOL_LR).len(),
        10
    );
    // 50 %: only the ones from half of the strongest
    let two = ions(&[(300.0, 1000.0), (200.0, 500.0), (150.0, 499.0)]);
    assert_eq!(
        find(364.0737, flufenacet(), &two, 50.0, LOSS_TOL_LR).len(),
        2
    );
    // a fragment with its ion formula: the loss is the difference of the formulas, not read from the mass
    let f = Ion {
        mz: 322.0267,
        intensity: 1.0,
        formula: Some("C11H8F4N3O2S".into()),
    };
    let found = find(364.0737, flufenacet(), &[f], 5.0, 0.0001);
    assert_eq!(found[0].loss_formula.as_deref(), Some("C3H6"));
    assert!(parse_formula("C3H6").is_ok());
    assert_eq!(text(&[]), "No fragment ions found above the threshold");
}
