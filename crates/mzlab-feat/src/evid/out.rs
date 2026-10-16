//! `evid/1` as JSON: the graph of the identities with their proofs (schema `docs/schemi/evid-1.json`). The bytes depend only on the graph: they are the
//! `evid` section of the `.mzfeat` and the base of the SHA-256 that compares two runs.

use super::graph::{Edge, Identity, Inputs, Node};
use super::prepared::{order as graph_order, Key, Prepared};
use super::{breakdown_fields, evidence_json, PointsTable};
use crate::store::Section;
use crate::table::*;
use mzlab_core::error::Result;
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

fn head_json(table: &PointsTable) -> String {
    let mut o = Out::default();
    o.raw("{\"formato\":").str(EVID_GRAPH_FORMAT);
    o.raw(",\"versione\":").int(i64::from(super::EVID_VERSION));
    o.raw(",\"versione_punti\":")
        .int(i64::from(table.version))
        .raw(",\"identita\":[");
    o.0
}

/// The document `evid/1`: identities in the order of the graph (the order of the candidates).
pub fn graph_json(ids: &[Identity], table: &PointsTable) -> String {
    let mut o = Out(head_json(table));
    list(&mut o, ids, identity_json);
    o.raw("]}");
    o.0
}

/// The same document written in pieces, so that it never exists as one string. Pass 1 makes every identity once for its place in the order and the length
/// of its JSON (an identity is a pure function of its row); pass 2, in `emit`, makes them again in order. Same bytes as `graph_json`, memory of one identity.
pub struct GraphStream<'a, 'b> {
    inp: &'b Inputs<'a>,
    prep: Prepared,
    order: Vec<u32>,
    head: String,
    len: usize,
}

impl<'a, 'b> GraphStream<'a, 'b> {
    pub fn new(inp: &'b Inputs<'a>) -> GraphStream<'a, 'b> {
        let prep = Prepared::new(inp);
        let n = inp.aligned.len();
        let head = head_json(inp.table);
        let (mut keys, mut body, mut o) = (Vec::with_capacity(n), 0usize, Out::default());
        for r in 0..n {
            let id = prep.identity(inp, r);
            keys.push(Key::of(&id));
            o.0.clear();
            identity_json(&mut o, &id);
            body += o.0.len();
        }
        let mut order: Vec<u32> = (0..n as u32).collect();
        crate::sortc::by(&mut order, |&a, &b| {
            graph_order(&keys[a as usize], &keys[b as usize])
        });
        let commas = n.saturating_sub(1);
        GraphStream {
            inp,
            prep,
            order,
            len: head.len() + body + commas + 2,
            head,
        }
    }

    /// Bytes this stream will give an estimate of what `new` holds: keys, order and the shared vectors of the rows.
    pub fn bytes_per_row() -> usize {
        std::mem::size_of::<Key>() + 4 + 48
    }
}

impl Section for GraphStream<'_, '_> {
    fn len(&self) -> usize {
        self.len
    }

    fn emit(&mut self, put: &mut dyn FnMut(&[u8]) -> Result<()>) -> Result<()> {
        const FLUSH: usize = 1 << 16;
        let mut o = Out(self.head.clone());
        for (i, &r) in self.order.iter().enumerate() {
            if i > 0 {
                o.raw(",");
            }
            identity_json(&mut o, &self.prep.identity(self.inp, r as usize));
            if o.0.len() >= FLUSH {
                put(o.0.as_bytes())?;
                o.0.clear();
            }
        }
        o.raw("]}");
        put(o.0.as_bytes())
    }
}
