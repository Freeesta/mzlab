//! The text of the key `rules` of `msn-graph/1` (schema `docs/schemi/msn-graph-1.json`) from an evaluation (`regole_valuta`): fixed key order, rounded numbers.

use super::rules::table;
use super::rules_evaluate::{valuta, Applicata, EdgeVal, Evaluation, RuleParams, VERSION};
use super::{Graph, Options};
use crate::json::{round, Out};

fn list(o: &mut Out, v: &[&str]) {
    o.raw("[");
    for (i, x) in v.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(x);
    }
    o.raw("]");
}

fn applied_json(o: &mut Out, r: &Applicata) {
    o.raw("{\"id\":")
        .str(r.id)
        .raw(",\"formula\":")
        .str(&r.formula)
        .raw(",\"mass\":")
        .num(round(r.massa, 5))
        .raw(",\"error_mda\":")
        .num(round(r.mda_error, 2))
        .raw(",\"radical\":")
        .raw(if r.radical { "true" } else { "false" })
        .raw(",\"priority\":")
        .int(r.priorita as i64)
        .raw(",\"energy\":")
        .str(r.energy)
        .raw(",\"why\":")
        .str(r.perche)
        .raw(",\"refs\":");
    list(o, &r.refs);
    o.raw("}");
}

fn edge_json(o: &mut Out, g: &Graph, i: usize, a: &EdgeVal) {
    o.raw("{\"from\":")
        .str(&format!("n{}", g.edges[i].from))
        .raw(",\"to\":")
        .str(&format!("n{}", g.edges[i].to))
        .raw(",\"rules\":[");
    for (j, r) in a.rules.iter().enumerate() {
        o.raw(if j > 0 { "," } else { "" });
        applied_json(o, r);
    }
    o.raw("],\"composite\":[");
    for (j, (x, y)) in a.composta.iter().enumerate() {
        o.raw(if j > 0 { "," } else { "" });
        list(o, &[x, y]);
    }
    o.raw("],\"stepped\":")
        .raw(if a.stepped { "true" } else { "false" })
        .raw(",\"warnings\":");
    list(o, &a.warnings);
    o.raw("}");
}

fn opts_text(o: &mut Out, v: Option<&str>) {
    match v {
        Some(x) => o.str(x),
        None => o.raw("null"),
    };
}

/// `fonti` (the references the rules of the arcs and the thresholds cite, by key, with their DOI and PMID) and `soglie` (the numeric thresholds of the table
/// with their origin and whether they are still to be calibrated): what the page shows next to a rule, so it does not carry the table.
fn sources_and_thresholds(o: &mut Out, v: &Evaluation) {
    let t = table();
    let mut keys: Vec<&str> = vec![];
    let citate = v
        .edges
        .iter()
        .flat_map(|a| a.rules.iter().flat_map(|r| r.refs.iter().copied()));
    let of_thresholds = t
        .globali
        .thresholds
        .iter()
        .flat_map(|s| s.refs.iter().map(|r| r.key.as_str()));
    for k in citate.chain(of_thresholds) {
        if !keys.contains(&k) {
            keys.push(k);
        }
    }
    o.raw(",\"sources\":[");
    let all = t
        .rules
        .iter()
        .flat_map(|r| r.refs.iter())
        .chain(t.globali.thresholds.iter().flat_map(|s| s.refs.iter()));
    for (i, k) in keys.iter().enumerate() {
        let Some(r) = all.clone().find(|r| r.key == *k) else {
            continue;
        };
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"key\":")
            .str(k)
            .raw(",\"doi\":");
        opts_text(o, r.doi.as_deref());
        o.raw(",\"pmid\":");
        opts_text(o, r.pmid.as_deref());
        o.raw("}");
    }
    o.raw("],\"thresholds\":[");
    for (i, s) in t.globali.thresholds.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"id\":")
            .str(&s.id)
            .raw(",\"value\":")
            .num(s.value)
            .raw(",\"why\":")
            .str(&s.perche)
            .raw(",\"origin\":");
        opts_text(o, s.origin.as_deref());
        o.raw(",\"to_calibrate\":")
            .raw(if s.to_calibrate { "true" } else { "false" })
            .raw(",\"refs\":");
        let ks: Vec<&str> = s.refs.iter().map(|r| r.key.as_str()).collect();
        list(o, &ks);
        o.raw("}");
    }
    o.raw("]");
}

/// The value of the key `rules` (see `docs/schemi/msn-graph-1.json`): keys in a fixed order, rounded numbers.
pub fn rules_json(g: &Graph, v: &Evaluation, p: &RuleParams) -> String {
    let mut o = Out::default();
    o.raw("{\"version\":")
        .int(VERSION as i64)
        .raw(",\"table\":\"fragrules/1\",\"cutoff_q\":")
        .num(p.cutoff_q)
        .raw(",\"loss_tolerance_mda\":")
        .num(p.toll_mda)
        .raw(",\"adduct\":");
    match v.addotto {
        Some(a) => o.str(a),
        None => o.raw("null"),
    };
    o.raw(",\"edges\":[");
    for (i, a) in v.edges.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" });
        edge_json(&mut o, g, i, a);
    }
    o.raw("],\"ions\":[");
    for (i, x) in v.ions.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"node\":")
            .str(&format!("n{}", x.nodo))
            .raw(",\"rule\":")
            .str(x.regola)
            .raw(",\"error_mda\":")
            .num(round(x.errore_mda, 2))
            .raw("}");
    }
    o.raw("],\"unobservable\":[");
    for (i, x) in v.non_osservabili.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"node\":")
            .str(&format!("n{}", x.node))
            .raw(",\"rule\":")
            .str(x.rule)
            .raw(",\"mz\":")
            .num(round(x.mz, 4))
            .raw(",\"threshold\":")
            .num(round(x.threshold, 4))
            .raw("}");
    }
    o.raw("],\"counts\":{\"explained_edges\":")
        .int(v.explained as i64)
        .raw(",\"edges_without_rule\":")
        .int(v.without_rule as i64)
        .raw(",\"explained_intensity\":")
        .num(round(v.explained_intensity, 3))
        .raw("}");
    sources_and_thresholds(&mut o, v);
    o.raw("}");
    o.0
}

/// `grafo` (the text of an `msn-graph/1`) with the key `rules` added at the end; the text of a graph without it does not change.
pub fn with_rules(graph: &str, g: &Graph, o: &Options, p: &RuleParams) -> String {
    super::view::with_key(graph, "rules", &rules_json(g, &valuta(g, o, p), p))
}
