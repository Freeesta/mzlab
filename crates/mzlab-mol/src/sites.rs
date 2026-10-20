//! Which atoms and double bonds of a record can carry stereo: `stereo_sites` lists them and `with_stereo_sites` turns a record into
//! "defined where written, `unknown` where it could be, nothing elsewhere". A site needs a chemically fit atom or bond and substituents
//! that differ; substituents that only differ through stereo elsewhere (ring cis/trans, pseudo-asymmetric centres) are settled by trying
//! to flip the element in records whose other sites are given values and comparing canonical forms.

use crate::model::{Cfg, Mol, Parity, Status, Stereo, StereoGroup};

/// A place in a record where stereo can exist.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Site {
    /// Tetrahedral centre.
    Tetra(u32),
    /// Double bond `a`-`b` with `a < b`.
    Db(u32, u32),
}

/// Number of random assignments of the other sites tried before a site is declared not stereogenic.
const TRIES: usize = 64;
/// Smallest ring that holds a stereogenic double bond.
const MIN_RING: usize = 8;
const H: usize = 0;
const LONE_PAIR: usize = 1;

struct Cand {
    site: Site,
    /// (centre, x, y): two neighbours of the same constitution class on a centre of the site.
    twins: Vec<(usize, usize, usize)>,
    /// All substituents differ already by constitution: no flip test needed.
    sure: bool,
}

impl Mol {
    /// The sites of the molecule, tetrahedral centres first, each group in ascending order. Whatever stereo the record carries is ignored.
    pub fn stereo_sites(&self) -> Vec<Site> {
        let base = self.stripped();
        let cands = base.candidates();
        let mut out = Vec::new();
        for (i, c) in cands.iter().enumerate() {
            if c.sure || base.flips(&cands, i) {
                out.push(c.site);
            }
        }
        out.sort();
        out
    }

    /// The record with stereo only on its sites: a written element is kept, a site without one becomes `unknown`, any other element is dropped
    /// (as are the group members that lose theirs).
    pub fn with_stereo_sites(&self) -> Mol {
        let sites = self.stereo_sites();
        let at = |s: &Stereo| match s {
            Stereo::Tetra { c, .. } => Site::Tetra(*c),
            Stereo::Db { a, b, .. } => Site::Db(*a, *b),
        };
        let stereo: Vec<Stereo> = sites
            .iter()
            .map(|&site| {
                self.stereo
                    .iter()
                    .find(|s| at(s) == site)
                    .cloned()
                    .unwrap_or_else(|| unknown(site))
            })
            .collect();
        let defined = |a: u32| {
            stereo
                .iter()
                .any(|s| matches!(s, Stereo::Tetra { c, status: Status::Defined, .. } if *c == a))
        };
        let groups: Vec<StereoGroup> = self
            .groups
            .iter()
            .map(|g| StereoGroup {
                kind: g.kind,
                atoms: g.atoms.iter().copied().filter(|&a| defined(a)).collect(),
            })
            .filter(|g| !g.atoms.is_empty())
            .collect();
        let mut m = self.clone();
        m.stereo = stereo;
        m.groups = groups;
        m
    }

    fn stripped(&self) -> Mol {
        Mol::new(self.atoms.clone(), self.bonds.clone(), Vec::new(), Vec::new())
    }

    fn neighbours(&self) -> Vec<Vec<(u32, u8)>> {
        let mut adj = vec![Vec::new(); self.atoms.len()];
        for b in &self.bonds {
            adj[b.a as usize].push((b.b, b.o));
            adj[b.b as usize].push((b.a, b.o));
        }
        for list in &mut adj {
            list.sort_unstable();
        }
        adj
    }

    fn candidates(&self) -> Vec<Cand> {
        let adj = self.neighbours();
        let cls = self.constitution_classes();
        let mut out = Vec::new();
        for c in 0..self.atoms.len() {
            if let Some(slots) = tetra_slots(self, &adj, &cls, c) {
                if let Some(sure) = differ(&slots, 3) {
                    out.push(Cand {
                        site: Site::Tetra(c as u32),
                        twins: twins(&adj, &cls, c, None),
                        sure,
                    });
                }
            }
        }
        for b in self.bonds.iter().filter(|b| b.o == 2) {
            if let Some(sure) = db_sure(self, &adj, &cls, b.a, b.b) {
                let mut t = twins(&adj, &cls, b.a as usize, Some(b.b));
                t.extend(twins(&adj, &cls, b.b as usize, Some(b.a)));
                out.push(Cand {
                    site: Site::Db(b.a, b.b),
                    twins: t,
                    sure,
                });
            }
        }
        // twins that no stereo can tell apart make the swap of the two an automorphism, so the site cannot exist
        let mut live = vec![false; self.atoms.len()];
        for c in &out {
            match c.site {
                Site::Tetra(a) => live[a as usize] = true,
                Site::Db(a, b) => (live[a as usize], live[b as usize]) = (true, true),
            }
        }
        out.retain(|c| !c.twins.iter().any(|&(z, x, y)| dead_pair(&adj, &live, z, x, y)));
        out
    }

