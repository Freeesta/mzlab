//! Families and identities of the ions of the `untargeted` table, as signs: the rows stay ions. Rewritten from scratch after the ion identity networking
//! of mzmine 3.9.0 (`modules/dataprocessing/id_ion_identity_networking/`, MIT, (c) 2004-2022 The mzmine Development Team) and of `group_metacorrelate`;
//! no code copied. Paper: Schmid et al., Nat. Commun. 12:3832 (2021), doi:10.1038/s41467-021-23953-9.
//!
//! The link is the one of the identities of mzLab (a mass relation between two ions at the same RT that an adduct pair explains): two ions at one RT whose
//! neutral masses agree under two different adducts of the polarity are one identity. The reading of each ion goes through `RowIdent::compute_in`, fed
//! with one hypothesis per linked ion (source `SRC_NETWORK`, RT of the ion); an ion with no link has no hypothesis and is read as [M+H]+ ([M-H]- in
//! negative), so `compute_in` is called with the hypotheses of the links only. Isotopologues take the identity of the representative of their isotope
//! group.
//!
//! Difference from mzmine: its networking first groups the features by RT and by the correlation of their peak shapes (metaMSEcorrelate; also the
//! intensity profile across samples) and only then looks for adducts inside a group. Here neither correlation is used: the link is by neutral mass and
//! RT alone, so two ions of compatible mass and RT are linked even if their profiles across samples disagree (a false link is possible). The 4.10.90 gold
//! data has no networking output, so there is no external value to test a correlation criterion against and no parity with mzmine is claimed for this
//! step (it is claimed for ADAP, resolver, join, gap filling and isotopes). The correlation across common samples lives in `legacy/identity.rs`.

use super::isotopes::{items_of, IsoGroups};
use crate::axis::half;
use crate::evid::HYPOTHESIS_PPM;
use crate::hypo::{Dir, Hypothesis, SRC_NETWORK};
use crate::rowident::RowIdent;
use crate::table::Aligned;
use mzlab_core::elements::{adducts, ion_mz};

/// Tolerances of the link.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IdentParams {
    /// Largest RT distance between two linked ions, minutes.
    pub rt_tol: f64,
    /// Window of m/z on the neutral masses: ppm if positive, Da if negative.
    pub win: f64,
}

impl Default for IdentParams {
    fn default() -> Self {
        IdentParams {
            rt_tol: 0.1,
            win: HYPOTHESIS_PPM,
        }
    }
}

/// The identity columns and the family id of each row (1-based in the order the families are found, 0 = alone).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Idents {
    pub ident: RowIdent,
    pub family: Vec<u32>,
}

/// Adducts that can name an ion of the polarity (positive: the first four of the registry, negative: the last three).
fn family_adducts(polarity: i8) -> Vec<(&'static str, f64)> {
    let all = adducts();
    if polarity >= 0 {
        all[..4].to_vec()
    } else {
        all[4..].to_vec()
    }
}

/// The ions linked to `anchor` read as adduct `i`: `(row, adduct index)` for each ion at the RT whose neutral mass agrees under another adduct.
fn linked(
    t: &Aligned,
    order: &[usize],
    free: &[bool],
    anchor: usize,
    i: usize,
    ads: &[(&str, f64)],
    p: &IdentParams,
) -> Vec<(usize, usize)> {
    let neutral = t.mz[anchor] - ads[i].1;
    let from = order.partition_point(|&r| t.rt[r] < t.rt[anchor] - p.rt_tol);
    let mut out = Vec::new();
    for &b in order[from..]
        .iter()
        .take_while(|&&b| t.rt[b] <= t.rt[anchor] + p.rt_tol)
    {
        if b == anchor || !free[b] {
            continue;
        }
        let tol = half(t.mz[b], p.win);
        let best = (0..ads.len())
            .filter(|&j| j != i)
            .map(|j| (j, (t.mz[b] - ads[j].1 - neutral).abs()))
            .filter(|&(_, d)| d <= tol)
            .min_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((j, _)) = best {
            out.push((b, j));
        }
    }
    out
}

fn hypothesis(
    id: u32,
    neutral: f64,
    adduct: &'static str,
    t: &Aligned,
    row: usize,
    polarity: i8,
) -> Hypothesis {
    Hypothesis {
        id,
        sources: SRC_NETWORK,
        label: adduct.to_string(),
        formula: String::new(),
        neutral,
        mz: ion_mz(neutral, adduct).unwrap_or(t.mz[row]),
        adduct,
        polarity,
        steps: 0,
        level: 0,
        dir: Dir::Free,
        rt_ref: Some(t.rt[row] as f32),
        alternatives: Vec::new(),
    }
}

/// Families of the ions and the `RowIdent` of every row. `iso` comes from `isotopes::group`: an isotopologue is never an anchor or a link, it copies the
/// identity of its representative.
pub fn ident(t: &Aligned, iso: &IsoGroups, polarity: i8, p: &IdentParams) -> Idents {
    let n = t.len();
    let ads = family_adducts(polarity);
    let mut follows = vec![usize::MAX; n];
    for g in &iso.groups {
        for &m in g.members.iter().filter(|&&m| m != g.representative) {
            follows[m] = g.representative;
        }
    }
    let mut free: Vec<bool> = follows.iter().map(|&f| f == usize::MAX).collect();
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| t.rt[a].total_cmp(&t.rt[b]));
    let mut by_height: Vec<usize> = (0..n).collect();
    let height = items_of(t);
    by_height.sort_by(|&a, &b| height[b].height.total_cmp(&height[a].height));
    let (mut family, mut hyps) = (vec![0u32; n], Vec::new());
    let mut next = 0u32;
    for a in by_height {
        if !free[a] {
            continue;
        }
        let best = (0..ads.len())
            .map(|i| (i, linked(t, &order, &free, a, i, &ads, p)))
            .filter(|(_, l)| !l.is_empty())
            .fold(
                None,
                |best: Option<(usize, Vec<(usize, usize)>)>, c| match best {
                    Some(b) if b.1.len() >= c.1.len() => Some(b),
                    _ => Some(c),
                },
            );
        free[a] = false;
        let Some((i, members)) = best else { continue };
        next += 1;
        let neutral = t.mz[a] - ads[i].1;
        family[a] = next;
        hyps.push(hypothesis(
            hyps.len() as u32 + 1,
            neutral,
            ads[i].0,
            t,
            a,
            polarity,
        ));
        for (b, j) in members {
            free[b] = false;
            family[b] = next;
            hyps.push(hypothesis(
                hyps.len() as u32 + 1,
                neutral,
                ads[j].0,
                t,
                b,
                polarity,
            ));
        }
    }
    let mut ident = RowIdent::compute_in(t, &hyps, polarity, p.win);
    for r in 0..n {
        if follows[r] != usize::MAX {
            let k = follows[r];
            family[r] = family[k];
            (ident.neutral[r], ident.ion[r], ident.src[r]) =
                (ident.neutral[k], ident.ion[k], ident.src[k]);
        }
    }
    Idents { ident, family }
}
