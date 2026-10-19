// Ported from Attributio reglists/amenability.py.
//! Can the method see this compound? One set of generic rules for every list (SPE on reversed phase, RP C18 water/acetonitrile gradient, ESI +/-): structure not
//! resolved, fumigants and gases, permanent cations, mass window, no carbon, heavy metals, salts, no N/O/S/P, very polar and perfluorinated entries. The MS1 window
//! of an instrument is not here. The RDKit-only helpers of the reference tool (`detection_form`, `structure_from_smiles`) need InChIKeys and are not ported: the
//! builder takes those fields from the source.

use crate::attributio::annotate::text::fmt_g;
use crate::chem::grafo::Mol;
use mzlab_core::elements;

pub const MASS_MIN: f64 = 50.0;
pub const MASS_MAX: f64 = 1500.0;

const METALS: [&str; 29] = [
    "Hg", "As", "Pb", "Sn", "Cd", "Cr", "Ni", "Mn", "Co", "Cu", "Zn", "Fe", "Ag", "Pt", "Au", "Ba",
    "Be", "Sb", "Se", "Tl", "Ti", "Zr", "Al", "Bi", "Mo", "W", "V", "Ge", "Te",
];
const COUNTERION_METALS: [&str; 6] = ["Na", "K", "Li", "Ca", "Mg", "NH4"];
/// Fumigants and gases (GC), by InChIKey first block.
const FUMIGANTS: [(&str, &str); 13] = [
    ("GZUXJHMPEANEGY", "methyl bromide (fumigant, GC)"),
    ("LFHISGNCFUNFFM", "chloropicrin (fumigant, GC)"),
    ("UOORRWUZONOOLO", "1,3-dichloropropene (fumigant, GC)"),
    ("HEDRZPFGACZZDS", "chloroform (volatile, GC)"),
    ("BLTXWCKMNMYXSF", "sulfuryl fluoride (gas, GC)"),
    ("XSCHRSMBECNVNS", "phosphine (inorganic gas)"),
    ("IAYPIBMASNFSPL", "ethylene oxide (gas, GC)"),
    ("QGJOPFRUJISHPQ", "carbon disulfide (volatile, GC)"),
    ("BVKMAKIKAIUGRJ", "methyl isothiocyanate (fumigant, GC)"),
    ("OYLRFHLPEAGKJU", "1,2-dichloropropane (fumigant, GC)"),
    ("XQRLCLUYWUNEEH", "ethylene dichloride (fumigant, GC)"),
    ("KWKAKUADMBZCLK", "1,2-dibromoethane (fumigant, GC)"),
    ("PAAZPARNPHGIKF", "dibromide (fumigant, GC)"),
];
const PERMANENT_CATIONS: [(&str, &str); 7] = [
    ("INFDPOAKFNIJBF", "paraquat (dicationic bipyridyl)"),
    (
        "FIKAKWIAUPDISJ",
        "paraquat dichloride (dicationic bipyridyl)",
    ),
    ("AQSBUEOAZIZKSA", "diquat (dicationic bipyridyl)"),
    ("ODPOAESBSUKMHD", "diquat dibromide (dicationic bipyridyl)"),
    ("SKYNPRKUXHXZFJ", "diquat dichloride (dicationic bipyridyl)"),
    ("FIAFUQMPZJWCLV", "chlormequat (quaternary ammonium)"),
    ("MJPCNSLOWPCAHP", "mepiquat (quaternary ammonium)"),
];

/// Whether the symbol appears in the formula as an element (`Cl` is not `C`, `Na` is not `N`).
pub fn element_in(formula: &str, symbol: &str) -> bool {
    let mut from = 0;
    while let Some(o) = formula[from..].find(symbol) {
        let i = from + o;
        let next = formula[i + symbol.len()..].chars().next();
        if next.is_none_or(|c| c.is_ascii_digit() || c.is_uppercase() || c == '(' || c == ')') {
            return true;
        }
        from = i + 1;
    }
    false
}

/// The first symbol of `list` found in the formula as an element (digit, uppercase or end after it).
fn find_symbol<'a>(formula: &str, list: &[&'a str]) -> Option<&'a str> {
    for m in list {
        let mut from = 0;
        while let Some(o) = formula[from..].find(m) {
            let i = from + o;
            let next = formula[i + m.len()..].chars().next();
            if next.is_none_or(|c| c.is_ascii_digit() || c.is_uppercase()) {
                return Some(m);
            }
            from = i + 1;
        }
    }
    None
}

