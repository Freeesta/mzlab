//! The observed network: what the table itself says about how the products chain. Two readings, both of rows already formed (gate 1) and not in-source fragments
//! (gate 2), never of the whole table:
//!
//! - `explain_by_arcs`: between rows whose areas co-vary along the series (shape of the kinetics, not coelution) the differences of mass of the rules are
//!   looked for at `ppm`; an arc runs from the row the rule starts at to the row it gives. A row no rule had explained, reached by an arc from the parent or
//!   from an explained row, is explained by the network (`VIA_NETWORK`) up to `max_steps` steps from the parent. Rules with a precondition on the groups
//!   or the elements of the parent are not used: the rows have no formula.
//! - `next_generation`: the accepted products with a kinship of at least `min_kinship` and a hypothesis with a formula become the parent of a second round of
//!   rules (one step), labelled `SRC_NETWORK`; the caller searches them like any other hypothesis. One pass, fixed order: the same table gives the same list.

use mzlab_core::elements::{self, mass_of};

use super::*;
use crate::aligned::{Aligned, CELL_ABSENT};
use crate::evid::match_hypotheses;
use crate::hypo::{counts_of, HypoParams, Hypothesis, Registry, Rules, SRC_NETWORK};
use crate::trace::filled;

/// Thresholds of the network.
#[derive(Debug, Clone, PartialEq)]
pub struct NetParams {
    /// Smallest correlation of the areas of two rows along the series for an arc.
    pub corr: f64,
    /// Fewest times with both rows measured to judge the correlation.
    pub points: usize,
    /// Tolerance on the difference of mass of a rule, ppm of the heavier m/z.
    pub ppm: f64,
    /// Most steps from the parent, in all (the third generation).
    pub max_steps: u8,
    /// Lowest kinship of a product to be the parent of the next round.
    pub min_kinship: u8,
}

impl Default for NetParams {
    fn default() -> Self {
        NetParams {
            corr: 0.8,
            points: 4,
            ppm: 5.0,
            max_steps: 3,
            min_kinship: KIN_PROBABLE,
        }
    }
}

/// An arc of the network: `from` becomes `to` by the rule `rule` (index in `Rules::rules`); `generation` is the steps from the parent of `to`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Arc {
    pub from: u32,
    pub to: u32,
    pub rule: u32,
    pub generation: u8,
}

/// Signed change of mass of a rule.
fn delta_mass(r: &crate::hypo::Rule) -> f64 {
    r.delta
        .iter()
        .map(|(e, n)| mass_of(e).unwrap_or(0.0) * *n as f64)
        .sum()
}

/// Areas of a row at the treated times, in time order (None where it has no peak).
fn series(inp: &TpInputs, files: &[usize], row: usize) -> Vec<Option<f64>> {
    let t = inp.aligned;
    files
        .iter()
        .map(|&k| {
            let c = row * t.n_files + k;
            (t.state[c] != CELL_ABSENT && t.area[c].is_finite() && t.area[c] > 0.0)
                .then_some(t.area[c])
        })
        .collect()
}

/// Pearson correlation over the times with both measured; None when there are fewer than `points` or a row is flat.
fn correlation(a: &[Option<f64>], b: &[Option<f64>], points: usize) -> Option<f64> {
    let pairs: Vec<(f64, f64)> = a
        .iter()
        .zip(b)
        .filter_map(|(x, y)| Some(((*x)?, (*y)?)))
        .collect();
    if pairs.len() < points.max(2) {
        return None;
    }
    let n = pairs.len() as f64;
    let (ma, mb) = (
        pairs.iter().map(|p| p.0).sum::<f64>() / n,
        pairs.iter().map(|p| p.1).sum::<f64>() / n,
    );
    let (mut sab, mut saa, mut sbb) = (0.0, 0.0, 0.0);
    for (x, y) in pairs {
        sab += (x - ma) * (y - mb);
        saa += (x - ma).powi(2);
        sbb += (y - mb).powi(2);
    }
    (saa > 0.0 && sbb > 0.0).then(|| sab / (saa * sbb).sqrt())
}

/// The arcs among the rows that can be in the network (formed, not in source), sorted by `from`, `to`, `rule`.
pub fn arcs(inp: &TpInputs, rules: &Rules, b: &TpBlock, np: &NetParams) -> Vec<Arc> {
    arcs_in(inp, rules, b, np, &[])
}

/// The treated files in time order (all of them, or, in `by`, one list per condition: the files of the condition).
fn series_files(inp: &TpInputs, by: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let mut lists: Vec<Vec<usize>> = if by.len() < 2 {
        vec![(0..inp.minutes.len())
            .filter(|&k| inp.minutes[k] > 0.0)
            .collect()]
    } else {
        by.iter()
            .map(|l| l.iter().copied().filter(|&k| inp.minutes[k] > 0.0).collect())
            .collect()
    };
    for l in &mut lists {
        crate::sortc::by(l, |&x, &y| inp.minutes[x].total_cmp(&inp.minutes[y]));
    }
    lists
}

