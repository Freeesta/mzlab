//! Substructure matching of a `SubQuery` in a `mol/1` record: depth-first subgraph monomorphism in the VF2 manner (query atoms in a
//! connected order, candidates among the neighbours of a matched atom, degree and bond checks before going deeper), with a cap on the
//! number of search steps so a symmetric query on a large target stops with an error instead of running away. Hydrogens: implicit
//! hydrogens of the query are ignored; an explicit query H on a heavy atom asks for at least that many hydrogens (implicit or explicit) on
//! its partner. Stereo is checked only on request, on complete matches. A match is a candidate for the eye, never an identification.

use crate::codec::MolError;
use crate::model::{Cfg, Mol, Parity, Status, Stereo};
use crate::query::{AtomTest, BondTest, SubQuery};
use std::collections::BTreeSet;

/// Options of a search.
#[derive(Clone, Copy, Debug)]
pub struct SubOptions {
    /// Defined tetrahedral centres and double bonds of the query must be defined and the same in the target.
    pub stereo: bool,
    /// At most this many matches (distinct sets of target atoms) are returned.
    pub max_matches: usize,
    /// At most this many search steps (atom assignments tried) per target; beyond, `MolError::Query`.
    pub max_steps: u64,
}

impl Default for SubOptions {
    fn default() -> Self {
        SubOptions {
            stereo: false,
            max_matches: 1000,
            max_steps: 2_000_000,
        }
    }
}

/// Bond classes compared by `BondTest::Order`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Class {
    Single,
    Double,
    Triple,
    Aromatic,
}

fn classes(m: &Mol) -> Vec<Class> {
    m.aromatic_bonds()
        .iter()
        .zip(&m.bonds)
        .map(|(&ar, b)| match (ar, b.o) {
            (true, _) => Class::Aromatic,
            (_, 1) => Class::Single,
            (_, 2) => Class::Double,
            _ => Class::Triple,
        })
        .collect()
}

/// Neighbours of every atom as (atom, bond index), ascending by atom.
fn adjacency(m: &Mol) -> Vec<Vec<(usize, usize)>> {
    let mut adj = vec![Vec::new(); m.atoms.len()];
    for (k, b) in m.bonds.iter().enumerate() {
        adj[b.a as usize].push((b.b as usize, k));
        adj[b.b as usize].push((b.a as usize, k));
    }
    adj.iter_mut().for_each(|l| l.sort_unstable());
    adj
}

/// The query without its mergeable hydrogens: kept atoms, their minimum hydrogen counts and the connected search order.
struct Plan {
    keep: Vec<bool>,
    min_h: Vec<u32>,
    order: Vec<usize>,
    /// For `order[k]`: an earlier atom of the order bonded to it, if any.
    parent: Vec<Option<usize>>,
}

fn plan(q: &SubQuery, adj: &[Vec<(usize, usize)>]) -> Plan {
    let n = q.mol.atoms.len();
    let (mut keep, mut min_h) = (vec![true; n], vec![0u32; n]);
    for (i, a) in q.mol.atoms.iter().enumerate() {
        let plain = a.el == "H" && a.iso == 0 && a.chg == 0 && q.atoms[i] == AtomTest::Element;
        let [(nb, k)] = adj[i][..] else { continue };
        if plain && q.mol.atoms[nb].el != "H" && q.mol.bonds[k].o == 1 {
            keep[i] = false;
            min_h[nb] += 1;
        }
    }
    let degree = |i: usize| adj[i].iter().filter(|(j, _)| keep[*j]).count();
    let (mut order, mut parent, mut seen) = (Vec::new(), Vec::new(), vec![false; n]);
    while let Some(start) = (0..n)
        .filter(|&i| keep[i] && !seen[i])
        .max_by_key(|&i| (degree(i), std::cmp::Reverse(i)))
    {
        seen[start] = true;
        let first = order.len();
        order.push(start);
        parent.push(None);
        let mut k = first;
        while k < order.len() {
            let at = order[k];
            for &(nb, _) in &adj[at] {
                if keep[nb] && !seen[nb] {
                    seen[nb] = true;
                    order.push(nb);
                    parent.push(Some(at));
                }
            }
            k += 1;
        }
    }
    Plan {
        keep,
        min_h,
        order,
        parent,
    }
}

const HALOGENS: [&str; 5] = ["F", "Cl", "Br", "I", "At"];

struct Search<'a> {
    q: &'a SubQuery,
    t: &'a Mol,
    qadj: Vec<Vec<(usize, usize)>>,
    tadj: Vec<Vec<(usize, usize)>>,
    qclass: Vec<Class>,
    tclass: Vec<Class>,
    t_h: Vec<u32>,
    plan: Plan,
    map: Vec<usize>,
    used: Vec<bool>,
    found: BTreeSet<Vec<usize>>,
    out: Vec<Vec<u32>>,
    steps: u64,
    opts: SubOptions,
}