/// The first heavy metal in the formula.
pub fn has_metal(formula: &str) -> Option<&'static str> {
    find_symbol(formula, &METALS)
}

/// A carbon atom in the formula (not Cl, Ca, Cd, Co, Cr, Cu, Cs).
pub fn has_carbon(formula: &str) -> bool {
    let f = ["Cl", "Ca", "Cd", "Co", "Cr", "Cu", "Cs"]
        .iter()
        .fold(formula.to_string(), |f, s| f.replace(s, ""));
    element_in(&f, "C")
}

fn has_counterion(formula: &str) -> Option<&'static str> {
    find_symbol(formula, &COUNTERION_METALS)
}

/// The fragment of a multicomponent SMILES with most letters (then longest); the first one wins a tie.
fn largest_organic_fragment(smiles: &str) -> Option<&str> {
    let mut best: Option<(&str, (usize, usize))> = None;
    for p in smiles.split('.').filter(|p| !p.is_empty()) {
        let key = (
            p.chars().filter(|c| c.is_alphabetic()).count(),
            p.chars().count(),
        );
        if best.is_none_or(|(_, k)| key > k) {
            best = Some((p, key));
        }
    }
    best.map(|(p, _)| p)
}

/// Elements present in a SMILES fragment (only for «contains N/O/S/P»).
fn formula_from_smiles(smiles: &str) -> String {
    let mut out = String::new();
    let tmp = smiles.replace("Cl", "").replace("Br", "").replace("Si", "");
    for sym in ["Cl", "Br", "Si", "N", "O", "S", "P", "F", "I", "C"] {
        if matches!(sym, "Cl" | "Br" | "Si") {
            if smiles.contains(sym) {
                out.push_str(sym);
            }
        } else if tmp.contains(sym) || tmp.contains(&sym.to_lowercase()) {
            out.push_str(sym);
        }
    }
    out
}

/// Quaternary N/P/S not compensated by an internal anion on O/N/S.
fn permanent_cation(frag: &str) -> i64 {
    let n = |p: &str| frag.matches(p).count() as i64;
    let pos = n("[N+]") + n("[n+]") + n("[P+]") + n("[S+]");
    let neg = n("[O-]") + n("[N-]") + n("[S-]");
    if pos > neg {
        pos - neg
    } else {
        0
    }
}

/// The exact mass of a SMILES fragment from the molecular graph of mzLab (RDKit's `ExactMolWt` in the reference tool).
fn fragment_mass(frag: &str) -> Option<f64> {
    let m = Mol::parse(frag).ok()?;
    let f: elements::Formula = m.formula().into_iter().collect();
    Some(elements::mass(&f))
}

fn cation_rule(q: i64, mono_mass: Option<f64>, frag: Option<&str>) -> (&'static str, String) {
    let mm = mono_mass.or_else(|| frag.and_then(fragment_mass));
    if q >= 2 || mm.is_some_and(|m| m <= 200.0) {
        (
            "false",
            format!("small or multiple permanent cation (+{q}): not retained in RP"),
        )
    } else {
        (
            "true",
            format!("lipophilic permanent cation (+{q}): detected as M+ in ESI(+)"),
        )
    }
}

