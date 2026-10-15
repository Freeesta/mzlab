//! Where a transformation can act: the atoms of the parent that can take it, with the formula of the product.

use mzlab_core::elements::{self, Formula};
use mzlab_core::error::{Error, Result};

use super::simmetria::symmetry_classes;
use crate::chem::frammenti::apply_delta;
use crate::chem::grafo::Mol;

/// One atom where the change can sit. `kind`: `aromatic_oh`, `aliphatic_oh`, `n_oxide`, `s_oxide`, `demethyl`, `carbonyl`, `aromatic_cl`, `aromatic_br`.
/// `class` is the symmetry class of the atom (`symmetry_classes`); `tp_formula` the Hill formula of the product.
#[derive(Debug, Clone, PartialEq)]
pub struct Site {
    pub atom: usize,
    pub kind: &'static str,
    pub class: u32,
    pub tp_formula: String,
}

fn f(el: &str, n: u64) -> Formula {
    vec![(el.to_string(), n)]
}

/// The signed formula change of a kind of site.
pub(super) fn delta_of(kind: &str) -> Vec<(i64, Formula)> {
    match kind {
        "demethyl" => vec![(-1, vec![("C".into(), 1), ("H".into(), 2)])],
        "carbonyl" => vec![(1, f("O", 1)), (-1, f("H", 2))],
        "aromatic_cl" => vec![(1, f("Cl", 1)), (-1, f("H", 1))],
        "aromatic_br" => vec![(1, f("Br", 1)), (-1, f("H", 1))],
        _ => vec![(1, f("O", 1))],
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
        "+Cl-H" => &["aromatic_cl"],
        "+Br-H" => &["aromatic_br"],
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
    let mut out = Vec::new();
    for i in 0..mol.atoms.len() {
        let Some(kind) = kinds.iter().find(|k| can_take(mol, i, k)) else {
            continue;
        };
        let tp = apply_delta(&parent, &delta_of(kind))?;
        out.push(Site {
            atom: i,
            kind,
            class: cls[i],
            tp_formula: elements::fmt(&tp),
        });
    }
    if out.is_empty() {
        return Err(Error::new("error.posiz.noSites").with("transform", transform));
    }
    check_formulas(&out)?;
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
