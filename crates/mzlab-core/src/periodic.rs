//! The periodic table for mass spectrometry (`elements/1`): per element name, class, monoisotopic and average mass and the natural
//! isotopes, from the nuclide and abundance tables. ONE place for the classes; the dialog «Periodic table», the `.wasm`, the app and
//! `mzlab-cli ion elements --json` print the same bytes. Only what MS needs: no density, melting point or electronegativity.

use crate::json::Out;

/// `(key, code, name)`: the code is the letter signal of a tile, so that colour is never the only one.
pub const CLASSES: [(&str, &str, &str); 9] = [
    ("alkali", "AM", "Alkali metal"),
    ("alkaline", "AE", "Alkaline earth metal"),
    ("transition", "TM", "Transition metal"),
    ("post_transition", "PM", "Post-transition metal"),
    ("metalloid", "MT", "Metalloid"),
    ("nonmetal", "NM", "Reactive nonmetal"),
    ("noble", "NG", "Noble gas"),
    ("lanthanide", "LN", "Lanthanide"),
    ("actinide", "AN", "Actinide"),
];

/// Symbols by atomic number, H (1) to Og (118).
const SYMBOLS: [&str; 118] = [
    "H", "He", "Li", "Be", "B", "C", "N", "O", "F", "Ne", "Na", "Mg", "Al", "Si", "P", "S", "Cl",
    "Ar", "K", "Ca", "Sc", "Ti", "V", "Cr", "Mn", "Fe", "Co", "Ni", "Cu", "Zn", "Ga", "Ge", "As",
    "Se", "Br", "Kr", "Rb", "Sr", "Y", "Zr", "Nb", "Mo", "Tc", "Ru", "Rh", "Pd", "Ag", "Cd", "In",
    "Sn", "Sb", "Te", "I", "Xe", "Cs", "Ba", "La", "Ce", "Pr", "Nd", "Pm", "Sm", "Eu", "Gd", "Tb",
    "Dy", "Ho", "Er", "Tm", "Yb", "Lu", "Hf", "Ta", "W", "Re", "Os", "Ir", "Pt", "Au", "Hg", "Tl",
    "Pb", "Bi", "Po", "At", "Rn", "Fr", "Ra", "Ac", "Th", "Pa", "U", "Np", "Pu", "Am", "Cm", "Bk",
    "Cf", "Es", "Fm", "Md", "No", "Lr", "Rf", "Db", "Sg", "Bh", "Hs", "Mt", "Ds", "Rg", "Cn", "Nh",
    "Fl", "Mc", "Lv", "Ts", "Og",
];

/// Names by atomic number.
const NAMES: [&str; 118] = [
    "Hydrogen",
    "Helium",
    "Lithium",
    "Beryllium",
    "Boron",
    "Carbon",
    "Nitrogen",
    "Oxygen",
    "Fluorine",
    "Neon",
    "Sodium",
    "Magnesium",
    "Aluminium",
    "Silicon",
    "Phosphorus",
    "Sulfur",
    "Chlorine",
    "Argon",
    "Potassium",
    "Calcium",
    "Scandium",
    "Titanium",
    "Vanadium",
    "Chromium",
    "Manganese",
    "Iron",
    "Cobalt",
    "Nickel",
    "Copper",
    "Zinc",
    "Gallium",
    "Germanium",
    "Arsenic",
    "Selenium",
    "Bromine",
    "Krypton",
    "Rubidium",
    "Strontium",
    "Yttrium",
    "Zirconium",
    "Niobium",
    "Molybdenum",
    "Technetium",
    "Ruthenium",
    "Rhodium",
    "Palladium",
    "Silver",
    "Cadmium",
    "Indium",
    "Tin",
    "Antimony",
    "Tellurium",
    "Iodine",
    "Xenon",
    "Caesium",
    "Barium",
    "Lanthanum",
    "Cerium",
    "Praseodymium",
    "Neodymium",
    "Promethium",
    "Samarium",
    "Europium",
    "Gadolinium",
    "Terbium",
    "Dysprosium",
    "Holmium",
    "Erbium",
    "Thulium",
    "Ytterbium",
    "Lutetium",
    "Hafnium",
    "Tantalum",
    "Tungsten",
    "Rhenium",
    "Osmium",
    "Iridium",
    "Platinum",
    "Gold",
    "Mercury",
    "Thallium",
    "Lead",
    "Bismuth",
    "Polonium",
    "Astatine",
    "Radon",
    "Francium",
    "Radium",
    "Actinium",
    "Thorium",
    "Protactinium",
    "Uranium",
    "Neptunium",
    "Plutonium",
    "Americium",
    "Curium",
    "Berkelium",
    "Californium",
    "Einsteinium",
    "Fermium",
    "Mendelevium",
    "Nobelium",
    "Lawrencium",
    "Rutherfordium",
    "Dubnium",
    "Seaborgium",
    "Bohrium",
    "Hassium",
    "Meitnerium",
    "Darmstadtium",
    "Roentgenium",
    "Copernicium",
    "Nihonium",
    "Flerovium",
    "Moscovium",
    "Livermorium",
    "Tennessine",
    "Oganesson",
];

/// Index into `CLASSES` of the element of atomic number `z` (1..=118).
pub fn class_of(z: u8) -> usize {
    match z {
        3 | 11 | 19 | 37 | 55 | 87 => 0,
        4 | 12 | 20 | 38 | 56 | 88 => 1,
        21..=30 | 39..=48 | 72..=80 | 104..=112 => 2,
        13 | 31 | 49 | 50 | 81..=84 | 113..=116 => 3,
        5 | 14 | 32 | 33 | 51 | 52 | 85 | 117 => 4,
        1 | 6 | 7 | 8 | 9 | 15 | 16 | 17 | 34 | 35 | 53 => 5,
        2 | 10 | 18 | 36 | 54 | 86 | 118 => 6,
        57..=71 => 7,
        _ => 8, // 89..=103
    }
}

