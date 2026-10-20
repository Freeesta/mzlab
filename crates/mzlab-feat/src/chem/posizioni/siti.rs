//! Where a transformation can act: the atoms of the parent that can take it, with the formula of the product.

use mzlab_core::elements::{self, Formula};
use mzlab_core::error::{Error, Result};

use super::simmetria::symmetry_classes;
use crate::chem::frammenti::apply_delta;
use crate::chem::grafo::Mol;

/// One atom where the change can sit. `kind`: `aromatic_oh`, `aliphatic_oh`, `n_oxide`, `s_oxide`, `demethyl`, `carbonyl`, `aromatic_cl`, `aromatic_br`.
/// `class` is the symmetry class of the atom (`symmetry_classes`); `tp_formula` the Hill formula of the product. The kind `dehydro` (`-2H`) is a bond, `deethyl` (`-C2H4`) an ethyl group (`atom` the CH2, `partner` the CH3, both lost):
/// `atom` and `partner` are its two carbons and `class` joins their two symmetry classes. The kinds `dioxid` (`+2O`), `carbonyl_oh` (`+2O-2H`) and `trioxid` (`+3O`)
/// are SETS of atoms that each take one change (`atom`, `partner` and, for `trioxid`, `third`; `carbonyl_oh`: `atom` is the carbonyl); their `class` is unique to the set.
#[derive(Debug, Clone, PartialEq)]
pub struct Site {
    pub atom: usize,
    pub partner: Option<usize>,
    pub third: Option<usize>,
    pub kind: &'static str,
    pub class: u32,
    pub tp_formula: String,
}

fn f(el: &str, n: u64) -> Formula {
    vec![(el.to_string(), n)]
}

/// True for the kinds whose site is several atoms that each take a change, not a bond.
pub fn is_multi(kind: &str) -> bool {
    matches!(kind, "dioxid" | "carbonyl_oh" | "trioxid")
}

/// The signed formula change of a kind of site.
pub(super) fn delta_of(kind: &str) -> Vec<(i64, Formula)> {
    match kind {
        "dioxid" => vec![(2, f("O", 1))],
        "trioxid" => vec![(3, f("O", 1))],
        "dechloro_oh" => vec![(-1, f("Cl", 1)), (1, f("O", 1)), (1, f("H", 1))],
        "amino_oh" => vec![(-1, f("N", 1)), (-1, f("H", 1)), (1, f("O", 1))],
        "methyl_cooh" => vec![(2, f("O", 1)), (-1, f("H", 2))],
        "deethyl" => vec![(-1, vec![("C".into(), 2), ("H".into(), 4)])],
        "carbonyl_oh" => vec![(2, f("O", 1)), (-1, f("H", 2))],
        "demethyl" => vec![(-1, vec![("C".into(), 1), ("H".into(), 2)])],
        "carbonyl" => vec![(1, f("O", 1)), (-1, f("H", 2))],
        "dehydro" => vec![(-1, f("H", 2))],
        "aromatic_cl" => vec![(1, f("Cl", 1)), (-1, f("H", 1))],
        "aromatic_br" => vec![(1, f("Br", 1)), (-1, f("H", 1))],
        _ => vec![(1, f("O", 1))],
    }
}

/// The atoms of a site with the change each one takes (a bond or a single atom: one entry; a pair: one per atom).
pub(super) fn atom_deltas(site: &Site) -> Vec<(usize, Vec<(i64, Formula)>)> {
    match (site.kind, site.partner) {
        ("trioxid", Some(p)) => {
            let o = delta_of("aliphatic_oh");
            let mut v = vec![(site.atom, o.clone()), (p, o.clone())];
            v.extend(site.third.map(|t| (t, o)));
            v
        }
        ("dioxid", Some(p)) => vec![
            (site.atom, delta_of("aliphatic_oh")),
            (p, delta_of("aliphatic_oh")),
        ],
        ("carbonyl_oh", Some(p)) => vec![
            (site.atom, delta_of("carbonyl")),
            (p, delta_of("aliphatic_oh")),
        ],
        _ => vec![(site.atom, delta_of(site.kind))],
    }
}

