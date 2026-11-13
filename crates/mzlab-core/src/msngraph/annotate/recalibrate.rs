//! Narrowing of the formula candidates of a graph (`docs/agenti/msn-grafo.md` §3): the even-electron preference and the removal of the systematic
//! mass error. Called by `annota::annota` after the fixed point of the reduction.

use super::{evidence, retain_mask, Cand, State};
use crate::msngraph::{Graph, Options};

/// A node with an even-electron candidate within the window loses its radical-ion candidates that have no evidence (the even-electron rule:
/// most fragments are even-electron ions); a node with only radical candidates keeps them. True when a candidate left.
pub(super) fn prefer_even(g: &Graph, st: &mut State, o: &Options) -> bool {
    let mut changed = false;
    for k in 1..st.order.len() {
        let j = st.order[k];
        let even = |x: &Cand| st.e.even(&x.c, st.e.pol);
        if st.user[j] || !st.cand[j].iter().any(|x| x.strict && even(x)) {
            continue;
        }
        let keep: Vec<bool> = st.cand[j]
            .iter()
            .map(|x| even(x) || !evidence(g, st, o, j, &x.c).is_empty())
            .collect();
        let n = st.cand[j].len();
        retain_mask(&mut st.cand[j], &keep);
        changed |= st.cand[j].len() < n;
    }
    changed
}

/// Fewest nodes with one formula needed to estimate the systematic error of the instrument.
const MIN_PER_OFFSET: usize = 5;
/// Scatter (ppm) left around the systematic error once it is removed.
const RESIDUO_PPM: f64 = 2.0;

/// The error of the instrument is mostly systematic: the median ppm of the nodes with one narrow-window candidate is taken out, and a narrow-window
/// candidate more than the residue (`RESIDUO_PPM`, wider for a noisy instrument: 4.8 half-widths of the densest 40 % of the values) from it leaves (a chance match of the dense sub-formula lattice is not). Nothing happens under
/// `MIN_PER_OFFSET` such nodes or when the window is not wider than the residue. True when a candidate left.
pub(super) fn ricalibra(st: &mut State, o: &Options) -> bool {
    if o.ppm_formula <= RESIDUO_PPM {
        return false;
    }
    let mut v: Vec<f64> = st.order[1..]
        .iter()
        .filter(|&&j| !st.user[j] && st.cand[j].len() == 1 && st.cand[j][0].strict)
        .map(|&j| st.cand[j][0].ppm)
        .collect();
    if v.len() < MIN_PER_OFFSET {
        return false;
    }
    crate::sortc::slice_by(&mut v, |a, b| a.total_cmp(b));
    // the densest 40 % of the values (the shorth) is the cluster of the true matches: chance matches of the dense lattice are spread over the window
    // and do not widen it as they do a quantile; for a normal scatter that window is 1.05 sigma wide, so 2.5 sigmas are 4.8 half-widths
    let k = (v.len() * 2).div_ceil(5);
    let i = (0..=v.len() - k)
        .min_by(|&a, &b| (v[a + k - 1] - v[a]).total_cmp(&(v[b + k - 1] - v[b])))
        .unwrap_or(0);
    let meta = (v[i + k - 1] - v[i]) / 2.0;
    let offset = v[v.len() / 2];
    let residuo = RESIDUO_PPM.max(4.8 * meta);
    if residuo >= o.ppm_formula {
        return false;
    }
    let mut changed = false;
    for k in 1..st.order.len() {
        let j = st.order[k];
        if st.user[j] {
            continue;
        }
        let n = st.cand[j].len();
        st.cand[j].retain(|x| !x.strict || (x.ppm - offset).abs() <= residuo);
        changed |= st.cand[j].len() < n;
    }
    changed
}
