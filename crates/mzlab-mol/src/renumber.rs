//! Renumbering of the atoms of a record: the same molecule in another atom order, with bonds, stereo refs, parities, cis/trans, groups
//! and the per-atom layers rewritten so that every rule of the format still holds. The writers and, later, the canonical ordering use it.

use crate::codec::MolError;
use crate::model::{Bond, Cfg, Coords, Mol, Parity, Prop, Region, Stereo, StereoGroup};

impl Mol {
    /// The record with atom `i` moved to index `new_of_old[i]` (a permutation of `0..atoms.len()`). Identity layers are kept as they are.
    pub fn renumbered(&self, new_of_old: &[u32]) -> Result<Mol, MolError> {
        let n = self.atoms.len();
        let mut old_of_new = vec![usize::MAX; n];
        for (old, &new) in new_of_old.iter().enumerate() {
            match old_of_new.get_mut(new as usize) {
                Some(slot) if *slot == usize::MAX && old < n => *slot = old,
                _ => {
                    return Err(MolError::Invalid(
                        "renumbering is not a permutation of the atoms".into(),
                    ))
                }
            }
        }
        if new_of_old.len() != n {
            return Err(MolError::Invalid(
                "renumbering needs one index per atom".into(),
            ));
        }
        let map = |x: u32| new_of_old[x as usize];
        let mut bonds: Vec<Bond> = self
            .bonds
            .iter()
            .map(|b| Bond {
                a: map(b.a).min(map(b.b)),
                b: map(b.a).max(map(b.b)),
                o: b.o,
            })
            .collect();
        bonds.sort_by_key(|b| (b.a, b.b));
        let adj = neighbours(n, &bonds);
        let mut stereo: Vec<Stereo> = self
            .stereo
            .iter()
            .map(|s| move_stereo(s, &map, &adj))
            .collect();
        stereo.sort_by_key(|s| match s {
            Stereo::Tetra { c, .. } => (0, *c, 0),
            Stereo::Db { a, b, .. } => (1, *a, *b),
        });
        let atoms = old_of_new.iter().map(|&o| self.atoms[o].clone()).collect();
        let mut m = Mol::new(atoms, bonds, stereo, self.moved_groups(&map));
        self.move_layers(&mut m, &old_of_new, &map);
        Ok(m)
    }

    fn moved_groups(&self, map: &dyn Fn(u32) -> u32) -> Vec<StereoGroup> {
        let mut groups: Vec<StereoGroup> = self
            .groups
            .iter()
            .map(|g| {
                let mut atoms: Vec<u32> = g.atoms.iter().map(|&a| map(a)).collect();
                atoms.sort_unstable();
                StereoGroup {
                    kind: g.kind,
                    atoms,
                }
            })
            .collect();
        groups.sort_by(|x, y| x.atoms.cmp(&y.atoms));
        groups
    }

    /// Carries the identity, coordinate, property and region layers over to `m`, whose atom `i` is `old_of_new[i]` of `self`.
    fn move_layers(&self, m: &mut Mol, old_of_new: &[usize], map: &dyn Fn(u32) -> u32) {
        m.id = self.id.clone();
        m.coords = self.coords.as_ref().map(|c| Coords {
            xy: old_of_new.iter().map(|&o| c.xy[o].clone()).collect(),
            ..c.clone()
        });
        m.props = self
            .props
            .iter()
            .map(|p| Prop {
                v: old_of_new.iter().map(|&o| p.v[o]).collect(),
                ..p.clone()
            })
            .collect();
        m.regions = self
            .regions
            .iter()
            .map(|r| {
                let mut atoms: Vec<u32> = r.atoms.iter().map(|&a| map(a)).collect();
                atoms.sort_unstable();
                Region { atoms, ..r.clone() }
            })
            .collect();
    }
}

fn neighbours(n: usize, bonds: &[Bond]) -> Vec<Vec<u32>> {
    let mut adj = vec![Vec::new(); n];
    for b in bonds {
        adj[b.a as usize].push(b.b);
        adj[b.b as usize].push(b.a);
    }
    adj
}

fn move_stereo(s: &Stereo, map: &dyn Fn(u32) -> u32, adj: &[Vec<u32>]) -> Stereo {
    match s {
        Stereo::Tetra {
            c,
            refs,
            implicit_h,
            par,
            status,
        } => {
            let moved: Option<Vec<u32>> =
                refs.as_ref().map(|r| r.iter().map(|&x| map(x)).collect());
            let odd = moved.as_ref().is_some_and(|r| inversions(r) % 2 == 1);
            let flip = |p: &Parity| match (p, odd) {
                (Parity::Cw, true) => Parity::Ccw,
                (Parity::Ccw, true) => Parity::Cw,
                (p, false) => *p,
            };
            Stereo::Tetra {
                c: map(*c),
                refs: moved.map(|mut r| {
                    r.sort_unstable();
                    r
                }),
                implicit_h: *implicit_h,
                par: par.as_ref().map(flip),
                status: *status,
            }
        }
        Stereo::Db {
            a,
            b,
            refs,
            cfg,
            status,
        } => {
            let (na, nb) = (map(*a), map(*b));
            let swap = na > nb;
            let (lo, hi) = if swap { (nb, na) } else { (na, nb) };
            let moved = refs.map(|[r1, r2]| {
                if swap {
                    [map(r2), map(r1)]
                } else {
                    [map(r1), map(r2)]
                }
            });
            // refs are the lowest-index neighbours of each end; a different one is on the other side of the bond
            let (la, lb) = (lowest(adj, lo, hi), lowest(adj, hi, lo));
            let flips = moved.is_some_and(|[x, y]| (x != la) != (y != lb));
            let cfg = cfg.map(|c| match (c, flips) {
                (Cfg::Cis, true) => Cfg::Trans,
                (Cfg::Trans, true) => Cfg::Cis,
                (c, false) => c,
            });
            let refs = moved.map(|_| [la, lb]);
            Stereo::Db {
                a: lo,
                b: hi,
                refs,
                cfg,
                status: *status,
            }
        }
    }
}

fn lowest(adj: &[Vec<u32>], x: u32, skip: u32) -> u32 {
    adj[x as usize]
        .iter()
        .copied()
        .filter(|&y| y != skip)
        .min()
        .unwrap_or(u32::MAX)
}

fn inversions(v: &[u32]) -> usize {
    (0..v.len())
        .flat_map(|i| (i + 1..v.len()).map(move |j| (i, j)))
        .filter(|&(i, j)| v[i] > v[j])
        .count()
}
