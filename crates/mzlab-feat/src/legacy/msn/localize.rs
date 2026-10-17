//! `hr/localize.py::localize`: where on the molecule did the change happen? Maximum likelihood on the shifts of formula between the MS2 of a
//! product and the fragment library of the parent: a fragment unshifted (U) does not contain the change, a shifted one (S) does; every
//! candidate site (a set of atoms) is scored on the atom sets the library gives to those fragments.

use crate::sortc::SortC;
use std::collections::BTreeSet;

use super::library::Library;
use super::space::candidates;
use crate::legacy::fragtree::{fmt, Graph, Space, Subs};

/// Floor of the per-fragment probability (noise, co-isolation, co-eluting isomers).
const EPS: f64 = 0.1;
/// % of the base peak: weaker fragments are not evidence.
const MIN_REL: f64 = 2.0;
/// Log-likelihood units below the best site.
const WINDOW: f64 = 0.25;

/// One fragment of the product in a known relation to the library.
#[derive(Debug, Clone)]
pub struct Evidence {
    pub mz: f64,
    pub rel: f64,
    pub formula: String,
    pub shifted: bool,
    pub library: String,
    pub ppm: f64,
}

/// What a localisation found when it has evidence.
#[derive(Debug, Clone, Default)]
pub struct Found {
    pub region_atoms: Vec<usize>,
    pub region: String,
    pub ll_best: f64,
    pub n_inside: usize,
    pub top_sites: Vec<(String, f64, Vec<usize>)>,
    pub margin: f64,
    pub fraction_explained: f64,
    /// For a loss: the atoms kept and their description.
    pub retained: Option<(Vec<usize>, String)>,
    pub n_groups: usize,
}

/// The dict of `localize` (plus `n_atoms` and `n_groups` that the engine adds).
#[derive(Debug, Clone, Default)]
pub struct Localization {
    pub kind: &'static str,
    pub delta: String,
    pub delta_sign: &'static str,
    pub evidence: Vec<Evidence>,
    pub n_sites: usize,
    pub note: &'static str,
    pub found: Option<Found>,
    pub n_atoms: usize,
}

/// Names for the atoms: every ring system (fused rings joined) `anello A`..., then every connected set of the atoms on no ring `catena A`...
pub fn default_groups(g: &Graph) -> Vec<(String, BTreeSet<usize>)> {
    let mut ring_atoms = BTreeSet::new();
    let mut systems: Vec<BTreeSet<usize>> = Vec::new();
    for ring in g.rings() {
        let mut r: BTreeSet<usize> = ring.into_iter().collect();
        ring_atoms.extend(r.iter().copied());
        let (merged, rest): (Vec<_>, Vec<_>) = systems
            .into_iter()
            .partition(|s| s.intersection(&r).count() >= 2);
        systems = rest;
        merged.into_iter().for_each(|s| r.extend(s));
        systems.push(r);
    }
    systems.sortc_by_key(|s| s.first().copied().unwrap_or(0));
    let letter = |k: usize| char::from_u32(65 + k as u32).unwrap_or('?');
    let mut out: Vec<(String, BTreeSet<usize>)> = systems
        .into_iter()
        .enumerate()
        .map(|(k, s)| (format!("anello {}", letter(k)), s))
        .collect();
    let mut seen = ring_atoms.clone();
    let mut k = 0;
    for a in 0..g.atoms.len() {
        if seen.contains(&a) {
            continue;
        }
        let (mut comp, mut st) = (BTreeSet::from([a]), vec![a]);
        while let Some(u) = st.pop() {
            for &(v, _) in &g.adj[u] {
                if !ring_atoms.contains(&v) && comp.insert(v) {
                    st.push(v);
                }
            }
        }
        seen.extend(comp.iter().copied());
        out.push((format!("catena {}", letter(k)), comp));
        k += 1;
    }
    out
}

/// `describe`: `name (k/n)` of every group with atoms in the set, largest share first.
pub fn describe(atoms: &BTreeSet<usize>, groups: &[(String, BTreeSet<usize>)]) -> String {
    let mut parts: Vec<(f64, &str, usize, usize)> = groups
        .iter()
        .filter_map(|(name, g)| {
            let k = atoms.intersection(g).count();
            (k > 0).then(|| (k as f64 / g.len() as f64, name.as_str(), k, g.len()))
        })
        .collect();
    parts.sortc_by(|a, b| b.0.total_cmp(&a.0).then(b.2.cmp(&a.2)));
    let s: Vec<String> = parts
        .iter()
        .map(|(_, name, k, n)| format!("{name} ({k}/{n})"))
        .collect();
    if s.is_empty() {
        "-".into()
    } else {
        s.join(", ")
    }
}

fn idx(els: &[&str], e: &str) -> usize {
    els.iter().position(|x| *x == e).unwrap_or(0)
}

