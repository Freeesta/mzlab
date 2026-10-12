//! `evid/1` as JSON: the graph of the identities with their proofs (schema `docs/schemi/evid-1.json`). The bytes depend only on the graph: they are the
//! `evid` section of the `.mzfeat` and the base of the SHA-256 that compares two runs.

use super::graph::{Edge, Identity, Node};
use super::{breakdown_fields, evidence_json, PointsTable};
use crate::table::*;
use mzlab_core::json::{round, Out};

pub const EVID_GRAPH_FORMAT: &str = super::FORMAT;

fn role_name(role: u8) -> &'static str {
    match role {
        ROLE_M => "M",
        ROLE_ISOTOPE => "isotopo",
        ROLE_ADDUCT => "addotto",
        ROLE_ISF => "isf",
        ROLE_DIMER => "dimero",
        ROLE_UNDETERMINED => "non determinato",
        _ => "",
    }
}

fn node_json(o: &mut Out, n: &Node) {
    match n {
        Node::Identity(i) => o
            .raw("{\"tipo\":\"identita\",\"id\":")
            .int(i64::from(*i))
            .raw("}"),
        Node::Member(i, k) => o
            .raw("{\"tipo\":\"membro\",\"id\":")
            .int(i64::from(*i))
            .raw(",\"indice\":")
            .int(i64::from(*k))
            .raw("}"),
        Node::Library(s) => o.raw("{\"tipo\":\"libreria\",\"nome\":").str(s).raw("}"),
        Node::External(s) => o.raw("{\"tipo\":\"esterno\",\"nome\":").str(s).raw("}"),
    };
}

fn edge_json(o: &mut Out, e: &Edge) {
    o.raw("{\"tipo\":").str(e.kind.name()).raw(",\"da\":");
    node_json(o, &e.from);
    o.raw(",\"a\":");
    node_json(o, &e.to);
    o.raw(",\"etichetta\":").str(&e.label).raw(",\"prove\":[");
    for (i, p) in e.proofs.iter().enumerate() {
        if i > 0 {
            o.raw(",");
        }
        evidence_json(o, p);
    }
    o.raw("]}");
}

fn list<T>(o: &mut Out, items: &[T], mut each: impl FnMut(&mut Out, &T)) {
    for (i, x) in items.iter().enumerate() {
        if i > 0 {
            o.raw(",");
        }
        each(o, x);
    }
}

fn identity_json(o: &mut Out, x: &Identity) {
    o.raw("{\"ipotesi\":");
    o.opt(x.hypothesis.map(f64::from));
    o.raw(",\"massa_neutra\":").num(round(x.neutral, 6));
    if let Some(f) = &x.formula {
        o.raw(",\"formula\":").str(f);
    }
    o.raw(",\"livello_max\":")
        .str(x.max_level)
        .raw(",\"membri\":[");
    list(o, &x.members, |o, m| {
        o.raw("{\"mz\":")
            .num(round(m.mz, 6))
            .raw(",\"rt\":")
            .num(round(m.rt, 4));
        o.raw(",\"ruolo\":").str(role_name(m.role)).raw("}");
    });
    o.raw("],");
    breakdown_fields(o, x.id, &x.breakdown);
    o.raw(",\"archi\":[");
    list(o, &x.edges, edge_json);
    o.raw("]}");
}

/// The document `evid/1`: identities in the order of the graph (the order of the candidates).
pub fn graph_json(ids: &[Identity], table: &PointsTable) -> String {
    let mut o = Out::default();
    o.raw("{\"formato\":").str(EVID_GRAPH_FORMAT);
    o.raw(",\"versione\":").int(i64::from(super::EVID_VERSION));
    o.raw(",\"versione_punti\":")
        .int(i64::from(table.version))
        .raw(",\"identita\":[");
    list(&mut o, ids, identity_json);
    o.raw("]}");
    o.0
}
