// Ported from Attributio chem/formula.py.
//! Masses of formulas and adduct ions, and the ppm errors that the cascade needs: the charge of a formula written as an ion, the neutral form of a permanently
//! charged structure, the m/z of an ion, the closest common adduct and `structure_fit` (does the formula of a library entry explain its own precursor?).
//!
//! The element masses and the readers of formulas and adducts (`formula_mass`, `parse_adduct`) are those of `review/adduct.rs`, which keeps the table of
//! Attributio (36 elements). `mzlab-core` has 14 elements and its masses differ from that table by up to 9·10⁻⁹ relative (F, P, S, K, Br, I and Se differ by
//! more than 10⁻⁹; H, C, N, O, Na, Cl and Si by less than 2·10⁻¹⁰), so the core table would change a ppm in the fourth decimal and drop Fe, Cu or Zn; the
//! difference is measured by `tests/attributio_formula.rs`.

pub use super::review::adduct::{formula_mass, mass_of, parse_adduct};

/// The common adducts of a polarity, in the order in which the closest one is searched.
pub fn common_adducts(pol: &str) -> &'static [&'static str] {
    match pol {
        "POS" => &[
            "[M+H]+",
            "[M+NH4]+",
            "[M+Na]+",
            "[M+K]+",
            "[M-H2O+H]+",
            "[M]+",
            "[M+2H]2+",
        ],
        "NEG" => &[
            "[M-H]-",
            "[M+Cl]-",
            "[M+HCOO]-",
            "[M+CH3COO]-",
            "[M-H2O-H]-",
            "[M-2H]2-",
        ],
        _ => &[],
    }
}

/// Library precursors are often rounded to three decimals.
pub const STRUCTURE_TOL_PPM: f64 = 20.0;

/// +1 / -1 for a formula written as a charged species (`C16H36N+`, `[C8H21NO6P]+`), else 0.
pub fn formula_charge(formula: &str) -> i32 {
    let f = formula.trim().trim_end_matches(']');
    if f.ends_with('+') {
        1
    } else if f.ends_with('-') {
        -1
    } else {
        0
    }
}

/// For a permanently charged structure (quaternary ammonium, carnitines, betaines) the mass of its conjugate neutral form (cation - H, anion + H): the
/// `[M+H]+` of that form is the cation itself, as SIRIUS and the libraries report it.
pub fn neutral_equivalent(mass: f64, formula: &str) -> f64 {
    mass - f64::from(formula_charge(formula)) * mass_of("H").unwrap_or(0.0)
}

/// The m/z of the ion of a neutral mass; `None` when the adduct cannot be read.
pub fn ion_mz(neutral_mass: f64, adduct: &str) -> Option<f64> {
    let (n, delta, charge) = parse_adduct(adduct)?;
    Some((n as f64 * neutral_mass + delta) / (charge.abs() as f64))
}

/// Error of an observed m/z against a theoretical one, in ppm.
pub fn ppm(observed: f64, theoretical: f64) -> f64 {
    (observed - theoretical) / theoretical * 1e6
}

/// `(adduct, ppm)` of the common adduct of `pol` closest to the observed m/z; `("", inf)` when none can be computed.
pub fn best_common_adduct(mz: f64, neutral_mass: f64, pol: &str) -> (String, f64) {
    let mut best = (String::new(), f64::INFINITY);
    for a in common_adducts(pol) {
        if let Some(th) = ion_mz(neutral_mass, a).filter(|t| *t != 0.0) {
            let e = ppm(mz, th);
            if e.abs() < best.1.abs() {
                best = ((*a).to_string(), e);
            }
        }
    }
    best
}

/// `(ppm, ok)` of a library entry's structure against its own precursor m/z: `ok` is true when the declared adduct, or for a mislabelled adduct a common one,
/// explains it within `tol_ppm`; `None` when it cannot be checked (no formula, no precursor). The mzMine library needs it because its structure columns can
/// belong to another entry (a known bug of the export). The ppm is NaN when nothing could be computed.
pub fn structure_fit(
    formula: Option<&str>,
    adduct: Option<&str>,
    precursor: Option<f64>,
    pol: &str,
    tol_ppm: f64,
) -> (f64, Option<bool>) {
    let (Some(formula), Some(p)) = (formula, precursor.filter(|p| !p.is_nan())) else {
        return (f64::NAN, None);
    };
    let Some(m) = formula_mass(formula) else {
        return (f64::NAN, None);
    };
    let q = formula_charge(formula);
    let adduct = adduct.unwrap_or("nan");
    let e = if q != 0 && matches!(adduct.trim(), "[M]" | "M" | "[M]+" | "[M]-" | "[M-e]+") {
        // the charged species itself
        ion_mz(m, if q > 0 { "[M]+" } else { "[M]-" }).map_or(f64::NAN, |t| ppm(p, t))
    } else {
        let n = neutral_equivalent(m, formula);
        let e = ion_mz(n, adduct)
            .filter(|t| *t != 0.0)
            .map_or(f64::NAN, |t| ppm(p, t));
        if !(e.abs() <= tol_ppm) {
            let (_, e2) = best_common_adduct(p, n, pol);
            if e2.abs() <= tol_ppm {
                return (e2, Some(true));
            }
        }
        e
    };
    (e, (!e.is_nan()).then(|| e.abs() <= tol_ppm))
}