/// Relative intensity of the M+2 isotope from which an element counts as «notable pattern» (Cl, Br, S, Cu, Ni, ...).
const M2_NOTABLE: f64 = 0.04;

/// `elements/1`: `{"schema","classes":[{key,code,name}],"elements":[{z,symbol,name,class,mono,avg,mono_only,m2_rel,isotopes:[{a,mass,abund}]}]}`.
/// `mono` is the exact mass of the most abundant natural isotope (the nuclide table where the abundances are missing), `avg` the abundance-weighted mean of the listed isotopes, `abund` is
/// in percent, `m2_rel` the M+2 / main intensity ratio; `avg` and `m2_rel` are `null` and `isotopes` empty for an element outside the
/// abundance tables, `mono` too when the nuclide table lacks it (Z above 108).
pub fn elements_json() -> String {
    let mut o = Out(String::new());
    o.raw("{\"schema\":\"elements/1\",\"classes\":[");
    for (i, (k, c, n)) in CLASSES.iter().enumerate() {
        o.raw(if i > 0 { ",{\"key\":" } else { "{\"key\":" })
            .str(k)
            .raw(",\"code\":")
            .str(c)
            .raw(",\"name\":")
            .str(n)
            .raw("}");
    }
    o.raw("],\"elements\":[");
    for i in 0..NAMES.len() {
        element_json(&mut o, i);
    }
    o.raw("]}");
    o.0
}

/// `(mono, avg, m2_rel)` of an element from its isotopes (see `elements_json`).
fn masses(symbol: &str, iso: &[(f64, f64)]) -> (Option<f64>, Option<f64>, Option<f64>) {
    let main = iso
        .iter()
        .copied()
        .fold(None, |a: Option<(f64, f64)>, b| match a {
            Some(x) if x.1 >= b.1 => Some(x),
            _ => Some(b),
        });
    // an element outside the abundance tables keeps the monoisotopic mass of the nuclide table, with no isotopes and no average
    let mono = main
        .map(|m| m.0)
        .or_else(|| crate::elements::nuclide(symbol, None).map(|n| n.1));
    let tot: f64 = iso.iter().map(|x| x.1).sum();
    let avg = if tot > 0.0 {
        Some(iso.iter().map(|x| x.0 * x.1).sum::<f64>() / tot)
    } else {
        None
    };
    let m2 = main.and_then(|m| {
        iso.iter()
            .find(|x| (x.0 - m.0 - 2.0).abs() < 0.1)
            .map(|x| x.1 / m.1)
    });
    (mono, avg, m2)
}

/// One element of `elements/1`, by index (atomic number - 1).
fn element_json(o: &mut Out, i: usize) {
    let (name, z) = (NAMES[i], (i + 1) as u8);
    let symbol = SYMBOLS[i];
    let iso = crate::isoscore::isotopes_of(symbol).unwrap_or(&[]);
    let (mono, avg, m2) = masses(symbol, iso);
    o.raw(if i > 0 { ",{\"z\":" } else { "{\"z\":" })
        .int(i64::from(z))
        .raw(",\"symbol\":")
        .str(symbol)
        .raw(",\"name\":")
        .str(name);
    o.raw(",\"class\":")
        .str(CLASSES[class_of(z)].0)
        .raw(",\"mono\":")
        .opt(mono)
        .raw(",\"avg\":")
        .opt(avg);
    o.raw(",\"mono_only\":")
        .raw(if iso.len() == 1 { "true" } else { "false" });
    o.raw(",\"m2_rel\":")
        .opt(m2.filter(|r| *r >= M2_NOTABLE))
        .raw(",\"isotopes\":[");
    for (j, (m, a)) in iso.iter().enumerate() {
        o.raw(if j > 0 { ",{\"a\":" } else { "{\"a\":" })
            .int(m.round() as i64)
            .raw(",\"mass\":")
            .num(*m)
            .raw(",\"abund\":")
            .num(a * 100.0)
            .raw("}");
    }
    o.raw("]}");
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_element_has_a_name_a_symbol_and_a_class() {
        assert_eq!((NAMES.len(), SYMBOLS.len()), (118, 118));
        assert_eq!(SYMBOLS[102], "Lr");
        assert_eq!(CLASSES[class_of(17)].0, "nonmetal");
        assert_eq!(CLASSES[class_of(57)].0, "lanthanide");
        assert_eq!(CLASSES[class_of(92)].0, "actinide");
        assert_eq!(CLASSES[class_of(30)].0, "transition");
    }

    #[test]
    fn chlorine_has_the_m2_pattern_and_sodium_is_monoisotopic() {
        let j = elements_json();
        let cl = j
            .split("{\"z\":17,")
            .nth(1)
            .unwrap()
            .split("{\"z\":18,")
            .next()
            .unwrap();
        assert!(
            cl.contains("\"mono_only\":false") && cl.contains("\"m2_rel\":0.31"),
            "{cl}"
        );
        let na = j
            .split("{\"z\":11,")
            .nth(1)
            .unwrap()
            .split("{\"z\":12,")
            .next()
            .unwrap();
        assert!(
            na.contains("\"mono_only\":true") && na.contains("\"m2_rel\":null"),
            "{na}"
        );
    }
}
