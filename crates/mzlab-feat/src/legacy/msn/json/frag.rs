//! `fragtree._hr_msn` and `fragtree.compare_hr` on an MSn tree: the fragmentation tree of the parent and the marks of a product on it.

use mzlab_core::json::{round, Out};

use super::super::localize::Localization;
use super::super::{MsnTree, Node};
use super::{opt_str, rounded};
use crate::legacy::fragtree::{loss_mass, loss_str, vec_of, COMPARE_NONE, MIN_REL};

/// One node of `frag_tree`.
struct FNode {
    id: String,
    parent: Option<String>,
    level: usize,
    frag: bool,
    mz: Option<f64>,
    formula: Option<String>,
    rel: Option<f64>,
    loss: Option<String>,
    ppm: Option<f64>,
    n_scans: usize,
    ce: Option<f64>,
    ghost: bool,
    smiles: Vec<(String, f64)>,
}

/// `parent_rel` of `_hr_msn`: the strongest peak of the parent within 0.5 of the precursor.
fn parent_rel(t: &MsnTree, n: &Node, drop: bool) -> Option<f64> {
    if n.parent.is_none() || (drop && n.parent.as_deref() == Some("MS1")) {
        return Some(100.0);
    }
    let par = t.nodes.iter().find(|x| Some(&x.id) == n.parent.as_ref())?;
    let pm = n.prec_mz?;
    par.peaks
        .iter()
        .filter(|p| (p.mz - pm).abs() <= 0.5)
        .map(|p| p.rel)
        .reduce(f64::max)
        .map(|r| round(r, 1))
}

fn top_smiles(t: &MsnTree, c: Option<&super::super::Cand>) -> Vec<(String, f64)> {
    match (t.subs.as_ref(), c) {
        (Some(s), Some((idx, w))) if !idx.is_empty() => s.top_smiles(idx, w),
        _ => Vec::new(),
    }
}

fn ions(t: &MsnTree, smiles: Option<&str>, drop: bool) -> Vec<FNode> {
    let vec = |id: &str| {
        t.nodes
            .iter()
            .find(|n| n.id == id)
            .and_then(|n| n.formula.as_deref())
            .and_then(|f| vec_of(f, &t.els))
    };
    let root: Vec<(String, f64)> = smiles
        .map(|s| vec![(s.to_string(), 1.0)])
        .unwrap_or_default();
    let mut out = Vec::new();
    for n in t.nodes.iter().filter(|n| !(drop && n.id == "MS1")) {
        let mut sm = top_smiles(t, t.cand.get(&n.id));
        if (n.id == "MS1" || (drop && n.parent.as_deref() == Some("MS1"))) && sm.is_empty() {
            sm = root.clone();
        }
        let loss = match (n.parent.as_deref().and_then(vec), vec(&n.id)) {
            (Some(p), Some(v)) => loss_str(&p, &v, &t.els),
            _ => None,
        };
        out.push(FNode {
            id: n.id.clone(),
            parent: n.parent.clone().filter(|p| !(drop && p == "MS1")),
            level: n.level,
            frag: false,
            mz: rounded(n.prec_mz, 4),
            formula: n.formula.clone(),
            rel: if n.id == "MS1" {
                Some(100.0)
            } else {
                parent_rel(t, n, drop)
            },
            loss,
            ppm: rounded(n.ppm, 1),
            n_scans: n.n_scans,
            ce: n.ce,
            ghost: n.ghost,
            smiles: sm,
        });
    }
    out
}

/// The strong peaks of every spectrum that are not themselves an isolated precursor.
fn frags(t: &MsnTree, out: &mut Vec<FNode>) {
    for n in t.nodes.iter().filter(|n| !n.peaks.is_empty()) {
        let isolated: Vec<&Node> = t
            .nodes
            .iter()
            .filter(|c| c.parent.as_ref() == Some(&n.id))
            .collect();
        let nv = n.formula.as_deref().and_then(|f| vec_of(f, &t.els));
        for (j, p) in n.peaks.iter().enumerate() {
            if p.rel < MIN_REL || p.formula.is_empty() {
                continue;
            }
            if isolated.iter().any(|c| {
                c.formula.as_deref() == Some(&p.formula)
                    || c.prec_mz.is_some_and(|m| (m - p.mz).abs() <= 0.5)
            }) {
                continue;
            }
            if n.formula.as_deref() == Some(&p.formula) {
                continue;
            }
            let loss = match (&nv, vec_of(&p.formula, &t.els)) {
                (Some(a), Some(b)) => loss_str(a, &b, &t.els),
                _ => None,
            };
            out.push(FNode {
                id: format!("{}#{j}", n.id),
                parent: Some(n.id.clone()),
                level: n.level,
                frag: true,
                mz: Some(round(p.mz, 4)),
                formula: Some(p.formula.clone()),
                rel: Some(round(p.rel, 1)),
                loss,
                ppm: Some(round(p.ppm, 1)),
                n_scans: p.n_scans,
                ce: n.ce,
                ghost: false,
                smiles: top_smiles(t, t.peak_cand.get(&(n.id.clone(), j))),
            });
        }
    }
}

