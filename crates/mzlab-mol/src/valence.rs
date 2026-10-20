//! Valence rules of the SMILES reader and writer: allowed valences per element and charge, the hydrogens a bare organic-subset atom
//! carries, the radicals of a bracket atom. Only main-group elements have rules; any other element gets no implicit hydrogens or radicals.

/// Outer electrons of the elements that have valence rules.
const OUTER: &[(&str, i32)] = &[
    ("B", 3),
    ("C", 4),
    ("N", 5),
    ("O", 6),
    ("F", 7),
    ("Si", 4),
    ("P", 5),
    ("S", 6),
    ("Cl", 7),
    ("As", 5),
    ("Se", 6),
    ("Br", 7),
    ("I", 7),
];

/// Elements that may expand the octet (valences base, base+2, … up to 7).
const HYPER: &[&str] = &["P", "S", "Cl", "Br", "I", "As", "Se"];

/// Allowed valences of `el` with charge `chg`, ascending; empty for an element without rules.
pub(crate) fn allowed(el: &str, chg: i32) -> Vec<u32> {
    let Some(&(_, outer)) = OUTER.iter().find(|(e, _)| *e == el) else {
        return Vec::new();
    };
    let eff = outer - chg;
    if !(0..=8).contains(&eff) {
        return Vec::new();
    }
    let base = if eff <= 4 { eff } else { 8 - eff } as u32;
    let mut v = vec![base];
    if eff >= 5 && HYPER.contains(&el) {
        let mut x = base + 2;
        while x <= 7 {
            v.push(x);
            x += 2;
        }
    }
    v
}

/// Bonds still missing to reach the lowest allowed valence that is at least `total` (bond orders plus hydrogens); 0 when none fits.
pub(crate) fn shortfall(el: &str, chg: i32, total: u32) -> u32 {
    allowed(el, chg)
        .into_iter()
        .find(|&v| v >= total)
        .map_or(0, |v| v - total)
}

/// True when an aromatic atom with `total` valence so far still needs its share of a double bond.
pub(crate) fn needs_double(el: &str, chg: i32, total: u32) -> bool {
    shortfall(el, chg, total) > 0
}