/// The kinds of site a transformation can produce (`+O-H2`, spaces and the minus sign «−» are accepted).
fn kinds_of(transform: &str) -> Result<&'static [&'static str]> {
    let t: String = transform
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| if c == '\u{2212}' { '-' } else { c })
        .collect();
    Ok(match t.as_str() {
        "+O" => &["aromatic_oh", "aliphatic_oh", "n_oxide", "s_oxide"],
        "-CH2" => &["demethyl"],
        "+O-2H" | "+O-H2" => &["carbonyl"],
        "-2H" => &["dehydro"],
        "+Cl-H" => &["aromatic_cl"],
        "+Br-H" => &["aromatic_br"],
        "+2O" => &["dioxid"],
        "+3O" => &["trioxid"],
        "-C2H4" => &["deethyl"],
        "+2O-2H" | "+2O-H4" => &["carbonyl_oh", "methyl_cooh"],
        "-Cl+OH" => &["dechloro_oh"],
        "NH2>OH" => &["amino_oh"],
        "CH3>COOH" => &["methyl_cooh"],
        _ => return Err(Error::new("error.posiz.transform").with("transform", transform)),
    })
}

fn all_single(m: &Mol, i: usize) -> bool {
    m.adj[i].iter().all(|(_, k)| m.bonds[*k].order == 1.0)
}

fn can_take(m: &Mol, i: usize, kind: &str) -> bool {
    let a = &m.atoms[i];
    let neutral = a.charge == 0;
    match kind {
        "aromatic_oh" | "aromatic_cl" | "aromatic_br" => a.el == "C" && a.arom && a.h >= 1,
        "aliphatic_oh" => m.sp3_c(i) && a.h >= 1,
        "carbonyl" => m.sp3_c(i) && a.h == 2,
        "n_oxide" => {
            let acyl = m.adj[i].iter().any(|(j, _)| m.is_acyl(*j));
            a.el == "N"
                && neutral
                && a.h == 0
                && !acyl
                && ((a.arom && m.heavy(i) == 2) || (!a.arom && m.heavy(i) == 3 && all_single(m, i)))
        }
        "s_oxide" => {
            a.el == "S" && neutral && !a.arom && a.h == 0 && m.heavy(i) == 2 && all_single(m, i)
        }
        // The atom is the one replaced: the Cl of an aryl chloride, the N of a primary amine, the carbon of a methyl on a carbon.
        "dechloro_oh" => a.el == "Cl" && m.adj[i].iter().any(|(j, _)| m.atoms[*j].arom),
        "amino_oh" => a.el == "N" && neutral && !a.arom && a.h == 2 && m.heavy(i) == 1,
        "methyl_cooh" => {
            a.el == "C" && !a.arom && a.h == 3 && m.heavy(i) == 1 && m.adj[i].iter().all(|(j, _)| m.atoms[*j].el == "C")
        }
        "deethyl" => {
            a.el == "C"
                && !a.arom
                && a.h == 2
                && m.heavy(i) == 2
                && m.adj[i]
                    .iter()
                    .any(|(j, _)| matches!(m.el(*j), "N" | "O" | "S"))
                && m.adj[i]
                    .iter()
                    .any(|(j, _)| m.atoms[*j].el == "C" && m.atoms[*j].h == 3 && m.heavy(*j) == 1)
        }
        "demethyl" => {
            a.el == "C"
                && !a.arom
                && a.h == 3
                && m.heavy(i) == 1
                && matches!(m.el(m.adj[i][0].0), "N" | "O" | "S")
        }
        _ => false,
    }
}

/// Every atom of the parent that can take `transform`, in SMILES order. `error.posiz.transform` for a transformation that is not supported,
/// `error.posiz.noSites` when no atom can take it, `error.posiz.formula` if the products would not share one formula.
pub fn sites(mol: &Mol, transform: &str) -> Result<Vec<Site>> {
    let kinds = kinds_of(transform)?;
    let cls = symmetry_classes(mol);
    let parent = mol.formula();
    if kinds == ["dehydro"] {
        return checked(dehydro_sites(mol, &cls, &parent)?, transform);
    }
    // `+2O-2H` also holds the single-atom sites of `CH3>COOH` (same net change), after the sets of atoms.
    let mut out = if is_multi(kinds[0]) {
        multi_sites(mol, kinds[0], &parent)?
    } else {
        Vec::new()
    };
    for (i, class) in cls.iter().enumerate() {
        let Some(kind) = kinds.iter().find(|k| can_take(mol, i, k)) else {
            continue;
        };
        let tp = apply_delta(&parent, &delta_of(kind))?;
        // An ethyl group is lost whole: the site holds the CH2 and its CH3.
        let partner = (*kind == "deethyl")
            .then(|| {
                mol.adj[i]
                    .iter()
                    .map(|(j, _)| *j)
                    .find(|j| mol.atoms[*j].el == "C" && mol.atoms[*j].h == 3 && mol.heavy(*j) == 1)
            })
            .flatten();
        out.push(Site {
            atom: i,
            partner,
            third: None,
            kind,
            class: *class,
            tp_formula: elements::fmt(&tp),
        });
    }
    checked(out, transform)
}

