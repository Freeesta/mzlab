//! The fragmentation tree of the parent seen from a row: for each node of the MSn tree of the infusion the fragment is in the MS2 of the row as it is
//! (`same`), moved by the modification (`shifted`), missing (`absent`) or cannot be told because its m/z lies outside the scan range (`not_checkable`).
//! The nodes (`LibNode`) are kept in the meta of the `.mzfeat` (`msnlib`) so the tree is built once, not for every row.

use mzlab_core::elements::{self, parse_formula};
use mzlab_core::idxread::{Index, ReadAt};

use super::modifica::Candidate;
use super::sito::{self, Candidates, TreeVotes};
use crate::legacy::fragtree::vec_of;
use crate::legacy::ms2::Consensus;
use crate::legacy::msn::library::{self, Library};
use crate::legacy::msn::localize::localize_sites;
use crate::legacy::msn::{self, ion_mass, MsnTree};
use crate::legacy::series::Parent;

/// Tolerance of the matching of a node, ppm.
pub const NODE_PPM: f64 = 5.0;

/// A formula of the fragment library with its theoretical m/z; `id` is the node of the tree (`MS1` is the parent itself), empty for a DDA-only entry.
#[derive(Debug, Clone, PartialEq)]
pub struct LibNode {
    pub id: String,
    pub formula: String,
    pub mz: f64,
}

/// The nodes of the tree with a formula, root included.
pub fn nodes_of(tree: &MsnTree) -> Vec<LibNode> {
    tree.nodes
        .iter()
        .filter_map(|n| {
            let f = n.formula.as_deref()?;
            let v = vec_of(f, &tree.els)?;
            Some(LibNode {
                id: n.id.clone(),
                formula: f.to_string(),
                mz: ion_mass(&v, &tree.els),
            })
        })
        .collect()
}

/// State of one node.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeState {
    pub id: String,
    pub formula: String,
    pub mz: f64,
    pub state: &'static str,
}

/// `peaks` = m/z of the consensus of the row; `delta` = mass of the modification; `range` = the scan range of its MS2 (None = not known).
/// A node above the precursor of the row is not a fragment of it: `not_checkable`.
pub fn states(
    nodes: &[LibNode],
    peaks: &[f64],
    delta: f64,
    range: Option<(f64, f64)>,
    precursor: f64,
) -> Vec<NodeState> {
    let near = |m: f64, t: f64| (m - t).abs() <= t * NODE_PPM * 1e-6;
    nodes
        .iter()
        .filter(|n| n.id != "MS1")
        .map(|n| {
            let outside =
                |t: f64| range.is_some_and(|(lo, hi)| t < lo || t > hi) || t >= precursor + 1.5;
            let state = if peaks.iter().any(|&m| near(m, n.mz)) {
                "same"
            } else if peaks.iter().any(|&m| near(m, n.mz + delta)) {
                "shifted"
            } else if outside(n.mz) && outside(n.mz + delta) {
                "not_checkable"
            } else {
                "absent"
            };
            NodeState {
                id: n.id.clone(),
                formula: n.formula.clone(),
                mz: n.mz,
                state,
            }
        })
        .collect()
}

/// The MSn tree of the infusion and the library built on it, kept while the rows are interpreted (the position of a change votes with them).
pub struct TreeCtx {
    pub tree: MsnTree,
    pub lib: Library,
}

/// `(nodes, entries, tree)` of `library_of`.
pub type Built = (Vec<LibNode>, Vec<LibNode>, Option<TreeCtx>);

/// The tree of the infusion and the library with the DDA of the parent: `(nodes, entries, tree)`, empty without an infusion.
pub fn library_of<R: ReadAt>(
    infusion: Option<&mut Index<R>>,
    parent: &Parent,
    dda: &Consensus,
) -> Result<Built, String> {
    let Some(ix) = infusion else {
        return Ok((vec![], vec![], None));
    };
    let scans = msn::scans_of(ix)?;
    let ion = elements::fmt(&parent.ion);
    let tree = msn::build_tree(&scans, &ion, parent.smiles.as_deref(), "auto")?;
    let lib = library::build(
        &tree,
        (!dda.mz.is_empty()).then_some((&dda.mz[..], &dda.rel[..])),
    );
    let entries = lib
        .entries
        .iter()
        .map(|(f, e)| LibNode {
            id: String::new(),
            formula: f.clone(),
            mz: ion_mass(&e.vec, &tree.els),
        })
        .collect();
    Ok((nodes_of(&tree), entries, Some(TreeCtx { tree, lib })))
}

/// The votes of the tree for the sites of a change: `localize_sites` over the masks of the candidates. None without a usable tree (negative
/// polarity, a graph that is not the parent's, more than 64 atoms) or evidence.
pub fn tree_votes(
    ctx: &TreeCtx,
    cand: &Candidates,
    parent: &Parent,
    change: &Candidate,
    cons: &Consensus,
    mz: f64,
    polarity: i8,
) -> Option<TreeVotes> {
    let subs = ctx.tree.subs.as_ref().filter(|_| polarity >= 0)?;
    let masks = sito::masks(cand)?;
    let els = &ctx.tree.els;
    let tp = vec_of(&change.ion_formula, els)?;
    let reference = vec_of(&elements::fmt(&parent.ion), els)?;
    // The counts must hold every element of the formulas, and the graph of the tree must be the parent's.
    let atoms = |f: &str| {
        parse_formula(f)
            .ok()
            .map(|p| p.iter().map(|x| x.1 as i64).sum::<i64>())
    };
    let ion = elements::fmt(&parent.ion);
    if atoms(&change.ion_formula) != Some(tp.iter().sum())
        || atoms(&ion) != Some(reference.iter().sum())
        || subs.g.atoms.len() != cand.mol.atoms.len()
    {
        return None;
    }
    let loc = localize_sites(
        &tp,
        &reference,
        els,
        &cons.mz,
        &cons.rel,
        &ctx.lib,
        subs,
        Some(mz),
        &masks,
    );
    let found = loc.found?;
    (found.all_ll.len() == masks.len()).then_some(TreeVotes {
        ll: found.all_ll,
        retained: found.retained,
    })
}

/// The null distribution of the lead of the best site over the second (log-likelihood): `n` times the masks of the substructures are shuffled among
/// their formulas, which cuts the link between a fragment and its atoms, and the votes are computed again. A real lead must stand out of it.
#[allow(clippy::too_many_arguments)]
pub fn null_margins(
    ctx: &mut TreeCtx,
    cand: &Candidates,
    parent: &Parent,
    change: &Candidate,
    cons: &Consensus,
    mz: f64,
    n: usize,
    seed: u64,
) -> Vec<f64> {
    let Some(original) = ctx.tree.subs.as_ref().map(|s| s.masks_copy()) else {
        return vec![];
    };
    let mut state = seed | 1;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let mut out = vec![];
    for _ in 0..n {
        let mut masks = original.clone();
        for i in (1..masks.len()).rev() {
            masks.swap(i, (next() % (i as u64 + 1)) as usize);
        }
        if let Some(s) = ctx.tree.subs.as_mut() {
            s.set_masks(masks);
        }
        if let Some(v) = tree_votes(ctx, cand, parent, change, cons, mz, 1) {
            let mut ll = v.ll;
            ll.sort_by(|a, b| b.total_cmp(a));
            if ll.len() >= 2 {
                out.push(ll[0] - ll[1]);
            }
        }
    }
    if let Some(s) = ctx.tree.subs.as_mut() {
        s.set_masks(original);
    }
    out
}
