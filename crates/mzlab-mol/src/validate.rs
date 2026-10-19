//! Rules of `mol/1` that the types cannot say: indices in range, Kekulé orders, refs that are real neighbours, status consistent with the
//! fields present, groups on defined centres, layer lengths. A record that passes is safe to index without further checks.

use crate::codec::MolError;
use crate::elements::is_element;
use crate::model::{Cfg, Mol, Parity, Status, Stereo, FORMAT};
use std::collections::BTreeSet;

type Res = Result<(), MolError>;

fn bad(msg: String) -> Res {
    Err(MolError::Invalid(msg))
}

/// Neighbour lists of the bonds, after the bonds themselves passed `check_bonds`.
fn adjacency(m: &Mol) -> Vec<Vec<u32>> {
    let mut adj = vec![Vec::new(); m.atoms.len()];
    for b in &m.bonds {
        if let Some(v) = adj.get_mut(b.a as usize) {
            v.push(b.b);
        }
        if let Some(v) = adj.get_mut(b.b as usize) {
            v.push(b.a);
        }
    }
    adj
}

/// Strictly ascending: sorted and without repeats.
fn ascending(v: &[u32]) -> bool {
    v.iter().zip(v.iter().skip(1)).all(|(x, y)| x < y)
}

fn bonded(adj: &[Vec<u32>], x: u32, y: u32) -> bool {
    adj.get(x as usize).is_some_and(|v| v.contains(&y))
}

impl Mol {
    /// Checks every rule of the format; `from_json` calls it, builders should call it before trusting a record.
    pub fn validate(&self) -> Result<(), MolError> {
        if self.format != FORMAT {
            return bad(format!("format is {:?}, expected {FORMAT:?}", self.format));
        }
        self.check_atoms()?;
        self.check_bonds()?;
        let adj = adjacency(self);
        self.check_stereo(&adj)?;
        self.check_groups()?;
        self.check_layers()
    }

    fn check_atoms(&self) -> Res {
        for (i, a) in self.atoms.iter().enumerate() {
            if !is_element(&a.el) {
                return bad(format!("atoms[{i}].el {:?} is not an element symbol", a.el));
            }
        }
        Ok(())
    }

    fn check_bonds(&self) -> Res {
        let n = self.atoms.len() as u32;
        let mut seen = BTreeSet::new();
        for (i, b) in self.bonds.iter().enumerate() {
            if b.a >= b.b || b.b >= n {
                return bad(format!(
                    "bonds[{i}] needs a < b < {n}, got {} and {}",
                    b.a, b.b
                ));
            }
            if !(1..=3).contains(&b.o) {
                return bad(format!(
                    "bonds[{i}].o is {}, Kekulé orders are 1, 2 or 3",
                    b.o
                ));
            }
            if !seen.insert((b.a, b.b)) {
                return bad(format!("bonds[{i}] repeats the bond {}-{}", b.a, b.b));
            }
        }
        Ok(())
    }

    fn check_stereo(&self, adj: &[Vec<u32>]) -> Res {
        let mut centres = BTreeSet::new();
        for (i, s) in self.stereo.iter().enumerate() {
            let fresh = match s {
                Stereo::Tetra { c, .. } => centres.insert((0u8, *c, 0)),
                Stereo::Db { a, b, .. } => centres.insert((1u8, *a, *b)),
            };
            if !fresh {
                return bad(format!("stereo[{i}] repeats an earlier element"));
            }
            match s {
                Stereo::Tetra {
                    c,
                    refs,
                    implicit_h,
                    par,
                    status,
                } => self.check_tetra(adj, (*c, refs, implicit_h, par, status), i)?,
                Stereo::Db {
                    a,
                    b,
                    refs,
                    cfg,
                    status,
                } => self.check_db(adj, (*a, *b, refs, cfg, status), i)?,
            }
        }
        Ok(())
    }

