//! `msn._atom_sets`: candidate substructures of every node from its formula, then the hierarchical constraint to a fixed point (a child candidate
//! must be a subset of a candidate of its parent, a parent candidate must contain a candidate of each child that has any), then the candidates
//! of every clean peak inside the node's own.

use super::{Cand, MsnTree};
use crate::legacy::fragtree::vec_of;

/// `(cm[:, None] & ~pm[None, :]) == 0`: rows = child candidates, columns = parent candidates.
fn subset(c: &[u64], p: &[u64]) -> Vec<Vec<bool>> {
    c.iter()
        .map(|&a| p.iter().map(|&b| a & !b == 0).collect())
        .collect()
}

fn filtered(c: &Cand, ok: &[bool]) -> Cand {
    let idx: Vec<usize> = (0..ok.len()).filter(|&i| ok[i]).map(|i| c.0[i]).collect();
    let w: Vec<f64> = (0..ok.len()).filter(|&i| ok[i]).map(|i| c.1[i]).collect();
    let s = w.iter().fold(0.0, |a, b| a + b).max(1e-300);
    (idx, w.iter().map(|x| x / s).collect())
}

/// One pass over every parent and child: true when a set changed.
fn constrain(
    cand: &mut std::collections::HashMap<String, Cand>,
    kids: &[(String, Vec<String>)],
    masks: &[u64],
) -> bool {
    let mut changed = false;
    for (p, cs) in kids {
        for c in cs {
            let (pc, cc) = (cand[p].clone(), cand[c].clone());
            if pc.0.is_empty() || cc.0.is_empty() {
                continue;
            }
            let pm: Vec<u64> = pc.0.iter().map(|&i| masks[i]).collect();
            let cm: Vec<u64> = cc.0.iter().map(|&i| masks[i]).collect();
            let sub = subset(&cm, &pm);
            let okc: Vec<bool> = sub.iter().map(|r| r.iter().any(|&x| x)).collect();
            let okp: Vec<bool> = (0..pm.len()).map(|j| sub.iter().any(|r| r[j])).collect();
            if !okc.iter().all(|&x| x) {
                cand.insert(c.clone(), filtered(&cc, &okc));
                changed = true;
            }
            if okp.iter().any(|&x| x) && !okp.iter().all(|&x| x) {
                cand.insert(p.clone(), filtered(&pc, &okp));
                changed = true;
            }
        }
    }
    changed
}

pub(super) fn atom_sets(tree: &mut MsnTree) {
    let Some(subs) = tree.subs.as_ref() else {
        return;
    };
    let mut cand = std::collections::HashMap::new();
    for n in tree.nodes.iter().filter(|n| n.id != "MS1") {
        if let Some(v) = n.formula.as_deref().and_then(|f| vec_of(f, &tree.els)) {
            cand.insert(n.id.clone(), subs.candidates(&v));
        }
    }
    let mut kids: Vec<(String, Vec<String>)> = Vec::new();
    for n in &tree.nodes {
        let Some(p) = n
            .parent
            .as_ref()
            .filter(|p| cand.contains_key(*p) && cand.contains_key(&n.id))
        else {
            continue;
        };
        match kids.iter_mut().find(|k| k.0 == *p) {
            Some(k) => k.1.push(n.id.clone()),
            None => kids.push((p.clone(), vec![n.id.clone()])),
        }
    }
    for _ in 0..10 {
        if !constrain(&mut cand, &kids, &subs.masks) {
            break;
        }
    }
    let n_atoms = subs.g.atoms.len();
    for n in tree.nodes.iter_mut() {
        let Some((idx, _)) = cand.get(&n.id) else {
            continue;
        };
        let ms: Vec<u64> = idx.iter().map(|&i| subs.masks[i]).collect();
        let bit = |j: usize, all: bool| {
            let f = |m: &u64| (m >> j) & 1 == 1;
            if all {
                ms.iter().all(f)
            } else {
                ms.iter().any(f)
            }
        };
        let (certain, possible) = if ms.is_empty() {
            (Vec::new(), Vec::new())
        } else {
            (
                (0..n_atoms).filter(|&j| bit(j, true)).collect(),
                (0..n_atoms).filter(|&j| bit(j, false)).collect(),
            )
        };
        n.atom_candidates = Some((idx.len(), certain, possible));
        for (j, pk) in n.peaks.iter_mut().enumerate() {
            let (mut pi, mut pw) = vec_of(&pk.formula, &tree.els)
                .map_or((Vec::new(), Vec::new()), |v| subs.candidates(&v));
            if !pi.is_empty() && !ms.is_empty() {
                let ok: Vec<usize> = (0..pi.len())
                    .filter(|&k| ms.iter().any(|&p| subs.masks[pi[k]] & !p == 0))
                    .collect();
                (pi, pw) = (
                    ok.iter().map(|&k| pi[k]).collect(),
                    ok.iter().map(|&k| pw[k]).collect(),
                );
                let s: f64 = pw.iter().fold(0.0, |a, b| a + b);
                pw = pw.iter().map(|x| x / s).collect();
            }
            pk.n_atom_candidates = Some(pi.len());
            tree.peak_cand.insert((n.id.clone(), j), (pi, pw));
        }
    }
    tree.cand = cand;
}
