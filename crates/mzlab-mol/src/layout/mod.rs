//! Own 2D layout of a `mol/1` record (no code from other toolkits): ring systems as regular polygons fused along shared bonds (bridges on
//! arcs, spiro rings hung on their atom, macrocycles as one large polygon), chains as a 120-degree zigzag, then a short overlap removal and
//! the written cis/trans of defined double bonds. Units: angstrom, bond length 1.5; deterministic, no libm calls, so the bytes match in the
//! `.wasm`. Fragments (salts, mixtures) are laid out one by one, left to right, largest first.

mod clean;
mod grow;
mod place;
mod rings;
mod trig;

use std::collections::VecDeque;

use crate::model::{Cfg, Coords, Mol, Status, Stereo};
use trig::{add, V};

/// Bond length of the layout in angstrom.
pub const BOND_A: f64 = 1.5;
/// Gap between fragments, in bond lengths (the quality gate asks for 1.5).
const FRAGMENT_GAP: f64 = 2.0;

/// A double bond with a written configuration: `r0` hangs on `a`, `r1` on `b`; `trans` is what the record says.
#[derive(Clone)]
pub(crate) struct Db {
    pub a: usize,
    pub b: usize,
    pub r0: usize,
    pub r1: usize,
    pub trans: bool,
}

/// The graph and the positions of one layout run (positions in bond lengths).
pub(crate) struct G {
    pub n: usize,
    pub adj: Vec<Vec<usize>>,
    pub bonds: Vec<(usize, usize)>,
    pub linear: Vec<bool>,
    pub rings: Vec<Vec<usize>>,
    pub systems: Vec<Vec<usize>>,
    pub ring_sys: Vec<usize>,
    pub rings_of: Vec<Vec<usize>>,
    pub dbs: Vec<Db>,
    pub pos: Vec<Option<V>>,
    pub frozen: Vec<bool>,
}

impl G {
    fn new(m: &Mol) -> G {
        let n = m.atoms.len();
        let mut adj = vec![Vec::new(); n];
        let mut bonds = Vec::new();
        let (mut triple, mut double) = (vec![0u8; n], vec![0u8; n]);
        for b in &m.bonds {
            let (x, y) = (b.a as usize, b.b as usize);
            adj[x].push(y);
            adj[y].push(x);
            bonds.push((x.min(y), x.max(y)));
            for z in [x, y] {
                match b.o {
                    3 => triple[z] += 1,
                    2 => double[z] += 1,
                    _ => {}
                }
            }
        }
        adj.iter_mut().for_each(|v| v.sort_unstable());
        bonds.sort_unstable();
        let linear = (0..n)
            .map(|i| adj[i].len() == 2 && (triple[i] > 0 || double[i] == 2))
            .collect();
        let rings = rings::sssr(&adj);
        let systems = rings::systems(&rings);
        let mut ring_sys = vec![0; rings.len()];
        for (s, rs) in systems.iter().enumerate() {
            rs.iter().for_each(|&r| ring_sys[r] = s);
        }
        let mut rings_of = vec![Vec::new(); n];
        for (r, atoms) in rings.iter().enumerate() {
            atoms.iter().for_each(|&a| rings_of[a].push(r));
        }
        let dbs = m
            .stereo
            .iter()
            .filter_map(|s| match s {
                Stereo::Db { a, b, refs: Some(r), cfg: Some(c), status: Status::Defined } if r.len() == 2 => Some(Db {
                    a: *a as usize,
                    b: *b as usize,
                    r0: r[0] as usize,
                    r1: r[1] as usize,
                    trans: *c == Cfg::Trans,
                }),
                _ => None,
            })
            .collect();
        G { n, adj, bonds, linear, rings, systems, ring_sys, rings_of, dbs, pos: vec![None; n], frozen: vec![false; n] }
    }

    /// For a ring of 8 or more atoms: the cycle index `i` of a defined double bond `r[i]=r[i+1]` that the ring must draw trans.
    pub(crate) fn kink_at(&self, ri: usize) -> Option<usize> {
        let r = &self.rings[ri];
        let n = r.len();
        if n < 8 {
            return None;
        }
        (0..n).find(|&i| {
            let (x, y) = (r[i], r[(i + 1) % n]);
            self.dbs.iter().any(|d| {
                let same = (d.a == x && d.b == y) || (d.a == y && d.b == x);
                // a ref outside the ring flips what "trans" means for the ring path
                let off = |p: usize| !r.contains(&p);
                same && (d.trans ^ off(d.r0) ^ off(d.r1))
            })
        })
    }

    /// The atoms of the connected pieces, each in ascending order, largest piece first (ties: the one with the lowest atom).
    fn fragments(&self) -> Vec<Vec<usize>> {
        let mut seen = vec![false; self.n];
        let mut out: Vec<Vec<usize>> = Vec::new();
        for s in 0..self.n {
            if seen[s] {
                continue;
            }
            seen[s] = true;
            let mut st = vec![s];
            let mut f = Vec::new();
            while let Some(x) = st.pop() {
                f.push(x);
                for &y in &self.adj[x] {
                    if !seen[y] {
                        seen[y] = true;
                        st.push(y);
                    }
                }
            }
            f.sort_unstable();
            out.push(f);
        }
        out.sort_by(|a, b| b.len().cmp(&a.len()).then(a[0].cmp(&b[0])));
        out
    }