    /// Whether flipping candidate `i` changes the molecule for some values of the other candidates.
    fn flips(&self, cands: &[Cand], i: usize) -> bool {
        let others: Vec<usize> = (0..cands.len()).filter(|&j| j != i).collect();
        let mut state = 0x9E37_79B9_7F4A_7C15u64;
        let total = if others.len() <= 5 { 1 << others.len() } else { TRIES };
        for t in 0..total {
            let bits: Vec<bool> = (0..others.len())
                .map(|j| match (others.len() <= 5, t) {
                    (true, t) => (t >> j) & 1 == 1,
                    (false, 0) => true,
                    _ => {
                        state = state
                            .wrapping_mul(6_364_136_223_846_793_005)
                            .wrapping_add(1_442_695_040_888_963_407);
                        (state >> 33) & 1 == 1
                    }
                })
                .collect();
            let given = |flip: bool| {
                let mut v: Vec<(Site, bool)> = others
                    .iter()
                    .zip(&bits)
                    .map(|(&j, &b)| (cands[j].site, b))
                    .collect();
                v.push((cands[i].site, flip));
                self.with_values(&v).canonical().core_json()
            };
            if given(true) != given(false) {
                return true;
            }
        }
        false
    }

    /// The record with every given site defined; `true` = clockwise / cis.
    fn with_values(&self, values: &[(Site, bool)]) -> Mol {
        let adj = self.neighbours();
        let stereo = values
            .iter()
            .map(|&(site, v)| match site {
                Site::Tetra(c) => {
                    let refs: Vec<u32> = adj[c as usize].iter().map(|&(x, _)| x).collect();
                    Stereo::Tetra {
                        c,
                        implicit_h: Some(refs.len() == 3 && self.atoms[c as usize].h == 1),
                        refs: Some(refs),
                        par: Some(if v { Parity::Cw } else { Parity::Ccw }),
                        status: Status::Defined,
                    }
                }
                Site::Db(a, b) => Stereo::Db {
                    a,
                    b,
                    refs: Some([lowest(&adj, a, b), lowest(&adj, b, a)]),
                    cfg: Some(if v { Cfg::Cis } else { Cfg::Trans }),
                    status: Status::Defined,
                },
            })
            .collect();
        Mol::new(self.atoms.clone(), self.bonds.clone(), stereo, Vec::new())
    }
}

fn twins(adj: &[Vec<(u32, u8)>], cls: &[usize], c: usize, skip: Option<u32>) -> Vec<(usize, usize, usize)> {
    let near: Vec<usize> = adj[c].iter().map(|&(x, _)| x as usize).filter(|&x| Some(x as u32) != skip).collect();
    let mut out = Vec::new();
    for (i, &x) in near.iter().enumerate() {
        for &y in &near[i + 1..] {
            if cls[x] == cls[y] {
                out.push((c, x, y));
            }
        }
    }
    out
}

/// Whether the branches `x` and `y` of `c` are separate (not in one ring) and hold no atom of a possible site.
fn dead_pair(adj: &[Vec<(u32, u8)>], live: &[bool], c: usize, x: usize, y: usize) -> bool {
    let branch = |s: usize| {
        let mut seen = vec![false; adj.len()];
        seen[c] = true;
        seen[s] = true;
        let mut stack = vec![s];
        let mut all = vec![s];
        while let Some(u) = stack.pop() {
            for &(v, _) in &adj[u] {
                if !seen[v as usize] {
                    seen[v as usize] = true;
                    stack.push(v as usize);
                    all.push(v as usize);
                }
            }
        }
        all
    };
    let (bx, by) = (branch(x), branch(y));
    !bx.contains(&y) && !bx.iter().chain(&by).any(|&a| live[a])
}

fn unknown(site: Site) -> Stereo {
    match site {
        Site::Tetra(c) => Stereo::Tetra {
            c,
            refs: None,
            implicit_h: None,
            par: None,
            status: Status::Unknown,
        },
        Site::Db(a, b) => Stereo::Db {
            a,
            b,
            refs: None,
            cfg: None,
            status: Status::Unknown,
        },
    }
}

fn lowest(adj: &[Vec<(u32, u8)>], x: u32, skip: u32) -> u32 {
    adj[x as usize]
        .iter()
        .map(|&(y, _)| y)
        .filter(|&y| y != skip)
        .min()
        .unwrap_or(u32::MAX)
}

/// `None` = fewer than `need` different slots (cannot be a site); `Some(true)` = all slots differ.
fn differ(slots: &[usize], need: usize) -> Option<bool> {
    let mut s = slots.to_vec();
    s.sort_unstable();
    s.dedup();
    (s.len() >= need).then_some(s.len() == slots.len())
}

