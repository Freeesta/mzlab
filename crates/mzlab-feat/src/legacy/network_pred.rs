//! The predecessors of the candidates (`hr/network.py::predecessors`): for every candidate with a formula, the best source among the parent and the other
//! candidates, by a known transformation (one step among candidates, up to two from the parent) or by an MS2 related to its own; they mark the edges of
//! the mass network as «both» or add the «network» edges that no single mass difference gives.

use super::dig::Cand;
use super::similarity::{pair_components, prepare};
use crate::hypo::{Counts, Rules};

/// Matched peaks and modified cosine from which two MS2 are related, and the kinetic slack (min).
const MIN_MATCHED: usize = 3;
const COS_RELATED: f64 = 0.6;
const STEP: f64 = 5.0;

/// A chosen predecessor: node `from` (0 = parent, k = `cands[k - 1]`) of node `to`, with the name of the transformation when there is one.
pub(super) struct Pred {
    pub from: usize,
    pub to: usize,
    pub name: Option<String>,
}

fn add(a: &Counts, b: &Counts) -> Counts {
    let mut c = a.clone();
    for (e, n) in b {
        *c.entry(e.clone()).or_insert(0) += n;
    }
    c
}

fn equal(a: &Counts, b: &Counts) -> bool {
    a.keys()
        .chain(b.keys())
        .all(|e| a.get(e).copied().unwrap_or(0) == b.get(e).copied().unwrap_or(0))
}

/// `derive`: the name of the rule (or of the two rules) whose change of formula is `delta`; `None` for no change.
fn derive(delta: &Counts, rules: &Rules, two: bool) -> Option<String> {
    if delta.values().all(|&n| n == 0) {
        return None;
    }
    let r = &rules.rules;
    if let Some(x) = r.iter().find(|x| equal(&x.delta, delta)) {
        return Some(x.id.clone());
    }
    if !two {
        return None;
    }
    for (i, a) in r.iter().enumerate() {
        for b in &r[i..] {
            if equal(&add(&a.delta, &b.delta), delta) {
                return Some(format!("{} + {}", a.id, b.id));
            }
        }
    }
    None
}

fn minus(a: &Counts, b: &Counts) -> Counts {
    let mut c = a.clone();
    for (e, n) in b {
        *c.entry(e.clone()).or_insert(0) -= n;
    }
    c
}

/// Modified cosine and matched peaks of the MS2 of candidate `j` against that of `i`.
fn related(j: &Cand, i: &Cand) -> Option<f64> {
    let (mj, mi) = (j.ms2.as_ref()?, i.ms2.as_ref()?);
    let (sj, si) = (
        prepare(&mj.cons.mz, &mj.cons.rel, j.mz),
        prepare(&mi.cons.mz, &mi.cons.rel, i.mz),
    );
    let c = pair_components(&sj, j.mz, &si, i.mz);
    (c.n.iter().sum::<usize>() >= MIN_MATCHED && c.cos_total() >= COS_RELATED)
        .then(|| c.cos_total())
}

/// One predecessor per candidate that has one (`forms[0]` is the parent, `tmax[k]` the maximum of the profile of node `k`).
pub(super) fn predecessors(
    forms: &[Counts],
    cands: &[&Cand],
    tmax: &[Option<f64>],
    rules: &Rules,
) -> Vec<Pred> {
    let mut out = Vec::new();
    for i in 1..forms.len() {
        let mut best: Option<(usize, Option<String>)> = None;
        let mut best_val = -1.0;
        if let Some(n) = derive(&minus(&forms[i], &forms[0]), rules, true) {
            best = Some((0, Some(n)));
            best_val = 0.5;
        }
        for j in (1..forms.len()).filter(|&j| j != i) {
            let name = derive(&minus(&forms[i], &forms[j]), rules, false);
            let cos = related(cands[j - 1], cands[i - 1]);
            if name.is_none() && cos.is_none() {
                continue;
            }
            if let (Some(tj), Some(ti)) = (tmax[j], tmax[i]) {
                if tj > ti + STEP {
                    continue;
                }
            }
            let coh = match (tmax[j], tmax[i]) {
                (Some(tj), Some(ti)) if tj > ti => 0.5,
                _ => 1.0,
            };
            let val = cos.unwrap_or(0.0) + coh;
            if val > best_val {
                best = Some((j, name));
                best_val = val;
            }
        }
        if let Some((from, name)) = best {
            out.push(Pred { from, to: i, name });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules() -> Rules {
        Rules::parse(include_bytes!("../../../../TP_Mine/data/regole_tp.json")).unwrap()
    }

    fn c(f: &[(&str, i64)]) -> Counts {
        f.iter().map(|(e, n)| (e.to_string(), *n)).collect()
    }

    #[test]
    fn a_two_step_change_is_named_with_both_rules_and_no_change_is_nothing() {
        let r = rules();
        let one = derive(&c(&[("O", 1)]), &r, true);
        assert!(one.is_some_and(|n| !n.contains(" + ")));
        assert!(derive(&c(&[("O", 2)]), &r, true).is_some());
        assert_eq!(derive(&c(&[("C", 0)]), &r, true), None);
    }

    #[test]
    fn a_product_two_steps_from_the_parent_has_the_parent_as_predecessor() {
        let r = rules();
        let forms = vec![
            c(&[("C", 8), ("H", 10), ("N", 4), ("O", 2)]),
            c(&[("C", 8), ("H", 10), ("N", 4), ("O", 4)]),
        ];
        let cand = &[];
        let p = predecessors(&forms, cand, &[Some(0.0), Some(10.0)], &r);
        assert!(p.len() == 1 && p[0].from == 0 && p[0].to == 1);
    }
}
