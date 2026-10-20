//! Where a transformation can act: the atoms of the parent that can take it, with the formula of the product.

use mzlab_core::elements::{self, Formula};
use mzlab_core::error::{Error, Result};

use super::simmetria::symmetry_classes;
use crate::chem::frammenti::apply_delta;
use crate::chem::grafo::Mol;

/// One atom where the change can sit. `kind`: `aromatic_oh`, `aliphatic_oh`, `n_oxide`, `s_oxide`, `demethyl`, `carbonyl`, `aromatic_cl`, `aromatic_br`.
/// `class` is the symmetry class of the atom (`symmetry_classes`); `tp_formula` the Hill formula of the product. The kind `dehydro` (`-2H`) is a bond:
/// `atom` and `partner` are its two carbons and `class` joins their two symmetry classes. The kinds `dioxid` (`+2O`) and `carbonyl_oh` (`+2O-2H`) are
/// PAIRS of atoms that each take one change (`atom` and `partner`; `carbonyl_oh`: `atom` is the carbonyl); their `class` is unique to the pair.
#[derive(Debug, Clone, PartialEq)]
pub struct Site {
    pub atom: usize,
    pub partner: Option<usize>,
    pub kind: &'static str,
    pub class: u32,
    pub tp_formula: String,
}

fn f(el: &str, n: u64) -> Formula {
    vec![(el.to_string(), n)]
}

/// True for the kinds whose site is two atoms that each take a change, not a bond.
pub fn is_pair(kind: &str) -> bool {
    matches!(kind, "dioxid" | "carbonyl_oh")
}

/// The signed formula change of a kind of site.
pub(super) fn delta_of(kind: &str) -> Vec<(i64, Formula)> {
    match kind {
        "dioxid" => vec![(2, f("O", 1))],
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
        "+2O-2H" | "+2O-H4" => &["carbonyl_oh"],
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
    if is_pair(kinds[0]) {
        return checked(pair_sites(mol, kinds[0], &parent)?, transform);
    }
    let mut out = Vec::new();
    for (i, class) in cls.iter().enumerate() {
        let Some(kind) = kinds.iter().find(|k| can_take(mol, i, k)) else {
            continue;
        };
        let tp = apply_delta(&parent, &delta_of(kind))?;
        out.push(Site {
            atom: i,
            partner: None,
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

/// `+2O` and `+2O-2H`: every pair of distinct atoms that can each take one of the two changes (an oxygen on each, or a carbonyl and an oxygen).
/// Two atoms with the same symmetry may still be placed differently from each other, so the class of a pair is the pair itself.
fn pair_sites(mol: &Mol, kind: &'static str, parent: &Formula) -> Result<Vec<Site>> {
    let oxid = |i: usize| {
        ["aromatic_oh", "aliphatic_oh", "n_oxide", "s_oxide"]
            .iter()
            .any(|k| can_take(mol, i, k))
    };
    let tp = elements::fmt(&apply_delta(parent, &delta_of(kind))?);
    let n = mol.atoms.len();
    let mut out = Vec::new();
    for a in 0..n {
        for b in 0..n {
            let ok = if kind == "dioxid" {
                a < b && oxid(a) && oxid(b)
            } else {
                a != b && can_take(mol, a, "carbonyl") && oxid(b)
            };
            if ok {
                out.push(Site {
                    atom: a,
                    partner: Some(b),
                    kind,
                    class: (a * 65536 + b) as u32,
                    tp_formula: tp.clone(),
                });
            }
        }
    }
    Ok(out)
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
