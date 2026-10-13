//! The one builder of the tree of an MSn file (`TP_Mine/py/tpmine/hr/msn.py`: `node_key`, `node_id`, the grouping of `build_tree` and
//! `find_parent`), without chemistry: scans → nodes (one per path of precursors) with their parent, and the consensus spectrum of a node
//! (`consensus_arrays`). mzFinder (`mzlab_feat::legacy::msn`) puts formulas, calibration and atoms on it.

use std::cmp::Ordering;
use std::collections::HashMap;

use crate::idxread::Scan;

/// How the scans become nodes; the default is what `msn.py` does.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum Tol {
    /// Precursors rounded to the nominal mass (`node_key`).
    #[default]
    Nominale,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Raggruppa {
    pub tol: Tol,
}

/// One node (not the root MS1).
#[derive(Debug, Clone, PartialEq)]
pub struct Nodo {
    /// `node_id`: `195@cid30>138@cid35`.
    pub id: String,
    /// ((nominal m/z, activation, collision energy), ...) of the path.
    pub chiave: Vec<(f64, String, f64)>,
    /// Index into `Albero::nodi` of the parent; `None` = the root MS1.
    pub genitore: Option<usize>,
    /// Generations missing between the parent and the node (`via`).
    pub salto: u8,
    pub livello: u8,
    /// Indices of its scans in the input.
    pub scansioni: Vec<usize>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Albero {
    /// The level-1 scans (the root).
    pub ms1: Vec<usize>,
    /// The nodes in the order (length of the path, path); a parent always comes before its children.
    pub nodi: Vec<Nodo>,
    /// MSn scans without a usable path (none in the filter, or not as long as the level asks).
    pub scartate: usize,
}

type Key = Vec<(i64, String, u64)>;

fn key_of(path: &[(f64, String, f64)]) -> Key {
    path.iter()
        .map(|(mz, act, ce)| (mz.round_ties_even() as i64, act.clone(), ce.to_bits()))
        .collect()
}

fn key_cmp(a: &Key, b: &Key) -> Ordering {
    a.len().cmp(&b.len()).then_with(|| {
        for (x, y) in a.iter().zip(b) {
            let o =
                x.0.cmp(&y.0)
                    .then_with(|| x.1.cmp(&y.1))
                    .then_with(|| f64::from_bits(x.2).total_cmp(&f64::from_bits(y.2)));
            if o.is_ne() {
                return o;
            }
        }
        Ordering::Equal
    })
}

/// `node_id`: `195@cid30>138@cid35`, `MS1` for the empty path.
pub fn node_id(chiave: &[(f64, String, f64)]) -> String {
    if chiave.is_empty() {
        return "MS1".into();
    }
    chiave
        .iter()
        .map(|(mz, act, ce)| {
            format!(
                "{}@{}{}",
                mz.round_ties_even() as i64,
                act.to_lowercase(),
                crate::json::fmt_g(*ce)
            )
        })
        .collect::<Vec<_>>()
        .join(">")
}

/// `find_parent`: the node of path[:-1] (the same energy if it exists, else the one with the most scans); the nearest ancestor when that
/// generation is not in the file. Returns it (empty = the root) and the depth.
fn find_parent(
    k: &Key,
    by_nominal: &HashMap<Vec<i64>, Vec<Key>>,
    groups: &HashMap<Key, Vec<usize>>,
) -> (Key, usize) {
    let (mut pk, mut depth) = (k[..k.len() - 1].to_vec(), 1);
    loop {
        if pk.is_empty() {
            return (pk, depth);
        }
        let nom: Vec<i64> = pk.iter().map(|x| x.0).collect();
        if let Some(cand) = by_nominal.get(&nom).filter(|c| !c.is_empty()) {
            if cand.contains(&pk) {
                return (pk, depth);
            }
            let mut best = &cand[0];
            for c in cand {
                if groups[c].len() > groups[best].len() {
                    best = c;
                }
            }
            return (best.clone(), depth);
        }
        pk.pop();
        depth += 1;
    }
}

/// `consensus_arrays(mzs, ints, ppm, min_frac, min_rel)`: (m/z, mean relative intensity over all the spectra, number of spectra with the peak).
pub fn consenso(
    spectra: &[(Vec<f64>, Vec<f64>)],
    ppm: f64,
    min_frac: f64,
    min_rel: f64,
) -> (Vec<f64>, Vec<f64>, Vec<usize>) {
    let n = spectra.len();
    let mut pk: Vec<(f64, f64, usize)> = vec![];
    for (i, (m, it)) in spectra.iter().enumerate() {
        let mx = it.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        if m.is_empty() || mx.is_nan() || mx <= 0.0 {
            continue;
        }
        for (&a, &b) in m.iter().zip(it) {
            let r = b / mx;
            if r > 0.0 {
                pk.push((a, r, i));
            }
        }
    }
    if pk.is_empty() {
        return (vec![], vec![], vec![]);
    }
    crate::sortc::by(&mut pk, |a, b| a.0.total_cmp(&b.0));
    // groups: a new one where the step to the next m/z exceeds ppm of that m/z
    let (mut w, mut wm, mut seen): (Vec<f64>, Vec<f64>, Vec<Vec<usize>>) = (vec![], vec![], vec![]);
    for (j, &(m, r, s)) in pk.iter().enumerate() {
        if j == 0 || m - pk[j - 1].0 > m * ppm * 1e-6 {
            w.push(0.0);
            wm.push(0.0);
            seen.push(vec![]);
        }
        let g = w.len() - 1;
        w[g] += r;
        wm[g] += r * m;
        if !seen[g].contains(&s) {
            seen[g].push(s);
        }
    }
    let need = (min_frac * n as f64).max(1.0);
    let (mut cm, mut mean, mut nsc) = (vec![], vec![], vec![]);
    for g in 0..w.len() {
        let m = w[g] / n as f64;
        if seen[g].len() as f64 >= need && m >= min_rel {
            cm.push(wm[g] / w[g]);
            mean.push(m);
            nsc.push(seen[g].len());
        }
    }
    (cm, mean, nsc)
}

/// The tree of the scans of an MSn file (`.mzidx`).
pub fn albero(scans: &[Scan], r: &Raggruppa) -> Albero {
    albero_di(scans.iter().map(|s| (s.level, s.path.as_deref())), r)
}

/// The tree of scans given as (level, path of the filter).
pub fn albero_di<'a>(
    scans: impl IntoIterator<Item = (u8, Option<&'a [(f64, String, f64)]>)>,
    _r: &Raggruppa,
) -> Albero {
    let mut out = Albero::default();
    let mut groups: HashMap<Key, Vec<usize>> = HashMap::new();
    for (i, (level, path)) in scans.into_iter().enumerate() {
        match path {
            _ if level == 1 => out.ms1.push(i),
            Some(p) if !p.is_empty() && p.len() + 1 == level as usize => {
                groups.entry(key_of(p)).or_default().push(i)
            }
            _ => out.scartate += 1,
        }
    }
    let mut keys: Vec<Key> = groups.keys().cloned().collect();
    keys.sort_by(key_cmp);
    let mut by_nominal: HashMap<Vec<i64>, Vec<Key>> = HashMap::new();
    for k in &keys {
        by_nominal
            .entry(k.iter().map(|x| x.0).collect())
            .or_default()
            .push(k.clone());
    }
    let at: HashMap<&Key, usize> = keys.iter().enumerate().map(|(i, k)| (k, i)).collect();
    for k in &keys {
        let (pk, depth) = find_parent(k, &by_nominal, &groups);
        let chiave: Vec<(f64, String, f64)> = k
            .iter()
            .map(|(m, a, ce)| (*m as f64, a.clone(), f64::from_bits(*ce)))
            .collect();
        out.nodi.push(Nodo {
            id: node_id(&chiave),
            livello: (k.len() + 1) as u8,
            genitore: at.get(&pk).copied(),
            salto: (depth - 1) as u8,
            scansioni: groups[k].clone(),
            chiave,
        });
    }
    out
}

#[cfg(test)]
mod tests;
