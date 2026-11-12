//! The connected pieces of a `mol/1` record as records of their own (one SDF record per structure): atoms keep their relative order,
//! bonds, stereo, groups, coordinates, property layers and regions follow their atoms; the `id` is dropped (it named the whole record).

use crate::model::{Mol, Stereo, StereoGroup};

impl Mol {
    /// Each connected piece as its own record, in the order of its first atom; atoms keep their relative order and their layers.
    pub fn pieces(&self) -> Vec<Mol> {
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
        if count <= 1 {
            return vec![self.clone()];
        }
        let mut out = Vec::new();
        for k in 0..count {
            out.extend(self.only_piece(&piece, k));
        }
        out
    }

    fn only_piece(&self, piece: &[usize], k: usize) -> Option<Mol> {
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
