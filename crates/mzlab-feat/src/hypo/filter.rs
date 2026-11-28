//! The optional formula filter of `tp.generate` (`formula_filter`, off by default): a candidate stays only if its formula is reachable from the parent in
//! at most `filter_steps` rule steps (the same rules and groups as the generator) and its expected M+2/M is within `m2_tol` of the reference ratio, the
//! parent's own from its formula or an observed value (`m2_ref`). Candidates without a formula and cleavage products of the structure skip the test they cannot
//! take; every count is written in the registry (`filter`), nothing is dropped silently.

use std::collections::BTreeSet;

use mzlab_core::composition::isotope_ratios;
use mzlab_core::elements;

use super::rules::{Counts, Rules};
use super::{counts_of, sources, HypoParams, Hypothesis, SRC_CLEAVAGE};

/// What the filter did: the settings used and where each candidate went.
#[derive(Debug, Clone, PartialEq)]
pub struct FilterLog {
    pub steps: u8,
    pub m2_tol: f64,
    pub m2_ref: f64,
    /// `observed` (the `m2_ref` parameter) or `parent_formula`.
    pub m2_source: &'static str,
    pub seen: usize,
    pub kept: usize,
    pub unreachable: usize,
    pub m2_off: usize,
    /// Candidates with no formula (a mass only): kept, neither test applies.
    pub unchecked: usize,
    /// Cleavage products of the structure: kept without the reachability test.
    pub structural: usize,
}

fn m2_of(c: &Counts) -> f64 {
    let v: Vec<(String, i64)> = c.iter().map(|(e, n)| (e.clone(), *n)).collect();
    isotope_ratios(&v).1
}

/// `raw` without the candidates the filter rejects, and the log. `parent` and `groups` are those of the generator.
pub fn apply(
    raw: Vec<Hypothesis>,
    rules: &Rules,
    parent: &Counts,
    groups: Option<&[String]>,
    p: &HypoParams,
) -> (Vec<Hypothesis>, FilterLog) {
    let reach_p = HypoParams {
        max_steps: p.filter_steps,
        ..p.clone()
    };
    let mut reach: BTreeSet<Counts> = sources::from_rules(rules, parent, groups, &reach_p)
        .iter()
        .filter_map(|h| elements::parse_formula(&h.formula).ok())
        .map(|f| counts_of(&f))
        .collect();
    reach.insert(parent.clone());
    let (m2_ref, m2_source) = match p.m2_ref {
        Some(v) => (v, "observed"),
        None => (m2_of(parent), "parent_formula"),
    };
    let mut log = FilterLog {
        steps: p.filter_steps,
        m2_tol: p.m2_tol,
        m2_ref,
        m2_source,
        seen: raw.len(),
        kept: 0,
        unreachable: 0,
        m2_off: 0,
        unchecked: 0,
        structural: 0,
    };
    let mut out = Vec::with_capacity(raw.len());
    for h in raw {
        let Ok(f) = elements::parse_formula(&h.formula) else {
            log.unchecked += 1;
            log.kept += 1;
            out.push(h);
            continue;
        };
        let c = counts_of(&f);
        if h.sources & SRC_CLEAVAGE != 0 {
            log.structural += 1;
        } else if !reach.contains(&c) {
            log.unreachable += 1;
            continue;
        }
        if (m2_of(&c) - m2_ref).abs() > p.m2_tol {
            log.m2_off += 1;
            continue;
        }
        log.kept += 1;
        out.push(h);
    }
    (out, log)
}
