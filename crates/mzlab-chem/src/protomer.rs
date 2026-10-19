//! Protonation state at pH 7 from a small table of typical pKa values (no quantum pKa): the single most probable charge form of
//! the parent, used before the reactivity calculation (`reatt/1`, field `protomer`). Used by `reatt.rs`.
//!
//! Rule: an acid is deprotonated when its pKa < 7, a base is protonated when its pKa > 7; each base already protonated within a
//! few bonds lowers the pKa of the next one. One form is returned, never an ensemble. Charges written in the SMILES are kept.

use mzlab_feat::chem::grafo::{Atom, Mol};

/// pH of the form.
pub const PH: f64 = 7.0;

/// Typical pKa values (acids: the acid itself; bases: the conjugate acid). Groups not listed stay as written: aromatic amines
/// (≈4.6), amides, urea, carbamate, N-acylsulfonamide and sulfonamide N–H (5.5–10), pyridine-type and azole N (≤7.0), thiols (≈10).
const CARBOXYLIC: (&str, f64) = ("carboxylic acid", 4.5);
const SULFONIC: (&str, f64) = ("sulfonic or sulfuric acid", -1.0);
const PHOSPHORIC: (&str, f64) = ("phosphoric or phosphonic acid (first OH)", 2.0);
const TETRAZOLE: (&str, f64) = ("tetrazole N–H", 4.9);
const PHENOL: (&str, f64) = ("phenol", 10.0);
const AMINE: (&str, f64) = ("aliphatic amine", 10.0);
const AMIDINE: (&str, f64) = ("amidine", 12.0);
const GUANIDINE: (&str, f64) = ("guanidine", 13.6);
/// Phenol pKa shift per nitro group or heavy halogen ortho or para to the OH.
const NITRO_SHIFT: f64 = 2.8;
const HALOGEN_SHIFT: f64 = 1.3;
/// pKa drop of a base for each cationic centre already in the molecule within 3 bonds, and within 5.
const NEAR_SHIFT: f64 = 3.5;
const FAR_SHIFT: f64 = 1.5;

/// One ionisation applied to the molecule.
pub struct Change {
    pub group: &'static str,
    pub pka: f64,
    /// Index in `grafo::Mol` of the atom that lost or gained the proton.
    pub atom: usize,
    pub deprotonated: bool,
}

pub struct Protomer {
    pub mol: Mol,
    pub smiles: String,
    pub changes: Vec<Change>,
    pub charge: i32,
}

type Nb = Vec<Vec<(usize, f32)>>;

fn neighbours(m: &Mol) -> Nb {
    let mut nb: Nb = vec![vec![]; m.atoms.len()];
    for b in &m.bonds {
        nb[b.a].push((b.b, b.order));
        nb[b.b].push((b.a, b.order));
    }
    nb
}

fn el(m: &Mol, i: usize) -> &str {
    &m.atoms[i].el
}

/// A carbon with only single bonds.
fn sp3_c(m: &Mol, nb: &Nb, i: usize) -> bool {
    el(m, i) == "C" && !m.atoms[i].arom && nb[i].iter().all(|&(_, o)| o == 1.0)
}

/// Number of oxygens double-bonded to `i`.
fn double_o(m: &Mol, nb: &Nb, i: usize) -> usize {
    nb[i]
        .iter()
        .filter(|&&(j, o)| o == 2.0 && el(m, j) == "O")
        .count()
}

/// Shortest path in bonds between two heavy atoms (None if they are not connected).
fn distance(nb: &Nb, from: usize, to: usize) -> Option<usize> {
    let mut seen = vec![false; nb.len()];
    let mut layer = vec![from];
    seen[from] = true;
    for d in 0..nb.len() {
        if layer.contains(&to) {
            return Some(d);
        }
        let mut next = vec![];
        for &i in &layer {
            for &(j, _) in &nb[i] {
                if !seen[j] {
                    seen[j] = true;
                    next.push(j);
                }
            }
        }
        layer = next;
    }
    None
}

/// Ring distance (aromatic atoms only) of every aromatic atom from `ipso`.
fn ring_distances(m: &Mol, nb: &Nb, ipso: usize) -> Vec<Option<usize>> {
    let mut dist = vec![None; nb.len()];
    dist[ipso] = Some(0);
    let mut layer = vec![ipso];
    while !layer.is_empty() {
        let mut next = vec![];
        for &i in &layer {
            for &(j, _) in &nb[i] {
                if m.atoms[j].arom && dist[j].is_none() {
                    dist[j] = dist[i].map(|d| d + 1);
                    next.push(j);
                }
            }
        }
        layer = next;
    }
    dist
}

