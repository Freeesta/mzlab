//! Condensed Fukui functions (Yang and Mortier 1986, doi:10.1021/ja00279a008) from the Mulliken charges of N, N+1 and N−1 electrons
//! at the same geometry, and the table of the C–H sites by symmetry class (`posizioni::symmetry_classes`).

use crate::mol3d::Mol3d;
use mzlab_feat::chem::grafo::Mol;
use mzlab_feat::chem::posizioni::symmetry_classes;

/// Two sites whose f(−) differ by less than this are a tie.
pub const TIE: f64 = 0.005;

/// Per-atom values (all atoms, hydrogens included); `None` where the method gives no value.
pub struct Fukui {
    pub q: Vec<Option<f64>>,
    pub f_plus: Vec<f64>,
    pub f_zero: Vec<Option<f64>>,
    pub f_minus: Vec<f64>,
}

impl Fukui {
    /// From the charges of the neutral molecule `q0`, the anion `qa` (N+1) and the cation `qc` (N−1).
    pub fn from_charges(q0: &[f64], qa: &[f64], qc: &[f64]) -> Fukui {
        let f_plus: Vec<f64> = q0.iter().zip(qa).map(|(n, a)| n - a).collect();
        let f_minus: Vec<f64> = qc.iter().zip(q0).map(|(c, n)| c - n).collect();
        Fukui {
            q: q0.iter().map(|&v| Some(v)).collect(),
            f_zero: f_plus
                .iter()
                .zip(&f_minus)
                .map(|(p, m)| Some(0.5 * (p + m)))
                .collect(),
            f_plus,
            f_minus,
        }
    }

    pub fn dual(&self, i: usize) -> f64 {
        self.f_plus[i] - self.f_minus[i]
    }
}

/// One C–H site: the heavy atoms of a symmetry class (0-based `heavy_index`), their hydrogens and the mean f(−).
pub struct Site {
    pub class: String,
    pub heavy_index: usize,
    pub n_h: usize,
    pub f_minus: f64,
    pub rank: usize,
    /// Within `TIE` of the first site.
    pub tie_first: bool,
}

/// Unsaturated carbons bearing hydrogens, grouped by symmetry class, highest mean f(−) first.
pub fn ch_sites(mol: &Mol, m: &Mol3d, f_minus: &[f64]) -> Vec<Site> {
    let cls = symmetry_classes(mol);
    let mut groups: Vec<(u32, Vec<usize>, usize)> = Vec::new();
    for (i, hi) in m.heavy_index.iter().enumerate() {
        let Some(h) = *hi else { continue };
        let n_h = m
            .bonds
            .iter()
            .filter(|b| (b.0 == i && m.el[b.1] == 1) || (b.1 == i && m.el[b.0] == 1))
            .count();
        let unsat = m.arom[i] || m.bonds.iter().any(|b| (b.0 == i || b.1 == i) && b.2 >= 2);
        if m.el[i] != 6 || n_h == 0 || !unsat {
            continue;
        }
        match groups.iter_mut().find(|g| g.0 == cls[h]) {
            Some(g) => {
                g.1.push(i);
                g.2 += n_h;
            }
            None => groups.push((cls[h], vec![i], n_h)),
        }
    }
    let mut sites: Vec<Site> = groups
        .into_iter()
        .map(|(_, atoms, n_h)| Site {
            class: atoms
                .iter()
                .map(|&i| format!("C{}", m.heavy_index[i].unwrap_or(i) + 1))
                .collect::<Vec<_>>()
                .join(", "),
            heavy_index: m.heavy_index[atoms[0]].unwrap_or(atoms[0]),
            n_h,
            f_minus: atoms.iter().map(|&i| f_minus[i]).sum::<f64>() / atoms.len() as f64,
            rank: 0,
            tie_first: false,
        })
        .collect();
    sites.sort_by(|a, b| b.f_minus.total_cmp(&a.f_minus));
    let top = sites.first().map_or(0.0, |s| s.f_minus);
    for (k, s) in sites.iter_mut().enumerate() {
        s.rank = k + 1;
        s.tie_first = top - s.f_minus < TIE;
    }
    sites
}
