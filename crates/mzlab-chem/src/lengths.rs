//! Ideal bond lengths (angstrom) for the light regularisation of the ETKDG geometry; written for mzLab from textbook values
//! (CRC Handbook, Allen et al. 1987), not taken from any force field.

use crate::mol3d::Mol3d;

/// Ideal length of the bond between atoms `a` and `b` of `m` with order `o` (1, 2, 3, 4 = aromatic).
pub fn ideal(m: &Mol3d, a: usize, b: usize, o: u8) -> f64 {
    let (x, y) = (m.el[a].min(m.el[b]), m.el[a].max(m.el[b]));
    if x == 1 {
        return hydrogen(y);
    }
    let charged = m.charge[a] != 0 || m.charge[b] != 0;
    if let Some(r) = conjugated(m, a, b, o) {
        return r;
    }
    match (x, y, o) {
        (6, 6, 1) => 1.51,
        (6, 6, 2) => 1.34,
        (6, 6, 3) => 1.20,
        (6, 6, _) => 1.40,
        (6, 7, 1) => 1.46,
        (6, 7, 2) => 1.28,
        (6, 7, 3) => 1.16,
        (6, 7, _) => 1.35,
        (6, 8, 1) => 1.40,
        (6, 8, 2) => 1.22,
        (6, 8, _) => 1.37,
        (6, 9, _) => 1.35,
        (6, 15, _) => 1.82,
        (6, 16, 1) => 1.78,
        (6, 16, 2) => 1.62,
        (6, 16, _) => 1.72,
        (6, 17, _) => 1.74,
        (6, 35, _) => 1.90,
        (6, 53, _) => 2.10,
        (7, 7, 2) => 1.25,
        (7, 7, 4) => 1.34,
        (7, 7, _) => 1.40,
        (7, 8, 4) => 1.38,
        (7, 8, _) if charged || o >= 2 => 1.23,
        (7, 8, _) => 1.40,
        (7, 16, 1) => 1.65,
        (7, 16, _) => 1.62,
        (8, 15, 1) => 1.60,
        (8, 15, _) => 1.48,
        (8, 16, 1) if !charged => 1.60,
        (8, 16, _) => 1.45,
        (15, 15, _) => 2.20,
        (16, 16, _) => 2.05,
        _ => 1.60,
    }
}

/// Single bonds of carbon to N or O that are shortened by conjugation: next to an aromatic atom or to a carbonyl carbon (amide, acid).
fn conjugated(m: &Mol3d, a: usize, b: usize, o: u8) -> Option<f64> {
    if o != 1 {
        return None;
    }
    let (c, h) = if m.el[a] == 6 { (a, b) } else { (b, a) };
    if m.el[c] != 6 || !matches!(m.el[h], 7 | 8) {
        return None;
    }
    let carbonyl = m.bonds.iter().any(|&(p, q, k)| {
        k == 2 && (p == c || q == c) && matches!(m.el[if p == c { q } else { p }], 8 | 16)
    });
    match (m.el[h], m.arom[c] || carbonyl) {
        (7, true) => Some(if carbonyl { 1.35 } else { 1.40 }),
        (8, true) => Some(if carbonyl { 1.35 } else { 1.37 }),
        _ => None,
    }
}

fn hydrogen(z: u8) -> f64 {
    match z {
        6 => 1.09,
        7 => 1.01,
        8 => 0.96,
        15 => 1.42,
        16 => 1.34,
        _ => 1.10,
    }
}
