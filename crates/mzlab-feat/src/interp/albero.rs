//! The fragmentation tree of the parent seen from a row: for each node of the MSn tree of the infusion the fragment is in the MS2 of the row as it is
//! (`same`), moved by the modification (`shifted`), missing (`absent`) or cannot be told because its m/z lies outside the scan range (`not_checkable`).
//! The nodes (`LibNode`) are kept in the meta of the `.mzfeat` (`msnlib`) so the tree is built once, not for every row.

use crate::legacy::fragtree::vec_of;
use crate::legacy::msn::{ion_mass, MsnTree};

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
