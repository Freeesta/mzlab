//! Alignment "per family": the unit that is matched across files is the family of ions of the same compound (the ion, its isotopologues, adducts
//! and in-source fragments), not the single ion. Steps: (1) groups of ions as in `align`; (2) the map of RT (`rtmap`) from those groups; (3) the
//! groups again with the RT corrected to the reference RT of the map; (4) the groups that hold members of the same family in some file are
//! joined (union-find, deterministic) unless the RT of the two ions differ too much, their charges differ, or the joined row would span more RT than
//! `max_span`; (5) one row per set: the reference ion (the one that is the root of its family most often over the files) gives the cells, and the
//! other ions are listed as members. A cell is `CELL_DETECTED` only where the reference ion has a peak; the other ions of the family stay visible
//! in the members and are what the gap filling does not need to guess.

use crate::align::{align_grouped, AlignParams};
use crate::rtmap::{build, RtMap, RtParams};
use crate::table::{Aligned, FileFeatures, ROLE_M, ROLE_NONE};

/// Parameters of `align_families`.
#[derive(Debug, Clone, Copy)]
pub struct FamAlignParams {
    pub ion: AlignParams,
    pub rt: RtParams,
    /// Correct the RT of every feature with the map before grouping.
    pub use_rtmap: bool,
    /// Largest RT distance (minutes) between an ion and the root of its family in the same file.
    pub link_rt: f64,
    /// Largest RT span (minutes) of the groups joined in one row.
    pub max_span: f64,
    /// An ion that is mostly the base of its own family joins another family only if it follows exactly one root and is a dependent in at least
    /// this many files.
    pub min_dependent: u32,
}

impl Default for FamAlignParams {
    fn default() -> Self {
        FamAlignParams {
            ion: AlignParams::default(),
            rt: RtParams::default(),
            use_rtmap: true,
            link_rt: 0.2,
            max_span: 0.5,
            min_dependent: 1,
        }
    }
}

/// Disjoint sets of groups; the root is the smallest index; each set keeps the RT range of its groups.
struct Sets {
    parent: Vec<u32>,
    lo: Vec<f64>,
    hi: Vec<f64>,
}

impl Sets {
    fn new(rt: &[f64]) -> Sets {
        Sets {
            parent: (0..rt.len() as u32).collect(),
            lo: rt.to_vec(),
            hi: rt.to_vec(),
        }
    }

    fn root(&mut self, mut x: u32) -> u32 {
        while self.parent[x as usize] != x {
            let up = self.parent[self.parent[x as usize] as usize];
            self.parent[x as usize] = up;
            x = up;
        }
        x
    }

    /// Joins the sets of `a` and `b` unless the joined RT range exceeds `max_span`.
    fn join(&mut self, a: u32, b: u32, max_span: f64) {
        let (ra, rb) = (self.root(a), self.root(b));
        if ra == rb {
            return;
        }
        let (lo, hi) = (
            self.lo[ra as usize].min(self.lo[rb as usize]),
            self.hi[ra as usize].max(self.hi[rb as usize]),
        );
        if hi - lo > max_span {
            return;
        }
        let (keep, drop) = (ra.min(rb) as usize, ra.max(rb) as usize);
        self.parent[drop] = keep as u32;
        self.lo[keep] = lo;
        self.hi[keep] = hi;
    }
}

/// What the files say about each group: how often it is the root of a family (or a lone ion) and the count of each role.
struct Votes {
    base: Vec<u16>,
    role: Vec<[u16; 7]>,
}

/// Counts how often each group is a root of a family or a lone ion and which role it has, and collects the links (group, group of the root of its
/// family) that pass the RT and charge checks, sorted and with repeated links merged.
fn collect(
    files: &[&FileFeatures],
    ions: &Aligned,
    group_of: &[Vec<u32>],
    p: &FamAlignParams,
) -> (Votes, Vec<(u32, u32)>) {
    let mut votes = Votes {
        base: vec![0; ions.len()],
        role: vec![[0; 7]; ions.len()],
    };
    let mut links = Vec::new();
    for (k, f) in files.iter().enumerate() {
        for i in 0..f.len() {
            let g = group_of[k][i];
            if g == u32::MAX {
                continue;
            }
            votes.role[g as usize][f.role[i].min(6) as usize] += 1;
            if f.role[i] == ROLE_M || f.role[i] == ROLE_NONE {
                votes.base[g as usize] += 1;
            }
            if f.family_id[i] == 0 {
                continue;
            }
            let root = f.family_id[i] as usize - 1;
            let gr = group_of[k][root];
            let same_charge =
                f.charge[i] == 0 || f.charge[root] == 0 || f.charge[i] == f.charge[root];
            if gr != u32::MAX
                && gr != g
                && same_charge
                && (f.rt[i] - f.rt[root]).abs() as f64 <= p.link_rt
            {
                links.push((g, gr));
            }
        }
    }
    links.sort_unstable();
    (votes, links)
}

