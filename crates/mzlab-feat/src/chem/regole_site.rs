//! Structure checks for the rules of `fragrules/1` (`docs/studi/regole-frammentazione-schema.md` §3): the groups added to `Mol::groups()` and the
//! named `site` functions, with no SMARTS. Used by `regole::attese`; `Mol::groups()` itself (so `regole_tp/2`) is not changed.

use std::collections::BTreeSet;

use super::grafo::Mol;
use mzlab_core::elements::fmt;

/// What a `site` gives for a rule: nothing (absent), the site only (the rule's own formula holds) or products with their formula; `true` = an ion.
pub(super) type Prodotti = Vec<(Option<String>, bool)>;

fn push_unique(out: &mut Prodotti, f: String, ione: bool) {
    if !out.iter().any(|(x, _)| x.as_deref() == Some(&f)) {
        out.push((Some(f), ione));
    }
}

/// The names of the sites; a rule of the table that names another is rejected by the test of the table.
pub const SITI: [&str; 17] = [
    "amide_nh2",
    "n_dimethyl",
    "n_ethyl",
    "gamma_h_carbonyl",
    "methyl_ester",
    "methoxy_near_charge",
    "acetate",
    "acetyl_on_heteroatom",
    "chromone",
    "cyclohexene_ring",
    "so3h",
    "methoxy_aryl",
    "ureido_nh2",
    "n_methylcarbamate",
    "benzyl",
    "alpha_c_n",
    "acyl",
];

/// The groups beyond `Mol::groups()` that the rules name.
pub const GRUPPI_NUOVI: [&str; 4] = ["phosphate", "sulfate", "o_glycoside", "glucuronide"];

