//! From the arcs to the roles: who derives from whom, the family of every row and the readings that stay open.
//!
//! An arc says which end depends on the other. An adduct or a dimer depends on the lighter ion; an in-source fragment (with its proof) on the
//! heavier one, which must be at least `parent_factor` times higher (two co-eluting products that differ by a neutral loss are not a
//! fragment and its precursor); a step that two readings explain and no proof separates depends on the weaker end, which stays
//! «undetermined». A row takes the best arc where it is the dependent end (definitive before undetermined, then more points, then the
//! stronger parent, then the lower row); an arc that would close a cycle is skipped. Isotopologues follow their group leader.

use super::evidence::{EdgeKind, ProofKind};
use super::rel::{RKind, Tables};
use super::Arc;
use crate::table::*;

pub(super) const NO_PARENT: u32 = u32::MAX;

struct Dep {
    dep: u32,
    par: u32,
    role: u8,
    rank: u8,
    points: f32,
}

fn has(a: &Arc, k: RKind) -> bool {
    a.readings.iter().any(|r| r.kind == k)
}

fn points(a: &Arc) -> f32 {
    a.proofs.iter().map(|p| p.points).sum()
}

/// `(dependent, parent, role, rank)` of an arc (rank 0 = definitive, 1 = undetermined); None when the step is no relation or the dependent is
/// not at least `factor` times weaker than its parent (a stronger «adduct» or «fragment» could be a product: it stays out of the family).
fn orient(f: &FileFeatures, a: &Arc, factor: f32) -> Option<(u32, u32, u8, u8)> {
    let (lo, up) = (a.lo, a.up);
    let (hl, hu) = (f.height[lo as usize], f.height[up as usize]);
    let (adduct, dimer, loss) = (
        has(a, RKind::Adduct),
        has(a, RKind::Dimer),
        has(a, RKind::Loss),
    );
    if a.kind == EdgeKind::Other {
        return None;
    }
    let (lo_dep, up_dep) = (hu >= factor * hl, hl >= factor * hu);
    // which end can depend on the other, by the readings: a fragment is the lighter, an adduct or dimer the heavier
    let (frag_ok, add_ok) = (loss && lo_dep, (adduct || dimer) && up_dep);
    match a.kind {
        EdgeKind::Adduct if add_ok => {
            Some((up, lo, if adduct { ROLE_ADDUCT } else { ROLE_DIMER }, 0))
        }
        EdgeKind::Isf if frag_ok => Some((lo, up, ROLE_ISF, 0)),
        EdgeKind::Adduct | EdgeKind::Isf | EdgeKind::Undetermined if frag_ok => {
            Some((lo, up, ROLE_UNDETERMINED, 1))
        }
        EdgeKind::Adduct | EdgeKind::Isf | EdgeKind::Undetermined if add_ok => {
            Some((up, lo, ROLE_UNDETERMINED, 1))
        }
        _ => None,
    }
}

fn reaches(parent: &[u32], from: u32, target: u32) -> bool {
    let mut x = from;
    for _ in 0..parent.len() + 1 {
        if x == target {
            return true;
        }
        if parent[x as usize] == NO_PARENT {
            return false;
        }
        x = parent[x as usize];
    }
    true
}

pub(super) struct Assigned {
    pub parent: Vec<u32>,
    pub role: Vec<u8>,
    pub alt: Vec<u8>,
    pub family: Vec<u32>,
}

fn excluded(f: &FileFeatures, i: usize) -> bool {
    f.flags[i] & (FLAG_BACKGROUND | FLAG_FT_LOBE) != 0
}

pub(super) fn assign(f: &FileFeatures, arcs: &[Arc], factor: f32) -> Assigned {
    let n = f.len();
    let mut deps: Vec<Dep> = arcs
        .iter()
        .filter_map(|a| {
            orient(f, a, factor).map(|(dep, par, role, rank)| Dep {
                dep,
                par,
                role,
                rank,
                points: points(a),
            })
        })
        .collect();
    deps.sort_by(|a, b| {
        a.dep
            .cmp(&b.dep)
            .then(a.rank.cmp(&b.rank))
            .then(b.points.total_cmp(&a.points))
            .then(f.height[b.par as usize].total_cmp(&f.height[a.par as usize]))
            .then(a.par.cmp(&b.par))
    });
    let mut parent = vec![NO_PARENT; n];
    let mut role = vec![ROLE_NONE; n];
    let mut alt = vec![0u8; n];
    for i in (0..n).filter(|&i| !excluded(f, i)) {
        role[i] = ROLE_M;
    }
    for i in 0..n {
        if f.flags[i] & FLAG_ISOTOPE != 0
            && f.iso_group[i] > 0
            && !excluded(f, f.iso_group[i] as usize - 1)
        {
            parent[i] = f.iso_group[i] - 1;
            role[i] = ROLE_ISOTOPE;
        }
    }
    let mut k = 0;
    while k < deps.len() {
        let d = deps[k].dep;
        let end = k + deps[k..].iter().take_while(|x| x.dep == d).count();
        let taken = parent[d as usize] != NO_PARENT;
        let mut chosen: Option<usize> = None;
        for (m, c) in deps[k..end].iter().enumerate() {
            if !taken
                && chosen.is_none()
                && !reaches(&parent, c.par, d)
                && !excluded(f, c.par as usize)
                && role[d as usize] != ROLE_NONE
            {
                chosen = Some(k + m);
            } else if chosen.is_some_and(|ch| deps[ch].rank == c.rank && deps[ch].par != c.par) {
                alt[d as usize] |= ALT_PARENT;
            }
        }
        if let Some(c) = chosen {
            parent[d as usize] = deps[c].par;
            role[d as usize] = deps[c].role;
        }
        k = end;
    }
    open_readings(arcs, &mut alt);
    let family = (0..n)
        .map(|i| {
            if role[i] == ROLE_NONE {
                return 0;
            }
            let mut x = i;
            while parent[x] != NO_PARENT {
                x = parent[x] as usize;
            }
            x as u32 + 1
        })
        .collect();
    Assigned {
        parent,
        role,
        alt,
        family,
    }
}

/// `ALT_*` of both ends of every undetermined arc that two readings explain.
fn open_readings(arcs: &[Arc], alt: &mut [u8]) {
    for a in arcs.iter().filter(|a| a.kind == EdgeKind::Undetermined) {
        let both = (has(a, RKind::Adduct) || has(a, RKind::Dimer)) && has(a, RKind::Loss);
        if !both {
            continue;
        }
        let nh3 = a.readings.iter().any(|r| Tables::is_nh3(&r.name));
        let fa = a.readings.iter().any(|r| Tables::is_formate(&r.name));
        let bit = if nh3 {
            ALT_NH3
        } else if fa {
            ALT_FORMATE
        } else {
            ALT_ROLE
        };
        alt[a.lo as usize] |= bit;
        alt[a.up as usize] |= bit;
    }
}

/// Points of the proofs of an arc that support one reading: the skeleton and the typical loss for the adduct, the fragment in the MS2 and
/// the ratio for the in-source fragment.
pub(super) fn reading_points(a: &Arc, fragment: bool) -> f32 {
    let mine = |k: ProofKind| {
        if fragment {
            matches!(k, ProofKind::FragmentInMs2 | ProofKind::Ratio)
        } else {
            matches!(k, ProofKind::Skeleton | ProofKind::TypicalLoss)
        }
    };
    a.proofs
        .iter()
        .filter(|p| mine(p.kind))
        .map(|p| p.points)
        .sum()
}