fn checked(out: Vec<Site>, transform: &str) -> Result<Vec<Site>> {
    if out.is_empty() {
        return Err(Error::new("error.posiz.noSites").with("transform", transform));
    }
    check_formulas(&out)?;
    Ok(out)
}

/// The most atoms that can take an oxygen for `+3O`: the sets are the triples among them, and the cost grows with their cube.
const MAX_TRIPLE_ATOMS: usize = 14;

/// `+2O`, `+2O-2H` and `+3O`: every set of distinct atoms that can each take one of the changes (an oxygen on each; a carbonyl and an oxygen).
/// Atoms with the same symmetry may still be placed differently from each other, so the class of a set is the set itself (its order of listing).
fn multi_sites(mol: &Mol, kind: &'static str, parent: &Formula) -> Result<Vec<Site>> {
    let oxid = |i: usize| {
        ["aromatic_oh", "aliphatic_oh", "n_oxide", "s_oxide"]
            .iter()
            .any(|k| can_take(mol, i, k))
    };
    let tp = elements::fmt(&apply_delta(parent, &delta_of(kind))?);
    let n = mol.atoms.len();
    let ox: Vec<usize> = (0..n).filter(|i| oxid(*i)).collect();
    let mut sets: Vec<[usize; 3]> = Vec::new();
    match kind {
        "dioxid" => {
            for (k, a) in ox.iter().enumerate() {
                sets.extend(ox[k + 1..].iter().map(|b| [*a, *b, usize::MAX]));
            }
        }
        "trioxid" if ox.len() > MAX_TRIPLE_ATOMS => {
            return Err(Error::new("error.posiz.noSites").with("transform", "+3O"))
        }
        "trioxid" => {
            for (k, a) in ox.iter().enumerate() {
                for (m, b) in ox.iter().enumerate().skip(k + 1) {
                    sets.extend(ox[m + 1..].iter().map(|c| [*a, *b, *c]));
                }
            }
        }
        _ => {
            for a in (0..n).filter(|a| can_take(mol, *a, "carbonyl")) {
                sets.extend(ox.iter().filter(|b| **b != a).map(|b| [a, *b, usize::MAX]));
            }
        }
    }
    Ok(sets
        .into_iter()
        .enumerate()
        .map(|(k, [a, b, c])| Site {
            atom: a,
            partner: Some(b),
            third: (c != usize::MAX).then_some(c),
            kind,
            class: k as u32,
            tp_formula: tp.clone(),
        })
        .collect())
}

/// `-2H`: every single C-C bond between two sp3 carbons that both keep an H (a ring bond too: the double bond closes inside the ring).
fn dehydro_sites(mol: &Mol, cls: &[u32], parent: &Formula) -> Result<Vec<Site>> {
    let tp = elements::fmt(&apply_delta(parent, &delta_of("dehydro"))?);
    let sp3 = |i: usize| mol.sp3_c(i) && mol.atoms[i].h >= 1;
    let mut out: Vec<Site> = mol
        .bonds
        .iter()
        .filter(|b| b.order == 1.0 && sp3(b.a) && sp3(b.b))
        .map(|b| {
            let (a, p) = (b.a.min(b.b), b.a.max(b.b));
            let (lo, hi) = (cls[a].min(cls[p]), cls[a].max(cls[p]));
            Site {
                atom: a,
                partner: Some(p),
                third: None,
                kind: "dehydro",
                class: lo * 65536 + hi,
                tp_formula: tp.clone(),
            }
        })
        .collect();
    out.sort_by_key(|s| (s.atom, s.partner));
    Ok(out)
}

/// All the sites of one transformation must give the same product formula; anything else is a bug upstream, never a result.
pub fn check_formulas(sites: &[Site]) -> Result<()> {
    match sites.iter().find(|s| s.tp_formula != sites[0].tp_formula) {
        Some(s) => Err(Error::new("error.posiz.formula")
            .with("a", &sites[0].tp_formula)
            .with("b", &s.tp_formula)),
        None => Ok(()),
    }
}
