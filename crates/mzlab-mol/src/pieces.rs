//! The connected pieces of a `mol/1` record as records of their own (one SDF record per structure): atoms keep their relative order,
//! bonds, stereo, groups, coordinates, property layers and regions follow their atoms; the `id` is dropped (it named the whole record).

use crate::model::{Mol, Stereo, StereoGroup};
use crate::sortm::SortM;

impl Mol {
    /// Each connected piece as its own record, in the order of its first atom; atoms keep their relative order and their layers.
    pub fn pieces(&self) -> Vec<Mol> {
        let (piece, count) = self.piece_labels();
        if count <= 1 {
            return vec![self.clone()];
        }
        let mut out = Vec::new();
        for k in 0..count {
            out.extend(self.only_piece(&piece, k));
        }
        out
    }

    /// The atoms of each connected piece, ascending, pieces in the order of their first atom as `pieces` gives them.
    pub fn piece_members(&self) -> Vec<Vec<u32>> {
        let (piece, count) = self.piece_labels();
        let mut members = vec![Vec::new(); count];
        for (x, &k) in piece.iter().enumerate() {
            members[k].push(x as u32);
        }
        members
    }

    /// The piece of every atom (numbered in the order of its first atom) and the number of pieces.
    pub(crate) fn piece_labels(&self) -> (Vec<usize>, usize) {
        let n = self.atoms.len();
        let mut piece = vec![usize::MAX; n];
        let mut count = 0;
        let mut nb = vec![Vec::new(); n];
        for b in &self.bonds {
            nb[b.a as usize].push(b.b as usize);
            nb[b.b as usize].push(b.a as usize);
        }
        for s in 0..n {
            if piece[s] != usize::MAX {
                continue;
            }
            let mut stack = vec![s];
            piece[s] = count;
            while let Some(x) = stack.pop() {
                for &y in &nb[x] {
                    if piece[y] == usize::MAX {
                        piece[y] = count;
                        stack.push(y);
                    }
                }
            }
            count += 1;
        }
        (piece, count)
    }

    /// The core (atoms, bonds, stereo, groups restricted to it) of every part (atoms with one `label`, `count` parts) in one pass,
    /// atoms in their relative order: the core of `only_piece` for each, without renumbering the whole record once per part.
    pub(crate) fn part_cores(&self, label: &[usize], count: usize) -> Option<Vec<Mol>> {
        let mut parts = vec![Mol::new(Vec::new(), Vec::new(), Vec::new(), Vec::new()); count];
        let mut local = vec![0u32; label.len()];
        for (x, &k) in label.iter().enumerate() {
            local[x] = parts.get(k)?.atoms.len() as u32;
            parts[k].atoms.push(self.atoms[x].clone());
        }
        let at = |x: u32| label.get(x as usize).copied();
        for b in &self.bonds {
            let k = at(b.a)?;
            let mut b = b.clone();
            (b.a, b.b) = (local[b.a as usize], local[b.b as usize]);
            parts[k].bonds.push(b);
        }
        for x in &self.stereo {
            let mut x = x.clone();
            let k = match &mut x {
                Stereo::Tetra { c, refs, .. } => {
                    for r in refs.iter_mut().flatten() {
                        *r = local[*r as usize];
                    }
                    let k = at(*c)?;
                    *c = local[*c as usize];
                    k
                }
                Stereo::Db { a, b, refs, .. } => {
                    for r in refs.iter_mut().flatten() {
                        *r = local[*r as usize];
                    }
                    let k = at(*b)?;
                    (*a, *b) = (local[*a as usize], local[*b as usize]);
                    k
                }
            };
            parts[k].stereo.push(x);
        }
        for g in &self.groups {
            let mut by_part: Vec<(usize, u32)> = Vec::new();
            for &a in &g.atoms {
                by_part.push((at(a)?, local[a as usize]));
            }
            by_part.sortm();
            for (w, &(k, a)) in by_part.iter().enumerate() {
                if w == 0 || by_part[w - 1].0 != k {
                    parts[k].groups.push(StereoGroup {
                        kind: g.kind,
                        atoms: Vec::new(),
                    });
                }
                parts[k].groups.last_mut()?.atoms.push(a);
            }
        }
        // the identity renumbering sorts and normalises each part as the renumbering of `only_piece` does
        parts
            .iter()
            .map(|p| {
                p.renumbered(&(0..p.atoms.len() as u32).collect::<Vec<_>>())
                    .ok()
            })
            .collect()
    }

    pub(crate) fn only_piece(&self, piece: &[usize], k: usize) -> Option<Mol> {
        // plain loops: iterator adapters and `retain` closures cost the `.wasm` more
        let n = piece.len();
        let mut new_of_old = vec![0u32; n];
        let mut size = 0u32;
        for i in 0..n {
            if piece[i] == k {
                new_of_old[i] = size;
                size += 1;
            }
        }
        let mut next = size;
        for i in 0..n {
            if piece[i] != k {
                new_of_old[i] = next;
                next += 1;
            }
        }
        let mut m = self.renumbered(&new_of_old).ok()?;
        let s = size as usize;
        m.atoms.truncate(s);
        let mut bonds = Vec::new();
        for b in m.bonds {
            if b.b < size {
                bonds.push(b);
            }
        }
        m.bonds = bonds;
        let mut stereo = Vec::new();
        for x in m.stereo {
            let at = match x {
                Stereo::Tetra { c, .. } => c,
                Stereo::Db { b, .. } => b,
            };
            if at < size {
                stereo.push(x);
            }
        }
        m.stereo = stereo;
        let mut groups = Vec::new();
        for g in m.groups {
            let mut atoms = Vec::new();
            for a in g.atoms {
                if a < size {
                    atoms.push(a);
                }
            }
            if !atoms.is_empty() {
                groups.push(StereoGroup {
                    kind: g.kind,
                    atoms,
                });
            }
        }
        m.groups = groups;
        if let Some(c) = m.coords.as_mut() {
            c.xy.truncate(s);
        }
        for p in &mut m.props {
            p.v.truncate(s);
        }
        let mut regions = Vec::new();
        for r in m.regions {
            if r.atoms.iter().all(|&a| a < size) {
                regions.push(r);
            }
        }
        m.regions = regions;
        m.id = None;
        Some(m)
    }
}