/// `arcs` with the files of each condition (`by`, treated files of one condition per list; fewer than two lists = one series, as `arcs`). With several
/// conditions the correlation along the series is read inside each condition, over its own treated times (at least `points`, so a condition with fewer
/// times gives no arc), and an arc holds when it holds in at least one condition.
pub fn arcs_in(
    inp: &TpInputs,
    rules: &Rules,
    b: &TpBlock,
    np: &NetParams,
    by: &[Vec<usize>],
) -> Vec<Arc> {
    let t = inp.aligned;
    let lists = series_files(inp, by);
    let lists: Vec<&Vec<usize>> = lists
        .iter()
        .filter(|l| by.len() < 2 || l.len() >= np.points.max(2))
        .collect();
    let mut rows: Vec<usize> = (0..t.len())
        .filter(|&r| matches!(b.status[r], ST_TP | ST_UNEXPLAINED | ST_PARENT))
        .collect();
    crate::sortc::by(&mut rows, |&x, &y| {
        t.mz[x].total_cmp(&t.mz[y]).then(x.cmp(&y))
    });
    let mut out = Vec::new();
    for (k, rule) in rules.rules.iter().enumerate() {
        if !rule.groups_all.is_empty()
            || !rule.groups_any.is_empty()
            || !rule.all.is_empty()
            || !rule.any.is_empty()
        {
            continue;
        }
        let dm = delta_mass(rule);
        if dm == 0.0 {
            continue;
        }
        for (i, &lo) in rows.iter().enumerate() {
            let want = t.mz[lo] + dm.abs();
            let tol = want * np.ppm * 1e-6;
            let from = i + rows[i..].partition_point(|&h| t.mz[h] < want - tol);
            for &hi in rows[from..].iter().take_while(|&&h| t.mz[h] <= want + tol) {
                let (src, dst) = if dm > 0.0 { (lo, hi) } else { (hi, lo) };
                if b.status[dst] == ST_PARENT {
                    continue;
                }
                // the parent decays while its products rise: from the parent the coupling is the opposite one
                let sign = if b.status[src] == ST_PARENT {
                    -1.0
                } else {
                    1.0
                };
                let holds = lists.iter().any(|files| {
                    let (a, c) = (series(inp, files, src), series(inp, files, dst));
                    correlation(&a, &c, np.points).is_some_and(|x| sign * x >= np.corr)
                });
                if holds {
                    out.push(Arc {
                        from: src as u32,
                        to: dst as u32,
                        rule: k as u32,
                        generation: 0,
                    });
                }
            }
        }
    }
    crate::sortc::by(&mut out, |x, y| {
        (x.from, x.to, x.rule).cmp(&(y.from, y.to, y.rule))
    });
    out
}

/// Steps from the parent of the rows already explained: the steps of the hypothesis that covers them (at least one), 0 for the parent.
fn anchors(inp: &TpInputs, b: &TpBlock) -> Result<Vec<u8>> {
    let matched = match_hypotheses(inp.hyps, inp.aligned);
    let mut g = filled(inp.aligned.len(), 0u8)?;
    for (r, x) in g.iter_mut().enumerate() {
        *x = match b.status[r] {
            ST_PARENT => 1,
            ST_TP => matched[r].map_or(1, |h| inp.hyps[h].steps.max(1)) + 1,
            _ => 0,
        };
    }
    Ok(g)
}

/// Explains by the network the formed rows that nothing else explained: the rows reached by an arc from the parent or from an explained row. The status of
/// those goes from unexplained to product and `VIA_NETWORK` is set; returns the arcs used (with the steps from the parent in `generation`).
/// Generations are counted from 1 for the first step from the parent.
pub fn explain_by_arcs(
    inp: &TpInputs,
    rules: &Rules,
    b: &mut TpBlock,
    np: &NetParams,
) -> Result<Vec<Arc>> {
    explain_by_arcs_in(inp, rules, b, np, &[])
}

/// `explain_by_arcs` with the arcs of `arcs_in` (the treated files of each condition in `by`).
pub fn explain_by_arcs_in(
    inp: &TpInputs,
    rules: &Rules,
    b: &mut TpBlock,
    np: &NetParams,
    by: &[Vec<usize>],
) -> Result<Vec<Arc>> {
    let all = arcs_in(inp, rules, b, np, by);
    // g[r]: 0 = not reached, else steps from the parent + 1 (the parent is 1)
    let mut g = anchors(inp, b)?;
    let mut used = Vec::new();
    loop {
        let mut changed = false;
        for a in &all {
            let (s, d) = (a.from as usize, a.to as usize);
            if g[s] == 0 || g[d] != 0 || g[s] > np.max_steps {
                continue;
            }
            g[d] = g[s] + 1;
            if b.status[d] == ST_UNEXPLAINED {
                b.status[d] = ST_TP;
            }
            b.via[d] |= VIA_NETWORK;
            used.push(Arc {
                generation: g[d] - 1,
                ..*a
            });
            changed = true;
        }
        if !changed {
            break;
        }
    }
    Ok(used)
}

/// The hypotheses of the next round, one step from the accepted products: not in `reg` and not repeated. Ids are given by `Registry::append`.
pub fn next_generation(
    rules: &Rules,
    reg: &Registry,
    aligned: &Aligned,
    b: &TpBlock,
    p: &HypoParams,
    np: &NetParams,
) -> Vec<Hypothesis> {
    let matched = match_hypotheses(&reg.hyps, aligned);
    let one = HypoParams {
        max_steps: 1,
        ..p.clone()
    };
    let mut out: Vec<Hypothesis> = Vec::new();
    for (r, m) in matched.iter().enumerate() {
        let Some(h) = *m else { continue };
        let h = &reg.hyps[h];
        if b.status.get(r) != Some(&ST_TP)
            || b.kinship[r] < np.min_kinship
            || h.formula.is_empty()
            || h.steps >= np.max_steps
        {
            continue;
        }
        let Ok(f) = elements::parse_formula(&h.formula) else {
            continue;
        };
        for mut n in crate::hypo::from_rules(rules, &counts_of(&f), None, &one) {
            n.label = format!("{}>{}", h.label, n.label);
            n.sources = SRC_NETWORK;
            n.steps = h.steps + 1;
            n.level = n.level.max(h.level);
            let key = |x: &Hypothesis| (x.formula.clone(), x.adduct);
            if !reg.hyps.iter().chain(&out).any(|x| key(x) == key(&n)) {
                out.push(n);
            }
        }
    }
    out
}