/// `change_type`: loss (only heavy atoms lost), oxygen (O or S gained), dehydro (hydrogens lost, heavy atoms unchanged) or other.
fn change_type(delta: &[i64], els: &[&str]) -> &'static str {
    let heavy: Vec<i64> = (0..els.len())
        .filter(|&i| els[i] != "H")
        .map(|i| delta[i])
        .collect();
    if heavy.iter().all(|&d| d <= 0) && heavy.iter().any(|&d| d < 0) {
        return "loss";
    }
    if delta[idx(els, "O")] > 0 || delta[idx(els, "S")] > 0 {
        return "oxygen";
    }
    if delta[idx(els, "H")] < 0 && heavy.iter().all(|&d| d == 0) {
        return "dehydro";
    }
    "other"
}

/// `candidate_sites`: (masks, names) of the places where `delta` can have happened.
fn sites(delta: &[i64], els: &[&str], subs: &Subs, kind: &str) -> (Vec<u64>, Vec<String>) {
    let g = &subs.g;
    let name = |j: usize| format!("{}{j}", g.atoms[j]);
    match kind {
        "loss" => {
            let heavy: Vec<usize> = (0..els.len()).filter(|&i| els[i] != "H").collect();
            let sel: Vec<usize> = (0..subs.f.len())
                .filter(|&i| heavy.iter().all(|&h| subs.f[i][h] == -delta[h]))
                .collect();
            let desc = |m: u64| {
                (0..g.atoms.len())
                    .filter(|&j| (m >> j) & 1 == 1)
                    .map(name)
                    .collect::<String>()
            };
            (
                sel.iter().map(|&i| subs.masks[i]).collect(),
                sel.iter().map(|&i| desc(subs.masks[i])).collect(),
            )
        }
        "oxygen" => {
            let at: Vec<usize> = (0..g.atoms.len())
                .filter(|&j| ["C", "N", "S"].contains(&g.atoms[j].as_str()))
                .collect();
            (
                at.iter().map(|&j| 1u64 << j).collect(),
                at.iter().map(|&j| name(j)).collect(),
            )
        }
        "dehydro" => {
            let bd: Vec<(usize, usize)> = g
                .bonds
                .iter()
                .filter(|b| g.h[b.0] > 0 && g.h[b.1] > 0)
                .map(|b| (b.0, b.1))
                .collect();
            (
                bd.iter().map(|&(a, b)| (1u64 << a) | (1u64 << b)).collect(),
                bd.iter()
                    .map(|&(a, b)| format!("{}-{}", name(a), name(b)))
                    .collect(),
            )
        }
        _ => (Vec::new(), Vec::new()),
    }
}

/// The best formula of a peak: candidate, shifted, library formula, its source, rank.
type Best = (
    usize,
    Option<bool>,
    Option<String>,
    Option<String>,
    (u8, f64),
);

/// `classify_fragments`: the U / S evidence of the MS2 of the product (mz, rel in %, base peak 100).
#[allow(clippy::too_many_arguments)]
fn classify(
    tp: &[i64],
    mz: &[f64],
    rel: &[f64],
    lib: &Library,
    sp: &Space,
    delta: &[i64],
    els: &[&str],
    tp_mz: Option<f64>,
) -> Vec<Evidence> {
    let mut shifts = vec![delta.to_vec()];
    let (io, ih) = (idx(els, "O"), idx(els, "H"));
    if delta[io] > 0 {
        let mut d = delta.to_vec();
        d[io] -= 1;
        d[ih] -= 2;
        shifts.push(d);
    }
    let find = |v: &[i64]| {
        lib.entries
            .iter()
            .rev()
            .find(|e| e.1.vec == v)
            .map(|e| e.0.clone())
    };
    let mut out = Vec::new();
    for (&m, &r) in mz.iter().zip(rel) {
        if r < MIN_REL || tp_mz.is_some_and(|p| m > p - 1.5) {
            continue;
        }
        let c = candidates(sp, m, 5.0, 0.002, Some(tp));
        let mut best: Option<Best> = None;
        for &ci in c.iter().take(8) {
            let v = &sp.g[ci];
            let k0 = find(v);
            let k1 = shifts
                .iter()
                .find_map(|d| find(&v.iter().zip(d).map(|(a, b)| a - b).collect::<Vec<_>>()));
            let kind = match (&k0, &k1) {
                (Some(_), None) => Some(false),
                (None, Some(_)) => Some(true),
                _ => None,
            };
            let rank = (u8::from(kind.is_none()), (sp.mass[ci] - m).abs());
            if best
                .as_ref()
                .is_none_or(|b| rank.0 < b.4 .0 || (rank.0 == b.4 .0 && rank.1 < b.4 .1))
            {
                best = Some((ci, kind, k0, k1, rank));
            }
        }
        let Some((ci, Some(shifted), k0, k1, _)) = best else {
            continue;
        };
        out.push(Evidence {
            mz: m,
            rel: r,
            formula: fmt(&sp.g[ci], els),
            shifted,
            library: if shifted { k1 } else { k0 }.unwrap_or_default(),
            ppm: (m - sp.mass[ci]) / sp.mass[ci] * 1e6,
        });
    }
    out
}