/// pKa of the phenol whose ring carbon is `ipso`: 10.0 lowered by nitro groups and heavy halogens ortho or para to it.
fn phenol_pka(m: &Mol, nb: &Nb, ipso: usize) -> f64 {
    let dist = ring_distances(m, nb, ipso);
    let mut pka = PHENOL.1;
    for (i, d) in dist.iter().enumerate() {
        if !matches!(d, Some(1) | Some(3)) {
            continue;
        }
        for &(j, _) in nb[i].iter().filter(|&&(j, _)| !m.atoms[j].arom) {
            let nitro =
                el(m, j) == "N" && nb[j].iter().filter(|&&(k, _)| el(m, k) == "O").count() == 2;
            if nitro {
                pka -= NITRO_SHIFT;
            } else if matches!(el(m, j), "Cl" | "Br" | "I") {
                pka -= HALOGEN_SHIFT;
            }
        }
    }
    pka
}

/// A five-membered aromatic ring through `i` made of four N and one C (tetrazole).
fn in_tetrazole(m: &Mol, nb: &Nb, i: usize) -> bool {
    fn walk(m: &Mol, nb: &Nb, path: &mut Vec<usize>) -> bool {
        let last = path[path.len() - 1];
        if path.len() == 5 {
            let n = path.iter().filter(|&&k| el(m, k) == "N").count();
            return n == 4 && nb[last].iter().any(|&(j, _)| j == path[0]);
        }
        for &(j, _) in &nb[last] {
            if m.atoms[j].arom && !path.contains(&j) {
                path.push(j);
                let found = walk(m, nb, path);
                path.pop();
                if found {
                    return true;
                }
            }
        }
        false
    }
    walk(m, nb, &mut vec![i])
}

/// Acid groups found in `m` as (atom that loses the proton, group, pKa); a P keeps one OH only.
fn acids(m: &Mol, nb: &Nb) -> Vec<(usize, &'static str, f64)> {
    let mut out = Vec::new();
    let mut used_p = Vec::new();
    for (i, a) in m.atoms.iter().enumerate() {
        if a.charge != 0 || a.h != 1 {
            continue;
        }
        if a.el == "N" {
            if a.arom && in_tetrazole(m, nb, i) {
                out.push((i, TETRAZOLE.0, TETRAZOLE.1));
            }
            continue;
        }
        if a.el != "O" || nb[i].len() != 1 {
            continue;
        }
        let x = nb[i][0].0;
        match el(m, x) {
            "C" if m.atoms[x].arom => out.push((i, PHENOL.0, phenol_pka(m, nb, x))),
            "C" if double_o(m, nb, x) == 1 => out.push((i, CARBOXYLIC.0, CARBOXYLIC.1)),
            "S" if double_o(m, nb, x) >= 2 => out.push((i, SULFONIC.0, SULFONIC.1)),
            "P" if double_o(m, nb, x) >= 1 && !used_p.contains(&x) => {
                used_p.push(x);
                out.push((i, PHOSPHORIC.0, PHOSPHORIC.1));
            }
            _ => {}
        }
    }
    out
}

/// An N whose substituent (other than `from`) withdraws electrons: a heteroatom, an acyl or imine carbon, or a nitrile. Such an N
/// is not a strong base.
fn withdrawing(m: &Mol, nb: &Nb, n: usize, from: usize) -> bool {
    nb[n].iter().filter(|&&(j, _)| j != from).any(|&(j, _)| {
        matches!(el(m, j), "O" | "N" | "S" | "P")
            || nb[j].iter().any(|&(k, o)| {
                k != n && (o == 3.0 || o == 2.0 && matches!(el(m, k), "O" | "N" | "S"))
            })
    })
}

/// Amidine or guanidine centred on carbon `c`: the double-bonded N that takes the proton, and the group.
fn amidine_at(m: &Mol, nb: &Nb, c: usize) -> Option<(usize, &'static str, f64)> {
    if el(m, c) != "C" || m.atoms[c].arom || nb[c].len() != 3 {
        return None;
    }
    let ns: Vec<(usize, f32)> = nb[c]
        .iter()
        .copied()
        .filter(|&(j, _)| el(m, j) == "N")
        .collect();
    let ok_rest = nb[c].iter().all(|&(j, _)| matches!(el(m, j), "N" | "C"));
    let imine = ns
        .iter()
        .find(|&&(j, o)| o == 2.0 && m.atoms[j].charge == 0 && !m.atoms[j].arom)?
        .0;
    let amino = ns
        .iter()
        .filter(|&&(j, o)| o == 1.0 && !m.atoms[j].arom)
        .count();
    let clean = ns
        .iter()
        .all(|&(j, _)| !withdrawing(m, nb, j, c) && m.atoms[j].charge == 0);
    if !ok_rest || amino == 0 || !clean {
        return None;
    }
    let g = if ns.len() == 3 { GUANIDINE } else { AMIDINE };
    Some((imine, g.0, g.1))
}