    /// The atom farthest from the lowest atom of the piece, then the one farthest from that (the end of the longest chain).
    fn chain_end(&self, frag: &[usize]) -> usize {
        let far = |from: usize| {
            let mut d = vec![usize::MAX; self.n];
            d[from] = 0;
            let mut q = VecDeque::from([from]);
            let mut best = from;
            while let Some(x) = q.pop_front() {
                if d[x] > d[best] || (d[x] == d[best] && x < best) {
                    best = x;
                }
                for &y in &self.adj[x] {
                    if d[y] == usize::MAX {
                        d[y] = d[x] + 1;
                        q.push_back(y);
                    }
                }
            }
            best
        };
        far(far(frag[0]))
    }

    /// Lays out one connected piece: from its placed atoms if any, else from the biggest ring system or the end of its longest chain.
    fn run_piece(&mut self, frag: &[usize]) {
        let mut queue: VecDeque<usize> = frag.iter().copied().filter(|&a| self.pos[a].is_some()).collect();
        if queue.is_empty() {
            let root = frag
                .iter()
                .flat_map(|&a| self.rings_of[a].iter().map(|&r| self.ring_sys[r]))
                .max_by_key(|&s| {
                    let size: usize = self.systems[s].iter().map(|&r| self.rings[r].len()).sum();
                    (self.systems[s].len(), size, usize::MAX - s)
                });
            match root {
                Some(s) => {
                    place::place_system(self, s);
                    let mut atoms: Vec<usize> = self.systems[s].iter().flat_map(|&r| self.rings[r].iter().copied()).collect();
                    atoms.sort_unstable();
                    atoms.dedup();
                    queue.extend(atoms);
                }
                None => {
                    let r = self.chain_end(frag);
                    self.pos[r] = Some([0.0, 0.0]);
                    queue.push_back(r);
                }
            }
        }
        grow::expand(self, &mut queue);
        clean::fix_ez(self);
        clean::tidy(self);
    }
}

fn bbox(pts: impl Iterator<Item = V>) -> Option<[f64; 4]> {
    pts.fold(None, |b: Option<[f64; 4]>, p| {
        Some(match b {
            None => [p[0], p[1], p[0], p[1]],
            Some(c) => [c[0].min(p[0]), c[1].min(p[1]), c[2].max(p[0]), c[3].max(p[1])],
        })
    })
}

/// Rounds to four decimals, the precision of a molfile.
fn r4(v: f64) -> f64 {
    (v * 1e4).round() / 1e4
}

impl Mol {
    /// A 2D layout of the whole record, bond length 1.5 angstrom: pieces left to right, largest first, 2 bond lengths apart.
    pub fn layout_2d(&self) -> Coords {
        self.layout_2d_fixed(&vec![None; self.atoms.len()])
    }

    /// A 2D layout that keeps the atoms given in `fixed` (angstrom) where they are and places the others around them; pieces
    /// without a fixed atom go to the right of the rest. `fixed` shorter than the record counts as no fixed atom.
    pub fn layout_2d_fixed(&self, fixed: &[Option<[f64; 2]>]) -> Coords {
        let mut g = G::new(self);
        for (i, f) in fixed.iter().enumerate().take(g.n) {
            if let Some(p) = f {
                g.pos[i] = Some([p[0] / BOND_A, p[1] / BOND_A]);
                g.frozen[i] = true;
            }
        }
        let frags = g.fragments();
        let has_fixed: Vec<bool> = frags.iter().map(|f| f.iter().any(|&a| g.frozen[a])).collect();
        let mut out: Vec<Option<V>> = vec![None; g.n];
        let mut right = f64::NEG_INFINITY;
        for f in frags.iter().zip(&has_fixed).filter(|(_, h)| **h).map(|(f, _)| f) {
            g.run_piece(f);
            for &a in f {
                out[a] = g.pos[a];
            }
            if let Some(b) = bbox(f.iter().filter_map(|&a| g.pos[a])) {
                right = right.max(b[2]);
            }
        }
        let mut cursor = if right.is_finite() { right + FRAGMENT_GAP } else { 0.0 };
        for f in frags.iter().zip(&has_fixed).filter(|(_, h)| !**h).map(|(f, _)| f) {
            g.run_piece(f);
            let Some(b) = bbox(f.iter().filter_map(|&a| g.pos[a])) else { continue };
            let shift = [cursor - b[0], -(b[1] + b[3]) / 2.0];
            for &a in f {
                out[a] = g.pos[a].map(|p| add(p, shift));
                g.pos[a] = None;
            }
            cursor += b[2] - b[0] + FRAGMENT_GAP;
        }
        let xy = out
            .iter()
            .map(|p| {
                let p = p.unwrap_or([0.0, 0.0]);
                vec![r4(p[0] * BOND_A), r4(p[1] * BOND_A)]
            })
            .collect();
        Coords { dim: 2, xy, from: "layout".into() }
    }

    /// The pieces of the record in the order `layout_2d` places them (atom indices, ascending in each).
    pub fn layout_pieces(&self) -> Vec<Vec<usize>> {
        G::new(self).fragments()
    }

    /// The defined double bonds (as atom pairs `a`, `b`) that the coordinates of the record draw with the wrong cis/trans.
    pub fn ez_mismatches(&self) -> Vec<(u32, u32)> {
        let Some(c) = &self.coords else { return Vec::new() };
        let g = G::new(self);
        let pos: Vec<Option<V>> = c.xy.iter().map(|r| Some([r[0], r[1]])).collect();
        clean::ez_wrong(&g, &pos)
            .into_iter()
            .map(|k| (g.dbs[k].a as u32, g.dbs[k].b as u32))
            .collect()
    }
}