/// `site_log_likelihood`: sum over fragments of sqrt(rel / 100) x log(eps + (1 - 2 eps) p), p = the weight of the atom sets of the library
/// fragment that hold the site (S) or miss it (U).
fn log_likelihood(r: &[u64], ev: &[Evidence], lib: &Library, subs: &Subs) -> Vec<f64> {
    let mut ll = vec![0.0; r.len()];
    for e in ev {
        let Some((idx, w)) = lib
            .get(&e.library)
            .and_then(|x| x.cand.as_ref())
            .filter(|c| !c.0.is_empty())
        else {
            continue;
        };
        let k = (e.rel / 100.0).sqrt();
        for (i, &site) in r.iter().enumerate() {
            let p = idx
                .iter()
                .zip(w)
                .filter(|(&f, _)| {
                    if e.shifted {
                        site & !subs.masks[f] == 0
                    } else {
                        site & subs.masks[f] == 0
                    }
                })
                .fold(0.0, |a, (_, &x)| a + x);
            ll[i] += k * (EPS + (1.0 - 2.0 * EPS) * p).ln();
        }
    }
    ll
}

fn atoms_of(m: u64, n: usize) -> Vec<usize> {
    (0..n).filter(|&j| (m >> j) & 1 == 1).collect()
}

/// The region, its margin and the best sites from the log-likelihood of every site.
fn found(
    ll: &[f64],
    r: &[u64],
    names: &[String],
    subs: &Subs,
    ev: &[Evidence],
    rel: &[f64],
    kind: &str,
) -> Found {
    let n = subs.g.atoms.len();
    let best = ll.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let inside: Vec<bool> = ll.iter().map(|&x| x >= best - WINDOW).collect();
    let region = (0..r.len())
        .filter(|&i| inside[i])
        .fold(0u64, |a, i| a | r[i]);
    let atoms: BTreeSet<usize> = atoms_of(region, n).into_iter().collect();
    let groups = default_groups(&subs.g);
    let mut o: Vec<usize> = (0..ll.len()).collect();
    o.sortc_by(|&a, &b| ll[b].total_cmp(&ll[a]));
    let out_best = (0..ll.len())
        .filter(|&i| !inside[i])
        .map(|i| ll[i])
        .fold(f64::NEG_INFINITY, f64::max);
    let strong = rel
        .iter()
        .filter(|&&x| x >= MIN_REL)
        .fold(0.0, |a, b| a + b);
    let retained = (kind == "loss").then(|| {
        let keep: BTreeSet<usize> = (0..n).filter(|j| !atoms.contains(j)).collect();
        (keep.iter().copied().collect(), describe(&keep, &groups))
    });
    Found {
        region_atoms: atoms.iter().copied().collect(),
        region: describe(&atoms, &groups),
        ll_best: best,
        n_inside: inside.iter().filter(|&&x| x).count(),
        top_sites: o
            .iter()
            .take(6)
            .map(|&i| (names[i].clone(), ll[i], atoms_of(r[i], n)))
            .collect(),
        margin: if inside.iter().all(|&x| x) {
            0.0
        } else {
            best - out_best
        },
        fraction_explained: ev.iter().map(|e| e.rel).fold(0.0, |a, b| a + b) / strong.max(1e-12),
        retained,
        n_groups: groups.iter().filter(|g| !g.1.is_disjoint(&atoms)).count(),
    }
}

/// `localize(tp, ref, mz, rel, lib, FormulaSpace(tp), tp_mz=...)` as the engine calls it: `tp` and `reference` = ion formulas (counts in the
/// order `els`), `mz` / `rel` the consensus MS2 of the product (rel in %).
#[allow(clippy::too_many_arguments)]
pub fn localize(
    tp: &[i64],
    reference: &[i64],
    els: &[&str],
    mz: &[f64],
    rel: &[f64],
    lib: &Library,
    subs: &Subs,
    tp_mz: Option<f64>,
) -> Localization {
    let delta: Vec<i64> = tp.iter().zip(reference).map(|(a, b)| a - b).collect();
    let abs: Vec<i64> = delta.iter().map(|d| d.abs()).collect();
    let kind = change_type(&delta, els);
    let mut out = Localization {
        kind,
        delta: fmt(&abs, els),
        delta_sign: if delta.iter().sum::<i64>() < 0 {
            "-"
        } else {
            "+"
        },
        n_atoms: subs.g.atoms.len(),
        ..Localization::default()
    };
    let (r, names) = sites(&delta, els, subs, kind);
    out.n_sites = r.len();
    if r.is_empty() {
        out.note = if kind == "other" {
            "modification type not covered by the site model"
        } else {
            "no candidate site in the graph"
        };
        return out;
    }
    let sp = Space::new(tp, els);
    out.evidence = classify(tp, mz, rel, lib, &sp, &delta, els, tp_mz);
    if out.evidence.is_empty() {
        out.note =
            "no fragment with a known relation to the library: the region cannot be determined";
        return out;
    }
    let ll = log_likelihood(&r, &out.evidence, lib, subs);
    out.found = Some(found(&ll, &r, &names, subs, &out.evidence, rel, kind));
    out
}
