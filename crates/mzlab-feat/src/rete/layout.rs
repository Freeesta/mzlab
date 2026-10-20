//! The columns of the network: the step of a node is its distance in arcs from the parent (arcs read both ways); a group of nodes that no arc joins to
//! the parent starts at step 1 from its earliest node (by time of the maximum, m/z, RT, id). Inside a column the nodes go by m/z, then RT, then id.

use std::collections::VecDeque;

use super::{Node, ReteRow};
use crate::tp::TpBlock;

/// The nodes touched by an arc, and the parent. `all` is the parent then the products in the order of m/z; `arcs` are indexes into `all`.
pub(super) fn nodes(all: &[&ReteRow], arcs: &[(usize, usize)], tp: &TpBlock) -> Vec<Node> {
    let n = all.len();
    let mut adj = vec![Vec::new(); n];
    for &(a, b) in arcs {
        adj[a].push(b);
        adj[b].push(a);
    }
    let mut step: Vec<Option<u32>> = vec![None; n];
    let mut queue = VecDeque::new();
    step[0] = Some(0);
    queue.push_back(0);
    loop {
        while let Some(k) = queue.pop_front() {
            for &j in &adj[k] {
                if step[j].is_none() {
                    step[j] = Some(step[k].unwrap_or(0) + 1);
                    queue.push_back(j);
                }
            }
        }
        // the earliest node of a group the parent does not reach
        let root = (1..n)
            .filter(|&k| step[k].is_none() && !adj[k].is_empty())
            .min_by(|&a, &b| {
                let t = |k: usize| all[k].tmax.unwrap_or(f64::INFINITY);
                t(a).total_cmp(&t(b)).then(a.cmp(&b))
            });
        match root {
            Some(k) => {
                step[k] = Some(1);
                queue.push_back(k);
            }
            None => break,
        }
    }
    let mut out: Vec<Node> = (0..n)
        .filter(|&k| step[k].is_some())
        .map(|k| Node {
            row: all[k].clone(),
            kinship: if k == 0 {
                0
            } else {
                tp.kinship.get(all[k].row).copied().unwrap_or(0)
            },
            step: step[k].unwrap_or(0),
            pos: 0,
        })
        .collect();
    out.sort_by(|a, b| {
        a.step
            .cmp(&b.step)
            .then(a.row.mz.total_cmp(&b.row.mz))
            .then(a.row.rt.total_cmp(&b.row.rt))
            .then(a.row.id.cmp(&b.row.id))
    });
    let mut last = (u32::MAX, 0);
    for node in &mut out {
        if node.step != last.0 {
            last = (node.step, 0);
        }
        node.pos = last.1;
        last.1 += 1;
    }
    out
}
