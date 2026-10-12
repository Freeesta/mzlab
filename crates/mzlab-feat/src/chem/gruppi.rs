//! Functional groups of a structure: the preconditions of `regole_tp/2`. Names are in `docs/agenti/mzfeat-tp.md`.

use std::collections::BTreeSet;

use super::grafo::Mol;

impl Mol {
    /// Names of the functional groups present, sorted. They are the keys of the `gruppi` preconditions of `regole_tp/2`.
    pub fn groups(&self) -> Vec<&'static str> {
        let mut g: BTreeSet<&'static str> = BTreeSet::new();
        for i in 0..self.atoms.len() {
            match self.atoms[i].el.as_str() {
                "F" => g.extend(["halogen", "fluoro"]),
                "Cl" => g.extend(["halogen", "chloro"]),
                "Br" => g.extend(["halogen", "bromo"]),
                "I" => g.extend(["halogen", "iodo"]),
                "C" => self.groups_c(i, &mut g),
                "N" => self.groups_n(i, &mut g),
                "O" => self.groups_o(i, &mut g),
                "S" => self.groups_s(i, &mut g),
                _ => {}
            }
        }
        for r in self.rings() {
            let sat = r.iter().all(|i| !self.atoms[*i].arom)
                && self
                    .bonds
                    .iter()
                    .filter(|b| r.contains(&b.a) && r.contains(&b.b))
                    .all(|b| b.order == 1.0);
            let hetero = r.iter().any(|i| matches!(self.el(*i), "N" | "O" | "S"));
            if sat && hetero {
                g.insert("ring_hetero_saturated");
            }
        }
        g.into_iter().collect()
    }

    fn groups_c(&self, i: usize, g: &mut BTreeSet<&'static str>) {
        let (a, nb) = (&self.atoms[i], &self.adj[i]);
        if a.arom {
            g.insert("aromatic_ring");
        }
        if self.is_carbonyl(i) {
            let single_o = |j: usize, k: usize| self.el(j) == "O" && self.bonds[k].order == 1.0;
            let o_h = nb.iter().any(|(j, k)| {
                single_o(*j, *k) && (self.atoms[*j].h > 0 || self.atoms[*j].charge < 0)
            });
            let o_c = nb
                .iter()
                .any(|(j, k)| single_o(*j, *k) && self.atoms[*j].h == 0 && self.heavy(*j) == 2);
            let n = nb.iter().any(|(j, _)| self.el(*j) == "N");
            let carbons = nb.iter().filter(|(j, _)| self.el(*j) == "C").count();
            g.insert(if o_h {
                "carboxylic_acid"
            } else if o_c {
                "ester"
            } else if n {
                "amide"
            } else if a.h > 0 || carbons == 1 && self.heavy(i) == 2 {
                "aldehyde"
            } else {
                "ketone"
            });
        }
        if self.sp3_c(i) && a.h > 0 && nb.iter().any(|(j, _)| self.hetero_alpha(*j)) {
            g.insert("alpha_ch_hetero");
        }
    }

    fn groups_n(&self, i: usize, g: &mut BTreeSet<&'static str>) {
        let (a, nb) = (&self.atoms[i], &self.adj[i]);
        let alkyl = |g: &mut BTreeSet<&'static str>| {
            for (j, _) in nb.iter().filter(|(j, _)| self.sp3_c(*j)) {
                g.insert("n_alkyl");
                self.alkyl_kind(*j, i, g);
            }
        };
        if nb.iter().filter(|(j, _)| self.el(*j) == "O").count() >= 2 {
            g.insert("nitro");
        } else if self.n_amine(i) {
            g.insert(match a.h {
                2 => "amine_primary",
                1 => "amine_secondary",
                _ => "amine_tertiary",
            });
            if nb.iter().any(|(j, _)| self.atoms[*j].arom) {
                g.insert("amine_aromatic");
            }
            alkyl(g);
        } else if nb.iter().any(|(j, _)| self.is_acyl(*j)) {
            alkyl(g);
            if nb.iter().any(|(j, _)| self.is_sulfonyl(*j)) {
                g.insert("sulfonamide");
            }
        } else if a.arom && nb.iter().any(|(j, _)| self.atoms[*j].arom) {
            g.insert("heteroaromatic");
        }
    }

    fn groups_o(&self, i: usize, g: &mut BTreeSet<&'static str>) {
        let (a, nb) = (&self.atoms[i], &self.adj[i]);
        let singles = nb.iter().all(|(_, k)| self.bonds[*k].order == 1.0);
        if a.arom {
            g.insert("heteroaromatic");
        } else if singles && a.h > 0 && nb.len() == 1 {
            let c = nb[0].0;
            if self.atoms[c].arom {
                g.insert("phenol");
            } else if self.sp3_c(c) {
                g.insert("alcohol");
            }
        } else if singles && nb.len() == 2 && a.h == 0 {
            let (x, y) = (nb[0].0, nb[1].0);
            if self.el(x) == "C" && self.el(y) == "C" && !self.is_acyl(x) && !self.is_acyl(y) {
                g.insert("ether");
                if self.sp3_c(x) || self.sp3_c(y) {
                    g.insert("o_alkyl");
                }
            }
        }
    }

    fn groups_s(&self, i: usize, g: &mut BTreeSet<&'static str>) {
        let a = &self.atoms[i];
        if self.is_sulfonyl(i) {
            g.insert("sulfonyl");
        } else if self.dbl_o(i) == 1 {
            g.insert("sulfoxide");
        } else if self.dbl_o(i) == 0 && (self.heavy(i) == 2 || a.arom) {
            g.insert("sulfide");
        }
        if a.arom {
            g.insert("heteroaromatic");
        }
    }

    /// Heteroatom whose neighbouring C-H is activated (·OH abstracts there): an amine N, an ether or alcohol O.
    fn hetero_alpha(&self, j: usize) -> bool {
        match self.el(j) {
            "N" => self.n_amine(j) || self.adj[j].iter().any(|(x, _)| self.is_acyl(*x)),
            "O" => {
                !self.atoms[j].arom
                    && self.adj[j]
                        .iter()
                        .all(|(x, k)| self.bonds[*k].order == 1.0 && !self.is_acyl(*x))
            }
            _ => false,
        }
    }

    /// Amine nitrogen: not aromatic, no double bond, no charge, not next to an acyl or sulfonyl group.
    fn n_amine(&self, i: usize) -> bool {
        let a = &self.atoms[i];
        self.el(i) == "N"
            && !a.arom
            && a.charge == 0
            && self.adj[i].iter().all(|(j, k)| {
                self.bonds[*k].order == 1.0 && !self.is_acyl(*j) && self.el(*j) != "O"
            })
    }

    /// Methyl, isopropyl and tert-butyl on N (the literature rule of the N-dealkylation names them).
    fn alkyl_kind(&self, c: usize, n: usize, g: &mut BTreeSet<&'static str>) {
        let rest: Vec<usize> = self.adj[c]
            .iter()
            .map(|(j, _)| *j)
            .filter(|j| *j != n)
            .collect();
        let all_c = rest.iter().all(|j| self.sp3_c(*j));
        let h = self.atoms[c].h;
        if rest.is_empty() && h == 3 {
            g.insert("n_methyl");
        } else if all_c && rest.len() == 2 && h == 1 && rest.iter().all(|j| self.heavy(*j) == 1) {
            g.insert("n_isopropyl");
        } else if all_c && rest.len() == 3 && h == 0 && rest.iter().all(|j| self.heavy(*j) == 1) {
            g.insert("n_tbutyl");
        }
    }
}