/// The substituent slots of a tetrahedral centre (class of each neighbour; hydrogen and lone pair have markers of their own), or `None`
/// when the atom cannot be a centre.
fn tetra_slots(m: &Mol, adj: &[Vec<(u32, u8)>], cls: &[usize], c: usize) -> Option<Vec<usize>> {
    let a = &m.atoms[c];
    let near = &adj[c];
    let doubles = near.iter().filter(|&&(_, o)| o == 2).count();
    if a.rad != 0 || near.iter().any(|&(_, o)| o == 3) {
        return None;
    }
    let total = near.len() + a.h as usize;
    let fit = match (a.el.as_str(), a.chg) {
        ("C" | "Si" | "Ge" | "Sn", 0) => doubles == 0 && total == 4,
        ("N" | "P" | "As", 1) | ("B", -1) => doubles == 0 && total == 4,
        ("N", 0) => doubles == 0 && near.len() == 3 && a.h == 0 && strained_n(adj, c),
        ("P" | "As", 0) => (doubles == 0 && total == 3 && a.h == 0) || (doubles <= 1 && total == 4),
        ("S" | "Se", 0) => a.h == 0 && ((near.len() == 3 && doubles == 1) || (near.len() == 4 && doubles == 2)),
        ("S" | "Se", 1) => doubles == 0 && near.len() == 3 && a.h == 0,
        _ => false,
    };
    if !fit {
        return None;
    }
    let mut slots: Vec<usize> = near.iter().map(|&(x, _)| cls[x as usize] + 2).collect();
    slots.extend(std::iter::repeat_n(H, a.h as usize));
    if total == 3 {
        slots.push(LONE_PAIR);
    }
    Some(slots)
}

/// A neutral three-coordinate nitrogen inverts unless it is in a three-membered ring or at the bridgehead of a bridged bicycle (not fused).
fn strained_n(adj: &[Vec<(u32, u8)>], n: usize) -> bool {
    let nb: Vec<usize> = adj[n].iter().map(|&(x, _)| x as usize).collect();
    let mut sizes = Vec::new();
    for (i, &x) in nb.iter().enumerate() {
        for &y in &nb[i + 1..] {
            match path(adj, x, y, &|u, v| u == n || v == n) {
                Some(1) => return true,
                Some(d) => sizes.push(d + 2),
                None => return false,
            }
        }
    }
    sizes.sort_unstable();
    sizes.len() == 3 && sizes[2] != sizes[0] + sizes[1] - 2
}

/// Length of the shortest path `s` to `t` that uses no edge `(u, v)` for which `blocked(u, v)`.
fn path(adj: &[Vec<(u32, u8)>], s: usize, t: usize, blocked: &dyn Fn(usize, usize) -> bool) -> Option<usize> {
    let mut dist = vec![usize::MAX; adj.len()];
    dist[s] = 0;
    let mut queue = std::collections::VecDeque::from([s]);
    while let Some(u) = queue.pop_front() {
        if u == t {
            return Some(dist[u]);
        }
        for &(v, _) in &adj[u] {
            let v = v as usize;
            if dist[v] == usize::MAX && !blocked(u, v) {
                dist[v] = dist[u] + 1;
                queue.push_back(v);
            }
        }
    }
    None
}

/// Slots of one end `x` of the double bond to `y`: its other substituents, plus H or a lone pair when only one is written.
fn end_slots(m: &Mol, adj: &[Vec<(u32, u8)>], cls: &[usize], x: u32, y: u32) -> Option<Vec<usize>> {
    let a = &m.atoms[x as usize];
    let others: Vec<(u32, u8)> = adj[x as usize].iter().copied().filter(|&(z, _)| z != y).collect();
    if !matches!(a.el.as_str(), "C" | "N") || a.rad != 0 || others.is_empty() || others.iter().any(|&(_, o)| o != 1) {
        return None;
    }
    let mut slots: Vec<usize> = others.iter().map(|&(z, _)| cls[z as usize] + 2).collect();
    match (others.len(), a.h, a.el.as_str(), a.chg) {
        (2, 0, _, _) => {}
        (1, h, _, _) if h > 0 => slots.push(H),
        (1, 0, "N", 0) => slots.push(LONE_PAIR),
        _ => return None,
    }
    Some(slots)
}

/// `Some(sure)` when the double bond `a`-`b` can carry cis/trans (not in a small ring, both ends fit).
fn db_sure(m: &Mol, adj: &[Vec<(u32, u8)>], cls: &[usize], a: u32, b: u32) -> Option<bool> {
    let ring = path(adj, a as usize, b as usize, &|u, v| {
        (u == a as usize && v == b as usize) || (u == b as usize && v == a as usize)
    });
    if ring.is_some_and(|d| d + 1 < MIN_RING) {
        return None;
    }
    let (sa, sb) = (end_slots(m, adj, cls, a, b)?, end_slots(m, adj, cls, b, a)?);
    Some(sa[0] != sa[1] && sb[0] != sb[1])
}