/// Joins each group with the group of the root it follows most often. An ion that is mostly the base of its own family joins only when it follows
/// one root and was a dependent in `min_dependent` files: a fragment shared by two compounds would otherwise join their families.
fn link(
    files: &[&FileFeatures],
    ions: &Aligned,
    group_of: &[Vec<u32>],
    p: &FamAlignParams,
) -> (Sets, Votes) {
    let (votes, links) = collect(files, ions, group_of, p);
    let mut sets = Sets::new(&ions.rt);
    for chunk in links.chunk_by(|a, b| a.0 == b.0) {
        let g = chunk[0].0;
        let v = &votes.role[g as usize];
        let role = main_role(v);
        let dependent = v[2..].iter().map(|&x| x as u32).sum::<u32>();
        let base = role == ROLE_M || role == ROLE_NONE;
        if base && dependent < p.min_dependent {
            continue;
        }
        let mut best = (0usize, chunk[0].1);
        for run in chunk.chunk_by(|a, b| a.1 == b.1) {
            if run.len() > best.0 {
                best = (run.len(), run[0].1);
            }
        }
        sets.join(g, best.1, p.max_span);
    }
    (sets, votes)
}

/// Reference ion of a set of groups: the ones that are mostly a base come first; then most votes as the root of a family, the largest total area
/// and the smallest index.
fn reference(ions: &Aligned, votes: &Votes, set: &[u32]) -> u32 {
    let nf = ions.n_files;
    let area = |g: u32| {
        ions.area[g as usize * nf..(g as usize + 1) * nf]
            .iter()
            .sum::<f64>()
    };
    let key = |g: u32| {
        let role = main_role(&votes.role[g as usize]);
        (
            (role == ROLE_M || role == ROLE_NONE) as u8,
            votes.base[g as usize],
        )
    };
    let mut best = set[0];
    for &g in &set[1..] {
        if key(g) > key(best) || (key(g) == key(best) && area(g) > area(best)) {
            best = g;
        }
    }
    best
}

/// Role code most often seen for the ion (ties: the smaller code).
fn main_role(v: &[u16; 7]) -> u8 {
    (0..7).fold(
        0u8,
        |b, r| if v[r as usize] > v[b as usize] { r } else { b },
    )
}

/// One row per set of groups, in the order of the reference ions; the cells are the ones of the reference ion.
fn rows(ions: &Aligned, sets: &mut Sets, votes: &Votes) -> Aligned {
    let nf = ions.n_files;
    let mut by_root: Vec<(u32, u32)> = (0..ions.len() as u32).map(|g| (sets.root(g), g)).collect();
    by_root.sort();
    let mut runs: Vec<(u32, Vec<u32>)> = Vec::new();
    for chunk in by_root.chunk_by(|a, b| a.0 == b.0) {
        let set: Vec<u32> = chunk.iter().map(|x| x.1).collect();
        runs.push((reference(ions, votes, &set), set));
    }
    runs.sort_by_key(|r| r.0);
    let mut out = Aligned {
        n_files: nf,
        member_start: vec![0],
        ..Aligned::default()
    };
    for (g, set) in runs {
        let c = g as usize * nf..(g as usize + 1) * nf;
        out.mz.push(ions.mz[g as usize]);
        out.rt.push(ions.rt[g as usize]);
        out.area.extend_from_slice(&ions.area[c.clone()]);
        out.height.extend_from_slice(&ions.height[c.clone()]);
        out.apex_rt.extend_from_slice(&ions.apex_rt[c.clone()]);
        out.width.extend_from_slice(&ions.width[c.clone()]);
        out.state.extend_from_slice(&ions.state[c]);
        for m in std::iter::once(g).chain(set.into_iter().filter(|&m| m != g)) {
            out.member_mz.push(ions.mz[m as usize]);
            out.member_rt.push(ions.rt[m as usize] as f32);
            out.member_role.push(main_role(&votes.role[m as usize]));
        }
        out.member_start.push(out.member_mz.len() as u32);
    }
    out
}

/// Aligns the files in "per family" mode (each file must carry `family_id`, `role` and `charge`).
pub fn align_families(files: &[&FileFeatures], p: &FamAlignParams) -> Aligned {
    let first = align_grouped(files, &p.ion, None, false).table;
    let map: RtMap = if p.use_rtmap {
        build(&first, &p.rt)
    } else {
        RtMap::default()
    };
    drop(first);
    let adj = map.ok().then_some(&map);
    let g = align_grouped(files, &p.ion, adj, true);
    let (mut sets, votes) = link(files, &g.table, &g.group_of, p);
    rows(&g.table, &mut sets, &votes)
}