/// Base groups found in `m` as (atom that gains the proton, group, pKa).
fn bases(m: &Mol, nb: &Nb) -> Vec<(usize, &'static str, f64)> {
    let mut out = Vec::new();
    for (i, a) in m.atoms.iter().enumerate() {
        if a.charge != 0 || a.arom {
            continue;
        }
        if a.el == "C" {
            out.extend(amidine_at(m, nb, i));
        } else if a.el == "N" && nb[i].len() as u32 + a.h == 3 {
            let plain = nb[i].iter().all(|&(j, o)| o == 1.0 && sp3_c(m, nb, j));
            if plain {
                out.push((i, AMINE.0, AMINE.1));
            }
        }
    }
    out
}

/// Cationic centres already in the SMILES that are not nitro groups or N-oxides.
fn fixed_cations(m: &Mol, nb: &Nb) -> Vec<usize> {
    (0..m.atoms.len())
        .filter(|&i| {
            m.atoms[i].charge > 0
                && !nb[i]
                    .iter()
                    .any(|&(j, _)| el(m, j) == "O" && m.atoms[j].charge < 0)
        })
        .collect()
}

/// pKa of a base after the cations already placed.
fn coupled_pka(nb: &Nb, atom: usize, pka: f64, cations: &[usize]) -> f64 {
    let shift: f64 = cations
        .iter()
        .map(|&c| match distance(nb, atom, c) {
            Some(d) if d <= 3 => NEAR_SHIFT,
            Some(d) if d <= 5 => FAR_SHIFT,
            _ => 0.0,
        })
        .sum();
    pka - shift
}

/// The most probable form of `mol` (parsed from `smiles`) at pH 7.
pub fn assign(smiles: &str, mol: &Mol) -> Protomer {
    let nb = neighbours(mol);
    let mut m = mol.clone();
    let mut changes = Vec::new();
    for (atom, group, pka) in acids(mol, &nb) {
        if pka < PH {
            m.atoms[atom].h -= 1;
            m.atoms[atom].charge = -1;
            changes.push(Change {
                group,
                pka,
                atom,
                deprotonated: true,
            });
        }
    }
    let mut cations = fixed_cations(mol, &nb);
    let mut cand = bases(mol, &nb);
    cand.sort_by(|a, b| b.2.total_cmp(&a.2));
    for (atom, group, pka) in cand {
        if coupled_pka(&nb, atom, pka, &cations) > PH {
            m.atoms[atom].h += 1;
            m.atoms[atom].charge = 1;
            cations.push(atom);
            changes.push(Change {
                group,
                pka,
                atom,
                deprotonated: false,
            });
        }
    }
    changes.sort_by_key(|c| c.atom);
    let charge = m.atoms.iter().map(|a| a.charge).sum();
    let smiles = rewrite(smiles, &m, &changes);
    Protomer {
        mol: m,
        smiles,
        changes,
        charge,
    }
}

/// The atom as a bracket SMILES atom with its explicit hydrogens and charge.
fn bracket_atom(a: &Atom) -> String {
    let sym = if a.arom {
        a.el.to_lowercase()
    } else {
        a.el.clone()
    };
    let h = match a.h {
        0 => String::new(),
        1 => "H".to_string(),
        n => format!("H{n}"),
    };
    let q = match a.charge {
        0 => String::new(),
        1 => "+".to_string(),
        -1 => "-".to_string(),
        n if n > 0 => format!("+{n}"),
        n => format!("{n}"),
    };
    format!("[{sym}{h}{q}]")
}

/// `smiles` with the atoms of `changes` written as bracket atoms of the new form; atoms are counted in the order of the string.
fn rewrite(smiles: &str, m: &Mol, changes: &[Change]) -> String {
    let s: Vec<char> = smiles.trim().chars().collect();
    let (mut out, mut p, mut idx) = (String::new(), 0, 0);
    while p < s.len() {
        let two: String = s[p..(p + 2).min(s.len())].iter().collect();
        let len = if s[p] == '[' {
            s[p..].iter().position(|&c| c == ']').map_or(1, |e| e + 1)
        } else if two == "Cl" || two == "Br" {
            2
        } else if "BCNOPSFIbcnops".contains(s[p]) {
            1
        } else {
            0
        };
        if len == 0 {
            out.push(s[p]);
            p += 1;
            continue;
        }
        if changes.iter().any(|c| c.atom == idx) {
            out.push_str(&bracket_atom(&m.atoms[idx]));
        } else {
            out.extend(&s[p..p + len]);
        }
        idx += 1;
        p += len;
    }
    out
}
