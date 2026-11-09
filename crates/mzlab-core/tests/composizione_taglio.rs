//! The element ranges cut to the ion mass lose nothing: on every grid that runs without the cut, the answer of `compose` is the same text
//! byte for byte (candidates, counts, order, roundings), over ions of both signs, charges 1 to 3, isotope labels, adduct atoms, minima above zero
//! and tolerances in ppm and mDa. A grid still over the limit names its size and the limit. Synthetic m/z values only.

use mzlab_core::composition::{compose, parse_elements, Opts};
use mzlab_core::Result;

/// The answer without the cut of the ranges to the ion mass, then with it.
fn both(mz: f64, mut o: Opts) -> (Result<String>, Result<String>) {
    o.uncut = true;
    let before = compose(mz, &o);
    o.uncut = false;
    (before, compose(mz, &o))
}

const SETS: [&str; 6] = [
    "C:0-40,H:0-80,N:0-6,O:0-12,F:0-30,Cl:0-6,Br:0-4",
    "C:0-30,H:0-60,13C:0-3,2H:0-5,15N:0-2,O:0-10,S:0-4,Si:0-6,P:0-3",
    "C:0-20,H:0-40,Na:0-2,K:0-2,I:0-5,37Cl:0-4,81Br:0-3,34S:0-3,18O:0-3",
    "C:2-20,H:1-40,N:1-4,O:2-8,F:1-60,Cl:0-20",
    "N:0-50,O:0-50,S:0-50",
    "",
];
const IONS: [&str; 11] = [
    "[M+H]+",
    "[M+NH4]+",
    "[M+Na]+",
    "[M+K]+",
    "[M-H]-",
    "[M+Cl]-",
    "[M+HCOO]-",
    "M+.",
    "M-.",
    "[M]+",
    "[M]-",
];

fn opts<'a>(set: &str, ion: &'a str, z: Option<i32>, tol: f64, unit: &'a str) -> Opts<'a> {
    Opts {
        elements: (!set.is_empty()).then(|| parse_elements(set).unwrap()),
        ion,
        z,
        tol,
        unit,
        max_results: 25,
        ..Opts::default()
    }
}

/// Every charge (none, 2, 3) with every tolerance (ppm and mDa, narrow and broad): two of these per element set, m/z and ion, in turn.
const STATES: [(Option<i32>, f64, &str); 12] = [
    (None, 2.0, "ppm"),
    (None, 50.0, "ppm"),
    (None, 5.0, "mda"),
    (None, 50.0, "mda"),
    (Some(2), 2.0, "ppm"),
    (Some(2), 50.0, "ppm"),
    (Some(2), 5.0, "mda"),
    (Some(2), 50.0, "mda"),
    (Some(3), 2.0, "ppm"),
    (Some(3), 50.0, "ppm"),
    (Some(3), 5.0, "mda"),
    (Some(3), 50.0, "mda"),
];

#[test]
fn the_cut_to_the_ion_mass_changes_no_answer() {
    let (mut ran, mut found, mut i) = (0, 0, 0);
    for set in SETS {
        for mz in [75.51, 181.0707, 455.1234, 1210.4] {
            for ion in IONS {
                for k in [i * 5 % 12, (i * 5 + 7) % 12] {
                    let (z, tol, unit) = STATES[k];
                    // heavy ions are slow in a debug build: the default set leaves C and H open up to the ion mass, and any set above 1300
                    let t = mz * f64::from(z.unwrap_or(1));
                    if t > 1300.0 || (set.is_empty() && t > 600.0) {
                        continue;
                    }
                    let (Ok(before), after) = both(mz, opts(set, ion, z, tol, unit)) else {
                        continue;
                    };
                    let after = after.unwrap();
                    assert_eq!(before, after, "{set} {mz} {ion} {z:?} {tol} {unit}");
                    ran += 1;
                    found += usize::from(!before.contains("\"n_candidates\":0"));
                }
                i += 1;
            }
        }
    }
    // the cases are not all empty answers: most grids find candidates
    assert!(ran > 300 && found > ran / 3, "{ran} {found}");
}

#[test]
fn the_cut_keeps_the_scored_answer_and_the_parent() {
    // the isotope fit (observed peaks) and a parent formula go through the same grid
    for (mz, parent) in [
        (181.070665, None),
        (181.070665, Some("C6H12O6")),
        (300.1, Some("C12F10Cl2")),
    ] {
        let mut o = opts(SETS[0], "[M+H]+", None, 20.0, "ppm");
        o.parent = parent;
        o.obs = vec![(mz, 100.0), (mz + 1.00336, 6.6), (mz + 2.0042, 1.2)];
        let (before, after) = both(mz, o);
        assert_eq!(before.unwrap(), after.unwrap(), "{parent:?}");
    }
}

#[test]
fn a_grid_wider_than_the_mass_now_runs() {
    // F, Cl and Br up to far more atoms than an ion of m/z 300 can hold: 3.7 million rows before the cut, a few hundred after
    let o = opts(
        "C:0-30,H:0-60,F:0-200,Cl:0-50,Br:0-50,Si:0-6",
        "[M+H]+",
        None,
        5.0,
        "ppm",
    );
    let (before, after) = both(300.0, o);
    assert_eq!(before.unwrap_err().key, "err.comp.many");
    assert!(after.is_ok());
}

#[test]
fn a_grid_still_over_the_limit_names_its_size_and_the_limit() {
    // N, O and F up to 150 fit in an ion of m/z 3000: the cut leaves 151^3 rows
    let o = opts("N:0-150,O:0-150,F:0-150", "[M+H]+", None, 5.0, "ppm");
    let e = compose(3000.0, &o).unwrap_err();
    assert_eq!(e.key, "err.comp.many");
    let p = |k: &str| {
        e.params
            .iter()
            .find(|(n, _)| *n == k)
            .map(|(_, v)| v.as_str())
    };
    assert_eq!(p("n"), Some("3442951"));
    assert_eq!(p("max"), Some("3000000"));
}