fn nodes(t: &MsnTree, smiles: Option<&str>) -> Vec<FNode> {
    let kids: Vec<&Node> = t
        .nodes
        .iter()
        .filter(|n| n.parent.as_deref() == Some("MS1"))
        .collect();
    let drop = kids.len() == 1 && kids[0].formula == t.nodes[0].formula;
    let mut out = ions(t, smiles, drop);
    frags(t, &mut out);
    out
}

fn fnode(o: &mut Out, n: &FNode) {
    o.raw("{\"id\":").str(&n.id).raw(",\"parent\":");
    opt_str(o, n.parent.as_deref());
    o.raw(",\"level\":")
        .int(n.level as i64)
        .raw(",\"kind\":")
        .str(if n.frag { "frag" } else { "ion" });
    o.raw(",\"mz\":").opt(n.mz).raw(",\"formula\":");
    opt_str(o, n.formula.as_deref());
    o.raw(",\"rel\":").opt(n.rel).raw(",\"loss\":");
    opt_str(o, n.loss.as_deref());
    o.raw(",\"loss_mass\":")
        .opt(n.loss.as_deref().and_then(loss_mass))
        .raw(",\"ppm\":")
        .opt(n.ppm);
    o.raw(",\"n_scans\":")
        .int(n.n_scans as i64)
        .raw(",\"ce\":")
        .opt(n.ce)
        .raw(",\"ghost\":")
        .raw(if n.ghost { "true" } else { "false" });
    o.raw(",\"smiles\":[");
    for (i, (s, w)) in n.smiles.iter().enumerate() {
        o.raw(if i > 0 {
            ",{\"smiles\":"
        } else {
            "{\"smiles\":"
        })
        .str(s)
        .raw(",\"score\":")
        .num(*w)
        .raw("}");
    }
    o.raw("],\"alternatives\":[]}");
}

/// `fragtree._hr_msn`: the tree of the MSn file (`smiles` = the parent's, the structure of the root).
pub fn frag_tree(t: &MsnTree, smiles: Option<&str>) -> String {
    let mut o = Out::default();
    o.raw("{\"ok\":true,\"mode\":\"hr\",\"supposed\":false,\"note\":\"\",\"nodes\":[");
    for (i, n) in nodes(t, smiles).iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" });
        fnode(&mut o, n);
    }
    o.raw("]}");
    o.0
}

/// `fragtree.compare_hr(frag_tree, localization)`: the nodes of the tree found unshifted (`same`) or shifted in the product.
pub fn compare(t: &MsnTree, loc: Option<&Localization>) -> String {
    let Some(l) = loc.filter(|l| !l.evidence.is_empty()) else {
        return COMPARE_NONE.into();
    };
    let has = |f: &str, s: bool| l.evidence.iter().any(|e| e.shifted == s && e.library == f);
    let all = nodes(t, None);
    let (mut o, mut same, mut shifted, mut k) = (Out::default(), 0, 0, 0);
    o.raw("{\"ok\":true,\"marks\":{");
    for n in &all {
        let Some(f) = n.formula.as_deref() else {
            continue;
        };
        let state = if has(f, false) {
            "same"
        } else if has(f, true) {
            "shifted"
        } else {
            continue;
        };
        o.raw(if k > 0 { "," } else { "" })
            .str(&n.id)
            .raw(":{\"state\":")
            .str(state);
        if state == "shifted" {
            o.raw(",\"delta\":")
                .str(&format!("{}{}", l.delta_sign, l.delta));
            shifted += 1;
        } else {
            same += 1;
        }
        o.raw("}");
        k += 1;
    }
    let n_nodes = all.iter().filter(|n| n.frag || n.id != "MS1").count();
    o.raw("},\"n_same\":")
        .int(same)
        .raw(",\"n_shifted\":")
        .int(shifted)
        .raw(",\"n_nodes\":")
        .int(n_nodes as i64)
        .raw(",\"note\":\"\"}");
    o.0
}