impl Mol {
    /// `groups()` plus the groups of the rules of fragmentation (`phosphate`, `sulfate`, `o_glycoside`, `glucuronide`), sorted.
    pub fn groups_ext(&self) -> Vec<&'static str> {
        let mut g: BTreeSet<&'static str> = self.groups().into_iter().collect();
        for i in 0..self.atoms.len() {
            match self.el(i) {
                "P" if self.dbl_o(i) >= 1 && self.o_neighbours(i) >= 3 => {
                    g.insert("phosphate");
                }
                "S" if self.is_sulfonyl(i) && self.single_o(i) >= 2 => {
                    g.insert("sulfate");
                }
                _ => {}
            }
        }
        for r in self.rings() {
            if let Some(gluc) = self.glycoside_ring(&r) {
                g.insert("o_glycoside");
                if gluc {
                    g.insert("glucuronide");
                }
            }
        }
        g.into_iter().collect()
    }

    /// The products of the site `nome`; `None` for a name that is not one of `SITI`.
    pub(super) fn site(&self, nome: &str) -> Option<Prodotti> {
        let si = |b: bool| if b { vec![(None, false)] } else { vec![] };
        Some(match nome {
            "amide_nh2" => si(self.any_atom(|i| self.nh2_on(i, false))),
            "ureido_nh2" => si(self.any_atom(|i| self.nh2_on(i, true))),
            "n_dimethyl" => si(self.any_atom(|i| self.n_amine_h0(i) && self.methyls(i) >= 2)),
            "n_ethyl" => si(self.any_atom(|i| self.ethyl_on_n(i))),
            "methyl_ester" => si(self.any_atom(|i| self.ester_o(i, |c| self.is_methyl(c)))),
            "acetate" => {
                si(self
                    .any_atom(|i| self.is_carbonyl(i) && self.has_methyl(i) && self.single_o_c(i)))
            }
            "acetyl_on_heteroatom" => si(self.any_atom(|i| {
                self.is_carbonyl(i)
                    && self.heavy(i) == 3
                    && self.has_methyl(i)
                    && self.adj[i].iter().any(|(j, k)| {
                        matches!(self.el(*j), "N" | "O") && self.bonds[*k].order == 1.0
                    })
            })),
            "methoxy_aryl" => si(self.any_atom(|i| self.methoxy(i, Some(true)))),
            "methoxy_near_charge" => si(self.methoxy_near_amine()),
            "so3h" => si(self.any_atom(|i| {
                self.is_sulfonyl(i)
                    && self.adj[i].iter().any(|(j, k)| {
                        self.el(*j) == "O"
                            && self.bonds[*k].order == 1.0
                            && (self.atoms[*j].h > 0 || self.atoms[*j].charge < 0)
                    })
            })),
            "n_methylcarbamate" => si(self.any_atom(|i| self.n_methylcarbamate(i))),
            "chromone" => si(self.chromone()),
            "gamma_h_carbonyl" => self.mclafferty_ester(),
            "cyclohexene_ring" => self.rda_pieces(),
            "benzyl" => self.benzyl_ions(),
            "alpha_c_n" => self.immonium_ions(),
            "acyl" => self.acylium_ions(),
            _ => return None,
        })
    }

    fn any_atom(&self, f: impl Fn(usize) -> bool) -> bool {
        (0..self.atoms.len()).any(f)
    }

    fn o_neighbours(&self, i: usize) -> usize {
        self.adj[i]
            .iter()
            .filter(|(j, _)| self.el(*j) == "O")
            .count()
    }

    fn single_o(&self, i: usize) -> usize {
        self.adj[i]
            .iter()
            .filter(|(j, k)| self.el(*j) == "O" && self.bonds[*k].order == 1.0)
            .count()
    }

    /// A methyl: a carbon with three H and one heavy neighbour.
    fn is_methyl(&self, i: usize) -> bool {
        self.el(i) == "C" && self.atoms[i].h == 3 && self.heavy(i) == 1
    }

    fn methyls(&self, n: usize) -> usize {
        self.adj[n]
            .iter()
            .filter(|(j, _)| self.is_methyl(*j))
            .count()
    }

    fn has_methyl(&self, c: usize) -> bool {
        self.methyls(c) >= 1
    }

    /// A carbonyl carbon with a single-bonded O that has a second carbon neighbour (an ester).
    fn single_o_c(&self, c: usize) -> bool {
        self.ester_o(c, |_| true)
    }

    /// The carbonyl `c` has an ester oxygen whose other neighbour passes `f`.
    fn ester_o(&self, c: usize, f: impl Fn(usize) -> bool) -> bool {
        self.is_carbonyl(c)
            && self.adj[c].iter().any(|(o, k)| {
                self.el(*o) == "O"
                    && self.bonds[*k].order == 1.0
                    && self.atoms[*o].h == 0
                    && self.heavy(*o) == 2
                    && self.adj[*o]
                        .iter()
                        .any(|(x, _)| *x != c && self.el(*x) == "C" && f(*x))
            })
    }

    /// An NH2 on a carbonyl (amide); with `urea`, the carbonyl has a second nitrogen.
    fn nh2_on(&self, n: usize, urea: bool) -> bool {
        self.el(n) == "N"
            && self.atoms[n].h == 2
            && self.adj[n].iter().any(|(c, _)| {
                let ns = self.adj[*c]
                    .iter()
                    .filter(|(x, _)| self.el(*x) == "N")
                    .count();
                self.is_carbonyl(*c) && (!urea || ns >= 2)
            })
    }

    /// A tertiary amine nitrogen.
    fn n_amine_h0(&self, n: usize) -> bool {
        self.el(n) == "N" && self.atoms[n].h == 0 && self.n_amine(n) && self.heavy(n) == 3
    }

    fn n_alkyl_any(&self, n: usize) -> bool {
        self.el(n) == "N" && self.n_amine(n)
    }

    /// An ethyl on a nitrogen: `c` is the CH2 of N-CH2-CH3.
    fn ethyl_on_n(&self, c: usize) -> bool {
        self.sp3_c(c)
            && self.atoms[c].h == 2
            && self.heavy(c) == 2
            && self.adj[c]
                .iter()
                .any(|(n, _)| self.el(*n) == "N" && self.n_alkyl_any(*n))
            && self.adj[c].iter().any(|(m, _)| self.is_methyl(*m))
    }

    /// A methoxy oxygen (O-CH3 on a carbon); `aryl`: the carbon must (or must not) be aromatic.
    fn methoxy(&self, o: usize, aryl: Option<bool>) -> bool {
        self.el(o) == "O"
            && self.atoms[o].h == 0
            && self.heavy(o) == 2
            && self.adj[o].iter().any(|(m, _)| self.is_methyl(*m))
            && self.adj[o].iter().any(|(x, _)| {
                !self.is_methyl(*x)
                    && self.el(*x) == "C"
                    && aryl.is_none_or(|a| self.atoms[*x].arom == a)
            })
    }

    /// Bonds from `n` to every atom (`usize::MAX` when not connected).
    fn distances(&self, n: usize) -> Vec<usize> {
        let mut dist = vec![usize::MAX; self.atoms.len()];
        dist[n] = 0;
        let mut q = vec![n];
        while !q.is_empty() {
            let mut next = vec![];
            for u in q {
                for (v, _) in &self.adj[u] {
                    if dist[*v] == usize::MAX {
                        dist[*v] = dist[u] + 1;
                        next.push(*v);
                    }
                }
            }
            q = next;
        }
        dist
    }

    /// A methoxy ether within three bonds of a basic amine nitrogen (the proton sits there and the methoxy leaves with it).
    fn methoxy_near_amine(&self) -> bool {
        (0..self.atoms.len()).any(|n| {
            self.n_amine(n) && {
                let d = self.distances(n);
                (0..self.atoms.len()).any(|o| self.methoxy(o, None) && d[o] <= 3)
            }
        })
    }

    /// N-methyl carbamate: CH3-N-C(=O)-O.
    fn n_methylcarbamate(&self, n: usize) -> bool {
        self.el(n) == "N"
            && self.methyls(n) >= 1
            && self.adj[n]
                .iter()
                .any(|(c, _)| self.is_carbonyl(*c) && self.single_o(*c) >= 1)
    }

    /// A six-membered ring with one oxygen and a ketone carbon, fused with an aromatic six-membered ring.
    fn chromone(&self) -> bool {
        let rings = self.rings();
        rings.iter().any(|r| {
            r.len() == 6
                && r.iter().filter(|i| self.el(**i) == "O").count() == 1
                && r.iter().any(|i| self.dbl_o(*i) == 1 && self.el(*i) == "C")
                && rings.iter().any(|r2| {
                    r2 != r
                        && r2.len() == 6
                        && r2.iter().all(|i| self.atoms[*i].arom)
                        && r2.iter().filter(|i| r.contains(i)).count() == 2
                })
        })
    }

    /// The atoms of a ring in the order of the cycle.
    fn ring_order(&self, ring: &[usize]) -> Vec<usize> {
        let mut out = vec![ring[0]];
        while out.len() < ring.len() {
            let last = *out.last().unwrap_or(&ring[0]);
            let next = self.adj[last]
                .iter()
                .map(|(j, _)| *j)
                .find(|j| ring.contains(j) && !out.contains(j));
            match next {
                Some(j) => out.push(j),
                None => break,
            }
        }
        out
    }

    /// The bond index between two atoms.
    fn bond_of(&self, a: usize, b: usize) -> Option<usize> {
        self.adj[a].iter().find(|(j, _)| *j == b).map(|(_, k)| *k)
    }

    /// The formula of the atoms reachable from `from` without crossing the bonds `cut`; `None` when `block` is reached (the molecule stays connected).
    fn piece(&self, from: usize, cut: &[usize], block: &[usize]) -> Option<String> {
        let mut seen = vec![false; self.atoms.len()];
        seen[from] = true;
        let mut st = vec![from];
        while let Some(u) = st.pop() {
            for (v, k) in &self.adj[u] {
                if !cut.contains(k) && !seen[*v] {
                    seen[*v] = true;
                    st.push(*v);
                }
            }
        }
        if block.iter().any(|b| seen[*b]) {
            None
        } else {
            Some(fmt(&self.formula_of(&seen)))
        }
    }

    /// The two pieces of a retro-Diels-Alder of a cyclohexene ring (the bonds two atoms away from the double bond).
    fn rda_pieces(&self) -> Prodotti {
        let mut out: Prodotti = vec![];
        for r in self.rings() {
            let plain =
                r.len() == 6 && r.iter().all(|i| self.el(*i) == "C" && !self.atoms[*i].arom);
            if !plain {
                continue;
            }
            let o = self.ring_order(&r);
            if o.len() != 6 {
                continue;
            }
            let order = |a: usize, b: usize| {
                self.bond_of(o[a], o[b])
                    .map_or(0.0, |k| self.bonds[k].order)
            };
            let doubles: Vec<usize> = (0..6).filter(|p| order(*p, (p + 1) % 6) == 2.0).collect();
            let singles = (0..6).filter(|p| order(*p, (p + 1) % 6) == 1.0).count();
            if doubles.len() != 1 || singles != 5 {
                continue;
            }
            // rotate so the double bond is o[0]=o[1]; the cut bonds are o[2]-o[3] and o[4]-o[5]
            let s = doubles[0];
            let at = |p: usize| o[(s + p) % 6];
            let cuts = [self.bond_of(at(2), at(3)), self.bond_of(at(4), at(5))];
            let (Some(k1), Some(k2)) = (cuts[0], cuts[1]) else {
                continue;
            };
            let (ene, diene) = (
                self.piece(at(3), &[k1, k2], &[at(2), at(5)]),
                self.piece(at(0), &[k1, k2], &[at(3)]),
            );
            for f in [ene, diene].into_iter().flatten() {
                push_unique(&mut out, f, false);
            }
        }
        out
    }

    /// The alkenes an ester alkyl of two or more carbons loses by a McLafferty-type rearrangement: the alkyl piece minus one H.
    fn mclafferty_ester(&self) -> Prodotti {
        let ring = self.ring_bonds();
        let mut out: Prodotti = vec![];
        for c in (0..self.atoms.len()).filter(|c| self.is_carbonyl(*c)) {
            for (o, k0) in &self.adj[c] {
                if self.el(*o) != "O" || self.bonds[*k0].order != 1.0 || self.heavy(*o) != 2 {
                    continue;
                }
                for (a, k) in self.adj[*o].iter().filter(|(a, _)| *a != c) {
                    let chain = self.sp3_c(*a)
                        && self.adj[*a]
                            .iter()
                            .any(|(b, _)| *b != *o && self.sp3_c(*b) && self.atoms[*b].h > 0);
                    if !chain || ring[*k] {
                        continue;
                    }
                    let keep = self.side_of(*k, *a);
                    if (0..self.atoms.len()).any(|i| keep[i] && self.el(i) != "C") {
                        continue;
                    }
                    let mut f = self.formula_of(&keep);
                    if let Some(h) = f.iter_mut().find(|(e, _)| e == "H") {
                        h.1 = h.1.saturating_sub(1);
                    }
                    push_unique(&mut out, fmt(&f), false);
                }
            }
        }
        out
    }

    /// The atoms on the side of atom `at` when the (acyclic) bond `k` is cut.
    fn side_of(&self, k: usize, at: usize) -> Vec<bool> {
        let (a, b) = self.sides(k);
        if a[at] {
            a
        } else {
            b
        }
    }

    /// The formula of the side of atom `at` when the bond `k` is cut (the cation has that formula).
    fn cation(&self, k: usize, at: usize) -> String {
        fmt(&self.formula_of(&self.side_of(k, at)))
    }

    /// The tropylium ion C7H7: a benzylic carbon whose cut leaves an unsubstituted phenyl-CH2.
    fn benzyl_ions(&self) -> Prodotti {
        let ring = self.ring_bonds();
        let mut out: Prodotti = vec![];
        for c in (0..self.atoms.len()).filter(|c| self.sp3_c(*c) && self.atoms[*c].h >= 1) {
            if !self.adj[c]
                .iter()
                .any(|(j, _)| self.atoms[*j].arom && self.el(*j) == "C")
            {
                continue;
            }
            for (_, k) in self.adj[c]
                .iter()
                .filter(|(j, k)| !self.atoms[*j].arom && !ring[*k])
            {
                if self.cation(*k, c) == "C7H7" {
                    push_unique(&mut out, "C7H7".into(), true);
                }
            }
        }
        out
    }

    /// The iminium ions of the alpha cleavages of the amines: the N side of each acyclic C(alpha)-C(beta) bond.
    fn immonium_ions(&self) -> Prodotti {
        let ring = self.ring_bonds();
        let mut out: Prodotti = vec![];
        for n in (0..self.atoms.len()).filter(|n| self.n_alkyl_any(*n)) {
            for (ca, _) in self.adj[n].iter().filter(|(c, _)| self.sp3_c(*c)) {
                for (_, k) in self.adj[*ca]
                    .iter()
                    .filter(|(b, k)| *b != n && self.el(*b) == "C" && !ring[*k])
                {
                    push_unique(&mut out, self.cation(*k, n), true);
                }
            }
        }
        out
    }

    /// The acylium ions R-C#O+: the carbonyl side of each acyclic single bond of a carbonyl carbon.
    fn acylium_ions(&self) -> Prodotti {
        let ring = self.ring_bonds();
        let mut out: Prodotti = vec![];
        for c in (0..self.atoms.len()).filter(|c| self.is_carbonyl(*c)) {
            for (_, k) in self.adj[c]
                .iter()
                .filter(|(_, k)| self.bonds[*k].order == 1.0 && !ring[*k])
            {
                push_unique(&mut out, self.cation(*k, c), true);
            }
        }
        out
    }

    /// `Some(is_glucuronide)` when the ring is a pyranose or furanose that carries an O-glycosidic bond at its anomeric carbon.
    fn glycoside_ring(&self, r: &[usize]) -> Option<bool> {
        if !(r.len() == 5 || r.len() == 6) || r.iter().filter(|i| self.el(**i) == "O").count() != 1
        {
            return None;
        }
        let ring_o = *r.iter().find(|i| self.el(**i) == "O")?;
        if r.iter()
            .any(|i| self.atoms[*i].arom || (self.el(*i) != "C" && *i != ring_o))
        {
            return None;
        }
        let exo_o = |c: usize| {
            self.adj[c]
                .iter()
                .filter(|(o, k)| {
                    self.el(*o) == "O" && !r.contains(o) && self.bonds[*k].order == 1.0
                })
                .map(|(o, _)| *o)
                .collect::<Vec<usize>>()
        };
        if r.iter()
            .filter(|i| **i != ring_o && !exo_o(**i).is_empty())
            .count()
            < 3
        {
            return None;
        }
        let anomeric = r.iter().copied().find(|c| {
            self.adj[*c].iter().any(|(o, _)| *o == ring_o)
                && exo_o(*c)
                    .iter()
                    .any(|o| self.adj[*o].len() == 2 && self.atoms[*o].h == 0)
        })?;
        let other = r
            .iter()
            .copied()
            .find(|c| *c != anomeric && self.adj[*c].iter().any(|(o, _)| *o == ring_o))?;
        let gluc = r.len() == 6
            && self.adj[other].iter().any(|(x, _)| {
                !r.contains(x)
                    && self.is_carbonyl(*x)
                    && self.adj[*x].iter().any(|(o, k)| {
                        self.el(*o) == "O"
                            && self.bonds[*k].order == 1.0
                            && (self.atoms[*o].h > 0 || self.atoms[*o].charge < 0)
                    })
            });
        Some(gluc)
    }
}
