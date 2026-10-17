//! «Fragment ions found»: the fragments of the representative MS2 scan of a feature with their neutral loss in m/z and, when it can be said, in
//! formula (`docs/agenti/mzfinder-ms2-regola.md`). Pure functions: the spectrum and the formulas come in, the list and its text go out.

use crate::sortc::SortC;
use mzlab_core::elements::{fmt, mass, parse_formula, Formula};

/// Fragments closer than this to the precursor are the precursor itself, Da.
pub const PRECURSOR_GAP: f64 = 1.5;
/// Ions listed at most.
pub const MAX_IONS: usize = 10;
/// Default relative threshold, % of the strongest fragment.
pub const MIN_REL: f64 = 5.0;
/// A neutral loss of a unit-resolution spectrum is read as a formula within this, Da.
pub const LOSS_TOL_LR: f64 = 0.3;
/// A loss from fragments with a formula is the exact difference of the formulas; without them it is read within `LOSS_TOL_LR`.
const LOSSES: [&str; 26] = [
    "H2", "H2O", "NH3", "CO", "CH2O", "CH4O", "HF", "N2", "C2H4", "CO2", "C2H2O", "C3H6", "HCN",
    "HNCO", "CH2O2", "H2S", "HCl", "NO2", "SO", "C4H8", "CH3N", "COS", "SO2", "C2H4O", "C2H6",
    "C5H8",
];

/// One fragment ion of the representative scan: m/z, intensity and, when the spectrum has it, its formula (the ion, e.g. from an HR library).
#[derive(Debug, Clone, PartialEq)]
pub struct Ion {
    pub mz: f64,
    pub intensity: f64,
    pub formula: Option<String>,
}

/// A fragment of the list.
#[derive(Debug, Clone, PartialEq)]
pub struct Found {
    pub mz: f64,
    pub intensity: f64,
    /// % of the strongest fragment of the scan.
    pub rel: f64,
    /// Precursor m/z minus fragment m/z.
    pub loss: f64,
    /// The neutral loss as a formula (`C3H6`), else `None` (written «unassigned»).
    pub loss_formula: Option<String>,
}

fn add(f: &mut Formula, el: &str, n: u64) {
    match f.iter_mut().find(|x| x.0 == el) {
        Some(x) => x.1 += n,
        None => f.push((el.to_string(), n)),
    }
}

/// Whether every element of `b` is in `a` in at least that number.
fn has(a: &Formula, b: &Formula) -> bool {
    b.iter()
        .all(|(e, n)| a.iter().any(|x| &x.0 == e && x.1 >= *n))
}

/// `a` minus `b` when `a` has all of `b`; `None` otherwise (and for an empty result).
pub fn subtract(a: &Formula, b: &Formula) -> Option<Formula> {
    if !has(a, b) {
        return None;
    }
    let out: Formula = a
        .iter()
        .map(|(e, n)| {
            (
                e.clone(),
                n - b.iter().find(|x| &x.0 == e).map_or(0, |x| x.1),
            )
        })
        .filter(|x| x.1 > 0)
        .collect();
    (!out.is_empty()).then_some(out)
}

/// The name of a loss as chemists write it (`SO2`, not the Hill order `O2S`) when it is one of the usual ones, else the Hill formula.
fn label(f: &Formula) -> String {
    let same = |a: &Formula, b: &Formula| a.len() == b.len() && has(a, b) && has(b, a);
    LOSSES
        .iter()
        .find(|s| parse_formula(s).is_ok_and(|g| same(f, &g)))
        .map_or_else(|| fmt(f), |s| (*s).to_string())
}

/// The usual neutral loss of mass `loss` made of elements that the precursor has (counts included), the closest within `tol`.
pub fn loss_formula(loss: f64, precursor: &Formula, tol: f64) -> Option<String> {
    LOSSES
        .iter()
        .filter_map(|s| parse_formula(s).ok())
        .filter(|f| has(precursor, f))
        .map(|f| ((mass(&f) - loss).abs(), f))
        .filter(|x| x.0 <= tol)
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|x| label(&x.1))
}

/// The ion formula of `[M+H]+` from the neutral formula of the row (`None` when it does not parse or is empty).
pub fn protonated(neutral: &str) -> Option<Formula> {
    let mut f = parse_formula(neutral).ok().filter(|f| !f.is_empty())?;
    add(&mut f, "H", 1);
    Some(f)
}

/// The fragments of a scan, ordered by intensity, at most `MAX_IONS`, each at least `min_rel` % of the strongest and below the precursor by more
/// than `PRECURSOR_GAP`; with their losses. `precursor` is the centre of the isolation window of the scan; `neutral` the formula of the row (or of
/// the parent when the row has none). A fragment with a formula gets the exact difference to `[M+H]+`; the others the closest usual loss within `tol`.
pub fn find(precursor: f64, neutral: &str, ions: &[Ion], min_rel: f64, tol: f64) -> Vec<Found> {
    let pf = protonated(neutral);
    let top = ions
        .iter()
        .filter(|i| precursor - i.mz > PRECURSOR_GAP)
        .map(|i| i.intensity)
        .fold(0.0, f64::max);
    let mut out: Vec<Found> = ions
        .iter()
        .filter(|i| {
            precursor - i.mz > PRECURSOR_GAP && top > 0.0 && 100.0 * i.intensity / top >= min_rel
        })
        .map(|i| {
            let loss = precursor - i.mz;
            let exact = i
                .formula
                .as_deref()
                .and_then(|s| parse_formula(s).ok())
                .zip(pf.as_ref())
                .and_then(|(f, p)| subtract(p, &f))
                .map(|d| label(&d));
            let by_mass = || pf.as_ref().and_then(|p| loss_formula(loss, p, tol));
            Found {
                mz: i.mz,
                intensity: i.intensity,
                rel: 100.0 * i.intensity / top,
                loss,
                loss_formula: exact.or_else(by_mass),
            }
        })
        .collect();
    out.sortc_by(|a, b| {
        b.intensity
            .total_cmp(&a.intensity)
            .then(a.mz.total_cmp(&b.mz))
    });
    out.truncate(MAX_IONS);
    out
}

/// «Fragment ions found: m/z X (intensity I, loss −ΔPm/z = −N.NNNN, −formula) ; …», or the sentence that says there are none.
pub fn text(found: &[Found]) -> String {
    if found.is_empty() {
        return "No fragment ions found above the threshold".into();
    }
    let parts: Vec<String> = found
        .iter()
        .map(|f| {
            let name = f
                .loss_formula
                .as_ref()
                .map_or("unassigned".to_string(), |s| format!("\u{2212}{s}"));
            format!(
                "m/z {:.2} (intensity {:.0}, loss \u{2212}\u{394}Pm/z = \u{2212}{:.4}, {name})",
                f.mz, f.intensity, f.loss
            )
        })
        .collect();
    format!("Fragment ions found: {}", parts.join(" ; "))
}
