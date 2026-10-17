//! The network of mass differences of the Dig (`hr/network.py::mass_network`): an edge from one ion to another for every rule whose change of formula takes
//! the formula of the first to the formula of the second (one step); of two ions the edge goes from the one that peaks first. The MS2 of the edge (shifted
//! fragments) is that of the parent against the product; the precursors of two steps or related by MS2 are not drawn.

use crate::sortc::SortC;
use std::collections::BTreeMap;

use mzlab_core::elements::{mass, parse_formula};
use mzlab_core::json::{round, Out};

use super::dig::{Cand, Dig};
use super::network_pred::{predecessors, related};
use crate::hypo::{Counts, Rules};

#[cfg(test)]
mod golden;

/// The most edges of the network that the page receives; the strongest (with shifted fragments) first.
const MAX_EDGES: usize = 1500;
/// Sampling step of the kinetics (min): a source may peak this much after its product and still be consistent.
const STEP: f64 = 5.0;

struct Edge {
    from: usize,
    to: usize,
    rule: usize,
    kinetic: &'static str,
    /// «mass» (one rule), «both» (also the predecessor of the candidate) or «network» (a predecessor without a single mass difference, named by `name`).
    kind: &'static str,
    name: Option<String>,
}

fn counts(f: &str) -> Counts {
    let mut c = Counts::new();
    for (e, n) in parse_formula(f).unwrap_or_default() {
        *c.entry(e).or_insert(0) += n as i64;
    }
    c
}

fn same_change(a: &Counts, b: &Counts, delta: &Counts) -> bool {
    a.keys().chain(b.keys()).chain(delta.keys()).all(|e| {
        let g = |c: &Counts| c.get(e).copied().unwrap_or(0);
        g(b) - g(a) == g(delta)
    })
}

fn neutral_mass(c: &Counts) -> f64 {
    mass(
        &c.iter()
            .map(|(e, n)| (e.clone(), *n as u64))
            .collect::<Vec<_>>(),
    )
}

fn shifted(c: &Cand) -> Option<(usize, f64)> {
    let s = &c.ms2.as_ref()?.sim;
    Some((s.n[1] + s.n[2], s.cos[1] + s.cos[2]))
}

/// Best edge of every pair of ions (kinetics, then from the parent) over all the rules. Node 0 is the parent.
fn best_edges(forms: &[Counts], tmax: &[Option<f64>], rules: &Rules) -> Vec<Edge> {
    let mut best: BTreeMap<(usize, usize), ((u8, u8), Edge)> = BTreeMap::new();
    for a in 0..forms.len() {
        for (ri, r) in rules.rules.iter().enumerate() {
            for b in
                (1..forms.len()).filter(|&b| b != a && same_change(&forms[a], &forms[b], &r.delta))
            {
                let kinetic = match (tmax[a], tmax[b]) {
                    (Some(ta), Some(tb)) if ta <= tb + STEP => "consistent",
                    (Some(_), Some(_)) => "against",
                    _ => "n/a",
                };
                let rank = (
                    match kinetic {
                        "consistent" => 2,
                        "n/a" => 1,
                        _ => 0,
                    },
                    u8::from(a == 0),
                );
                let key = (a.min(b), a.max(b));
                if best.get(&key).is_none_or(|e| rank > e.0) {
                    let edge = Edge {
                        from: a,
                        to: b,
                        rule: ri,
                        kinetic,
                        kind: "mass",
                        name: None,
                    };
                    best.insert(key, (rank, edge));
                }
            }
        }
    }
    best.into_values().map(|e| e.1).collect()
}

/// The predecessors of the candidates: the edge of a predecessor that is also a mass difference becomes «both», the others are added as «network».
fn mark_predecessors(
    edges: &mut Vec<Edge>,
    forms: &[Counts],
    cands: &[&Cand],
    tmax: &[Option<f64>],
    rules: &Rules,
) {
    let cos = |j: usize, i: usize| related(cands[j - 1], cands[i - 1]);
    for p in predecessors(forms, tmax, rules, &cos) {
        if let Some(e) = edges.iter_mut().find(|e| (e.from, e.to) == (p.from, p.to)) {
            e.kind = "both";
        } else if !edges.iter().any(|e| (e.from, e.to) == (p.to, p.from)) {
            edges.push(Edge {
                from: p.from,
                to: p.to,
                rule: 0,
                kinetic: "consistent",
                kind: "network",
                name: Some(p.name.unwrap_or_else(|| "MS2 related".into())),
            });
        }
    }
}