fn core_rules(
    smiles: &str,
    formula: Option<&str>,
    charge: Option<i64>,
    xlogp: Option<f64>,
    mono_mass: Option<f64>,
) -> (&'static str, String) {
    let f = formula
        .filter(|f| !f.is_empty())
        .map_or_else(|| formula_from_smiles(smiles), str::to_string);
    let q_struct = permanent_cation(smiles);
    if q_struct != 0 {
        return cation_rule(q_struct, mono_mass, None);
    }
    let q = charge.unwrap_or(0);
    if q > 0 && !smiles.contains('.') {
        return cation_rule(q, mono_mass, None);
    }
    if q < 0 && !smiles.contains('.') {
        return (
            "false",
            format!("net permanent charge {q:+} (anionic), not retained in RP"),
        );
    }
    if !["N", "O", "S", "P"].iter().any(|e| element_in(&f, e)) {
        return if ["F", "Cl", "Br", "I"].iter().any(|e| element_in(&f, e)) {
            (
                "false",
                "organohalogen without N/O/S/P: GC-EI or APCI, not ESI".into(),
            )
        } else {
            (
                "false",
                "hydrocarbon without heteroatoms: not ionizable in ESI".into(),
            )
        };
    }
    if let Some(lp) = xlogp.filter(|v| *v <= -2.0) {
        let small = mono_mass.is_some_and(|m| m < 300.0);
        if element_in(&f, "P") && small {
            return ("false", format!("very polar/amphoteric (XLogP3 {}, phosphonic acid, < 300 Da): not retained in RP", fmt_g(lp)));
        }
        if small
            && element_in(&f, "N")
            && (smiles.contains("C(=O)O")
                || smiles.contains("[O-]")
                || smiles.contains("S(=O)(=O)O"))
        {
            return (
                "false",
                format!(
                    "small amphoteric (XLogP3 {}, < 300 Da): not retained in RP",
                    fmt_g(lp)
                ),
            );
        }
        return (
            "doubtful",
            format!("very polar (XLogP3 {}): RP retention uncertain", fmt_g(lp)),
        );
    }
    if element_in(&f, "F")
        && f.matches('F').count() > 0
        && !["C(=O)O", "S(=O)(=O)O", "N", "OC", "O="]
            .iter()
            .any(|g| smiles.contains(g))
    {
        return (
            "doubtful",
            "polyfluorinated without clear ionizable group".into(),
        );
    }
    (
        "true",
        "neutral organic molecule with ionizable heteroatoms in the mass range".into(),
    )
}

/// `("true" | "doubtful" | "false", rule)` for one list entry. `formula` and `smiles` are empty when missing, `skeleton` is the InChIKey first block.
pub fn lc_esi_amenable(
    smiles: &str,
    mono_mass: Option<f64>,
    formula: Option<&str>,
    charge: Option<i64>,
    xlogp: Option<f64>,
    skeleton: &str,
) -> (&'static str, String) {
    let skel: String = skeleton.chars().take(14).collect();
    let smi = smiles.trim();
    if smi.is_empty() || smi == "nan" {
        return ("false", "structure not resolved on PubChem".into());
    }
    if let Some((_, why)) = FUMIGANTS.iter().find(|(k, _)| *k == skel) {
        return ("false", format!("volatile/fumigant: {why}"));
    }
    if let Some((_, why)) = PERMANENT_CATIONS.iter().find(|(k, _)| *k == skel) {
        return ("false", format!("permanent charge: {why}"));
    }
    let Some(mm) = mono_mass else {
        return ("false", "monoisotopic mass not available".into());
    };
    if !(MASS_MIN..=MASS_MAX).contains(&mm) {
        return ("false", format!("mass out of range {MASS_MIN:.0}-{MASS_MAX:.0} Da (single charge workflow annotation window)"));
    }
    let fstr = formula.unwrap_or("None");
    if !has_carbon(fstr) {
        return ("false", "inorganic (no carbon atom)".into());
    }
    let metal = has_metal(fstr);
    let multi = smiles.contains('.');
    if let (Some(m), false) = (metal, multi) {
        return ("false", format!("inorganic/organometallic (contains {m})"));
    }
    if multi {
        if let Some(m) = metal {
            return ("false", format!("heavy metal salt (contains {m})"));
        }
        let frag = largest_organic_fragment(smiles).unwrap_or("");
        let q_struct = permanent_cation(frag);
        if q_struct != 0 {
            return cation_rule(q_struct, None, Some(frag));
        }
        let counter = has_counterion(fstr);
        let (base, rule) = core_rules(frag, None, Some(0), xlogp, Some(mm));
        if base == "false" {
            return (
                "false",
                format!("salt; the organic component is incompatible: {rule}"),
            );
        }
        if let Some(c) = counter {
            return (
                "true",
                format!("salt of {c}: the free form is detected ({rule})"),
            );
        }
        return (
            "doubtful",
            "salt/multicomponent: the free form in solution is assumed".into(),
        );
    }
    core_rules(smiles, formula, charge, xlogp, Some(mm))
}