    fn check_tetra(
        &self,
        adj: &[Vec<u32>],
        t: (
            u32,
            &Option<Vec<u32>>,
            &Option<bool>,
            &Option<Parity>,
            &Status,
        ),
        i: usize,
    ) -> Res {
        let (c, refs, implicit_h, par, status) = t;
        let Some(atom) = self.atoms.get(c as usize) else {
            return bad(format!("stereo[{i}].c {c} is out of range"));
        };
        if *status == Status::Unknown {
            return if refs.is_some() || implicit_h.is_some() || par.is_some() {
                bad(format!(
                    "stereo[{i}] is unknown but carries refs, implicitH or par"
                ))
            } else {
                Ok(())
            };
        }
        let (Some(refs), Some(implicit_h), Some(_)) = (refs, implicit_h, par) else {
            return bad(format!(
                "stereo[{i}] is defined and needs refs, implicitH and par"
            ));
        };
        if !(3..=4).contains(&refs.len()) || !ascending(refs) {
            return bad(format!(
                "stereo[{i}].refs must be 3 or 4 strictly ascending indices"
            ));
        }
        if refs.iter().any(|&r| !bonded(adj, c, r)) {
            return bad(format!(
                "stereo[{i}].refs must all be bonded to the centre {c}"
            ));
        }
        // three refs leave one implicit position: a hydrogen (h = 1) or a lone pair (h = 0)
        let hydrogen = refs.len() == 3 && atom.h == 1;
        if (refs.len() == 4 && *implicit_h)
            || (refs.len() == 3 && *implicit_h != hydrogen)
            || (refs.len() == 3 && atom.h > 1)
        {
            return bad(format!(
                "stereo[{i}].implicitH disagrees with the hydrogens of atom {c}"
            ));
        }
        Ok(())
    }

    fn check_db(
        &self,
        adj: &[Vec<u32>],
        d: (u32, u32, &Option<[u32; 2]>, &Option<Cfg>, &Status),
        i: usize,
    ) -> Res {
        let (a, b, refs, cfg, status) = d;
        let double = self.bonds.iter().any(|x| x.a == a && x.b == b && x.o == 2);
        if !double {
            return bad(format!(
                "stereo[{i}] needs a double bond {a}-{b} with a < b"
            ));
        }
        if *status == Status::Unknown {
            return if refs.is_some() || cfg.is_some() {
                bad(format!("stereo[{i}] is unknown but carries refs or cfg"))
            } else {
                Ok(())
            };
        }
        let (Some([r1, r2]), Some(_)) = (refs, cfg) else {
            return bad(format!("stereo[{i}] is defined and needs refs and cfg"));
        };
        if *r1 == b || *r2 == a || !bonded(adj, a, *r1) || !bonded(adj, b, *r2) {
            return bad(format!("stereo[{i}].refs must be a neighbour of {a} and one of {b}, other than the bond itself"));
        }
        Ok(())
    }

    fn check_groups(&self) -> Res {
        let mut used = BTreeSet::new();
        for (i, g) in self.groups.iter().enumerate() {
            if g.atoms.is_empty() || !ascending(&g.atoms) {
                return bad(format!(
                    "groups[{i}].atoms must be non-empty and strictly ascending"
                ));
            }
            for &a in &g.atoms {
                let defined = self.stereo.iter().any(
                    |s| matches!(s, Stereo::Tetra { c, status: Status::Defined, .. } if *c == a),
                );
                if !defined {
                    return bad(format!(
                        "groups[{i}] lists atom {a}, which is not a defined tetrahedral centre"
                    ));
                }
                if !used.insert(a) {
                    return bad(format!(
                        "groups[{i}] lists atom {a}, already in another group"
                    ));
                }
            }
        }
        Ok(())
    }

    fn check_layers(&self) -> Res {
        let n = self.atoms.len();
        if let Some(c) = &self.coords {
            if !(2..=3).contains(&c.dim)
                || c.xy.len() != n
                || c.xy.iter().any(|r| r.len() != c.dim as usize)
            {
                return bad(format!("coords needs {n} rows of {} numbers", c.dim));
            }
        }
        for p in &self.props {
            if p.v.len() != n {
                return bad(format!(
                    "props {:?} has {} values for {n} atoms",
                    p.name,
                    p.v.len()
                ));
            }
        }
        for (i, r) in self.regions.iter().enumerate() {
            if r.atoms.iter().any(|&a| a as usize >= n)
                || r.support.is_some_and(|s| !(0.0..=1.0).contains(&s))
            {
                return bad(format!(
                    "regions[{i}] has an atom out of range or a support outside 0..1"
                ));
            }
        }
        Ok(())
    }
}