const NONE: usize = usize::MAX;

impl Search<'_> {
    fn atom_ok(&self, qi: usize, ti: usize) -> bool {
        let (qa, ta) = (&self.q.mol.atoms[qi], &self.t.atoms[ti]);
        let el = ta.el.as_str();
        let ok = match &self.q.atoms[qi] {
            AtomTest::Element => {
                qa.el == ta.el
                    && (qa.chg == 0 || qa.chg == ta.chg)
                    && (qa.iso == 0 || qa.iso == ta.iso)
                    && (qa.rad == 0 || qa.rad == ta.rad)
            }
            AtomTest::List(e) => e.iter().any(|x| x == el),
            AtomTest::NotList(e) => !e.iter().any(|x| x == el),
            AtomTest::Any { h } => *h || el != "H",
            AtomTest::Hetero { h } => el != "C" && (*h || el != "H"),
            AtomTest::Halogen { h } => HALOGENS.contains(&el) || (*h && el == "H"),
        };
        let degree = self.qadj[qi]
            .iter()
            .filter(|(j, _)| self.plan.keep[*j])
            .count();
        ok && self.t_h[ti] >= self.plan.min_h[qi] && self.tadj[ti].len() >= degree
    }

    fn bond_ok(&self, qk: usize, tk: usize) -> bool {
        let c = self.tclass[tk];
        match self.q.bonds[qk] {
            BondTest::Order => self.qclass[qk] == c,
            BondTest::Aromatic => c == Class::Aromatic,
            BondTest::SingleOrDouble => matches!(c, Class::Single | Class::Double),
            BondTest::SingleOrAromatic => matches!(c, Class::Single | Class::Aromatic),
            BondTest::DoubleOrAromatic => matches!(c, Class::Double | Class::Aromatic),
            BondTest::Any => true,
        }
    }

    /// Every bond from `qi` to an already matched query atom has a partner bond from `ti` that passes.
    fn edges_ok(&self, qi: usize, ti: usize) -> bool {
        self.qadj[qi].iter().all(|&(qj, qk)| {
            let tj = self.map[qj];
            tj == NONE
                || self.tadj[ti]
                    .binary_search_by_key(&tj, |&(a, _)| a)
                    .is_ok_and(|p| self.bond_ok(qk, self.tadj[ti][p].1))
        })
    }

    fn step(&mut self, depth: usize) -> Result<bool, MolError> {
        if depth == self.plan.order.len() {
            self.record();
            return Ok(self.out.len() >= self.opts.max_matches);
        }
        let qi = self.plan.order[depth];
        let cands: Vec<usize> = match self.plan.parent[depth] {
            Some(p) => self.tadj[self.map[p]].iter().map(|&(a, _)| a).collect(),
            None => (0..self.t.atoms.len()).collect(),
        };
        for ti in cands {
            if self.used[ti] {
                continue;
            }
            self.steps += 1;
            if self.steps > self.opts.max_steps {
                return Err(MolError::Query(format!(
                    "the search stopped after {} steps (a very symmetric query or target): draw a more specific query",
                    self.opts.max_steps
                )));
            }
            if !self.atom_ok(qi, ti) || !self.edges_ok(qi, ti) {
                continue;
            }
            self.map[qi] = ti;
            self.used[ti] = true;
            let stop = self.step(depth + 1)?;
            self.map[qi] = NONE;
            self.used[ti] = false;
            if stop {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn record(&mut self) {
        let mut set: Vec<usize> = self.plan.order.iter().map(|&q| self.map[q]).collect();
        set.sort_unstable();
        if self.found.contains(&set) || (self.opts.stereo && !self.stereo_ok()) {
            return;
        }
        self.found.insert(set);
        let row = (0..self.q.mol.atoms.len())
            .filter(|&i| self.plan.keep[i])
            .map(|i| self.map[i] as u32)
            .collect();
        self.out.push(row);
    }

    fn stereo_ok(&self) -> bool {
        self.q.mol.stereo.iter().all(|s| match s {
            Stereo::Tetra {
                c,
                refs: Some(r),
                par: Some(p),
                status: Status::Defined,
                ..
            } => self.tetra_ok(*c as usize, r, *p),
            Stereo::Db {
                a,
                b,
                refs: Some(r),
                cfg: Some(c),
                status: Status::Defined,
            } => self.db_ok(*a as usize, *b as usize, r, *c),
            _ => true,
        })
    }

    /// The four places around a tetrahedral centre as target atoms (`NONE` = implicit H or lone pair) in the order of the record.
    fn tetra_ok(&self, c: usize, refs: &[u32], par: Parity) -> bool {
        let tc = self.map[c];
        let Some((trefs, tpar)) = self.t.stereo.iter().find_map(|s| match s {
            Stereo::Tetra {
                c,
                refs: Some(r),
                par: Some(p),
                status: Status::Defined,
                ..
            } if *c as usize == tc => Some((r, *p)),
            _ => None,
        }) else {
            return false;
        };
        let mut tslots: Vec<usize> = trefs.iter().map(|&r| r as usize).collect();
        let mut qslots: Vec<usize> = refs.iter().map(|&r| self.mapped(r as usize)).collect();
        for s in [&mut tslots, &mut qslots] {
            if s.len() == 3 {
                s.push(NONE);
            }
        }
        let rest: Vec<usize> = tslots
            .iter()
            .copied()
            .filter(|&t| t == NONE || !qslots.contains(&t))
            .collect();
        let open = qslots.iter().filter(|&&s| s == NONE).count();
        if rest.len() != open || open > 1 {
            return open > 1;
        }
        for s in qslots.iter_mut().filter(|s| **s == NONE) {
            *s = rest[0];
        }
        let Some(perm) = qslots
            .iter()
            .map(|s| tslots.iter().position(|t| t == s))
            .collect::<Option<Vec<_>>>()
        else {
            return false;
        };
        (par == tpar) != odd(&perm)
    }

    fn mapped(&self, q: usize) -> usize {
        if self.plan.keep[q] {
            self.map[q]
        } else {
            NONE
        }
    }

    fn db_ok(&self, a: usize, b: usize, refs: &[u32; 2], cfg: Cfg) -> bool {
        let (ta, tb) = (self.map[a], self.map[b]);
        let (r0, r1) = (self.mapped(refs[0] as usize), self.mapped(refs[1] as usize));
        if r0 == NONE || r1 == NONE {
            return true;
        }
        let found = self.t.stereo.iter().find_map(|s| match s {
            Stereo::Db {
                a,
                b,
                refs: Some(r),
                cfg: Some(c),
                status: Status::Defined,
            } if (*a as usize, *b as usize) == (ta.min(tb), ta.max(tb)) => {
                Some((*a as usize, r, *c))
            }
            _ => None,
        });
        let Some((low, trefs, tcfg)) = found else {
            return false;
        };
        let (sa, sb) = if low == ta {
            (trefs[0], trefs[1])
        } else {
            (trefs[1], trefs[0])
        };
        let flip = (sa as usize != r0) != (sb as usize != r1);
        (tcfg == cfg) != flip
    }
}

fn odd(perm: &[usize]) -> bool {
    let mut inv = 0;
    for i in 0..perm.len() {
        inv += perm[i + 1..].iter().filter(|&&x| x < perm[i]).count();
    }
    inv % 2 == 1
}

/// Total hydrogens of every atom: implicit plus explicit H neighbours.
fn hydrogens(m: &Mol, adj: &[Vec<(usize, usize)>]) -> Vec<u32> {
    m.atoms
        .iter()
        .zip(adj)
        .map(|(a, l)| {
            u32::from(a.h) + l.iter().filter(|(j, _)| m.atoms[*j].el == "H").count() as u32
        })
        .collect()
}

impl SubQuery {
    /// The matches in `target`: one row per distinct set of target atoms, the target atom of every kept query atom in query order
    /// (merged hydrogens have no column). Empty when the query does not occur; `MolError::Query` when the step cap is reached.
    pub fn matches(&self, target: &Mol, opts: &SubOptions) -> Result<Vec<Vec<u32>>, MolError> {
        let qadj = adjacency(&self.mol);
        let tadj = adjacency(target);
        let plan = plan(self, &qadj);
        let mut s = Search {
            q: self,
            t: target,
            qclass: classes(&self.mol),
            tclass: classes(target),
            t_h: hydrogens(target, &tadj),
            map: vec![NONE; self.mol.atoms.len()],
            used: vec![false; target.atoms.len()],
            qadj,
            tadj,
            plan,
            found: BTreeSet::new(),
            out: Vec::new(),
            steps: 0,
            opts: *opts,
        };
        if s.plan.order.is_empty()
            || s.plan.order.len() > target.atoms.len()
            || opts.max_matches == 0
        {
            return Ok(Vec::new());
        }
        s.step(0)?;
        Ok(s.out)
    }

    /// The number of query atoms a match maps (atoms without the merged hydrogens).
    pub fn mapped_atoms(&self) -> usize {
        plan(self, &adjacency(&self.mol)).order.len()
    }
}