/// The nodes that an edge touches: the parent, then the candidates (`cands[k - 1]` is node `k`).
fn write_nodes(o: &mut Out, d: &Dig, cands: &[&Cand], edges: &[Edge]) {
    o.raw("{\"nodes\":[{\"id\":\"progenitore\",\"label\":\"parent\",\"formula\":")
        .str(&d.parent.formula())
        .raw(",\"mz\":null,\"rt\":null,\"score\":null,\"level\":null}");
    let used = |k: usize| edges.iter().any(|e| e.from == k || e.to == k);
    for (_, c) in cands.iter().enumerate().filter(|(k, _)| used(k + 1)) {
        let f = c.formula.as_deref().unwrap_or("");
        let label = c
            .hyp
            .as_ref()
            .map(|h| h.label.as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or(f);
        o.raw(",{\"id\":").int(c.id()).raw(",\"label\":").str(label);
        o.raw(",\"formula\":")
            .str(f)
            .raw(",\"mz\":")
            .num(round(c.mz, 4));
        o.raw(",\"rt\":")
            .num(round(c.rt, 2))
            .raw(",\"score\":")
            .num(round(c.score, 1));
        o.raw(",\"level\":").int(i64::from(c.level())).raw("}");
    }
}

fn edge_share(e: &Edge, cands: &[&Cand]) -> Option<(usize, f64)> {
    if e.from == 0 && e.kind != "network" {
        shifted(cands[e.to - 1])
    } else {
        None
    }
}

/// The edges of the network, as the page reads them.
fn write_edges(o: &mut Out, edges: &[Edge], cands: &[&Cand], forms: &[Counts], rules: &Rules) {
    let id = |o: &mut Out, k: usize| {
        if k == 0 {
            o.str("progenitore");
        } else {
            o.int(cands[k - 1].id());
        }
    };
    let share = |e: &Edge| edge_share(e, cands);
    for (i, e) in edges.iter().enumerate() {
        let r = &rules.rules[e.rule];
        let name = e.name.as_deref().unwrap_or(&r.id);
        o.raw(if i > 0 { ",{\"from\":" } else { "{\"from\":" });
        id(o, e.from);
        o.raw(",\"to\":");
        id(o, e.to);
        o.raw(",\"rule\":")
            .str(name)
            .raw(",\"kinetic\":")
            .str(e.kinetic);
        o.raw(",\"dm\":").num(round(
            neutral_mass(&forms[e.to]) - neutral_mass(&forms[e.from]),
            5,
        ));
        if e.kind == "network" {
            o.raw(",\"scope\":null");
        } else {
            o.raw(",\"scope\":").str(match r.level {
                2 => "literature",
                3 => "class",
                _ => "generic",
            });
        }
        o.raw(",\"kind\":").str(e.kind);
        if let Some((n, s)) = share(e) {
            o.raw(",\"shifted\":{\"n\":")
                .int(n as i64)
                .raw(",\"share\":")
                .num(round(s, 3))
                .raw("}");
        }
        o.raw("}");
    }
}

/// The `mass_network` of the summary: `{nodes, edges, n_edges}`; `null` when there are no rules or no edge.
pub fn mass_network(d: &Dig, rules: &Rules) -> String {
    let pf = d.parent.formula();
    let cands: Vec<&Cand> = d
        .cands
        .iter()
        .filter(|c| c.formula.as_deref().is_some_and(|f| f != pf))
        .collect();
    let mut forms = vec![counts(&pf)];
    forms.extend(
        cands
            .iter()
            .map(|c| counts(c.formula.as_deref().unwrap_or(""))),
    );
    let tmax: Vec<Option<f64>> = std::iter::once(Some(0.0))
        .chain(cands.iter().map(|c| c.kin.as_ref().map(|k| k.0.tmax)))
        .collect();
    let mut edges = best_edges(&forms, &tmax, rules);
    mark_predecessors(&mut edges, &forms, &cands, &tmax, rules);
    let total = edges.len();
    if total == 0 {
        return "null".into();
    }
    let share = |e: &Edge| edge_share(e, &cands);
    if total > MAX_EDGES {
        edges.sortc_by(|a, b| {
            let s = |e: &Edge| share(e).map_or(0.0, |x| x.1);
            (a.kind == "mass")
                .cmp(&(b.kind == "mass"))
                .then(s(b).total_cmp(&s(a)))
        });
        edges.truncate(MAX_EDGES);
    }
    let mut o = Out::default();
    write_nodes(&mut o, d, &cands, &edges);
    o.raw("],\"edges\":[");
    write_edges(&mut o, &edges, &cands, &forms, rules);
    o.raw("],\"n_edges\":").int(total as i64).raw("}");
    o.0
}
