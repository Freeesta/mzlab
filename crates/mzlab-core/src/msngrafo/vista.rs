//! The key `vista` of `msngrafo/1`: what a drawer needs and the graph does not say outright, computed ONLY from the fields the JSON already has
//! (so an old graph is completed by `completa` with the same bytes): the MS levels of each ion, its intensity in the spectrum of its parent on
//! the best path, the best path itself (`best-path/1`), the number of precursors, the tree layout (barycentre plus swaps against crossings)
//! and the radial layout (ring, angle). Coordinates are abstract numbers: pixels and styles belong to the drawer. A proposal of a rule
//! stated in the JSON, never a mechanism.

use super::posa::{albero, Network};
use super::posa_radiale::radiale;
use crate::error::{Error, Result};
use crate::json::{parse, round, Out, Value};
use std::collections::BTreeMap;

/// The rule of the best path as written in the JSON (`vista.regola`).
const RULE: &str = "Fewest steps from a root; ties: the highest smallest relative intensity along the path (the most intense measure of each arc), then more isolated arcs, then the smaller precursor id";

pub(super) struct EdgeD {
    pub(super) da: usize,
    pub(super) a: usize,
    pub(super) isolato: bool,
    /// The most intense measure of the arc, % of the base peak of its spectrum.
    pub(super) rel: f64,
    /// MS levels of the measures in which the fragment really is (`rel` > 0).
    pub(super) levels: Vec<u8>,
}

pub(super) struct Data {
    pub(super) id: Vec<String>,
    pub(super) mz: Vec<f64>,
    pub(super) level: Vec<u8>,
    pub(super) rel_max: Vec<f64>,
    pub(super) edges: Vec<EdgeD>,
}

fn bad() -> Error {
    Error::new("err.msngrafo.badGraph")
}

fn num(v: &Value, k: &str) -> Result<f64> {
    v.get(k).and_then(|x| x.f64()).ok_or_else(bad)
}

pub(super) fn read(v: &Value) -> Result<Data> {
    super::version(v)?;
    let nodes = v.get("nodi").ok_or_else(bad)?.arr();
    let mut d = Data {
        id: vec![],
        mz: vec![],
        level: vec![],
        rel_max: vec![],
        edges: vec![],
    };
    let mut index: BTreeMap<&str, usize> = BTreeMap::new();
    for (i, n) in nodes.iter().enumerate() {
        let id = n.get("id").and_then(|x| x.str()).ok_or_else(bad)?;
        index.insert(id, i);
        d.id.push(id.to_string());
        d.mz.push(num(n, "mz")?);
        d.level.push((num(n, "livello")? as u8).max(1));
        d.rel_max.push(num(n, "rel_max")?);
    }
    for a in v.get("archi").ok_or_else(bad)?.arr() {
        let node = |k: &str| {
            a.get(k)
                .and_then(|x| x.str())
                .and_then(|s| index.get(s).copied())
                .ok_or_else(bad)
        };
        let measures = a.get("misure").map_or(&[][..], |m| m.arr());
        let mut livelli: Vec<u8> = vec![];
        let mut rel = 0.0f64;
        for m in measures {
            let r = num(m, "rel")?;
            rel = rel.max(r);
            if r > 0.0 {
                livelli.push(num(m, "livello")? as u8);
            }
        }
        d.edges.push(EdgeD {
            da: node("da")?,
            a: node("a")?,
            isolato: a.get("isolamento").and_then(|x| x.bool()).unwrap_or(false),
            rel,
            levels: livelli,
        });
    }
    Ok(d)
}

/// The best entering arc of a node and what the path to it costs: (steps, smallest relative intensity on the path, isolated arcs).
#[derive(Clone, Copy)]
pub(super) struct Best {
    pub(super) da: Option<usize>,
    pub(super) steps: usize,
    pub(super) min_rel: Option<f64>,
    pub(super) isolati: usize,
}

/// Whether the path `x` beats `y`: fewer steps, then the higher smallest intensity, then more isolated arcs, then the smaller precursor.
fn better(x: &Best, y: &Best) -> bool {
    let rel = |b: &Best| b.min_rel.unwrap_or(f64::INFINITY);
    x.steps
        .cmp(&y.steps)
        .then(rel(y).total_cmp(&rel(x)))
        .then(y.isolati.cmp(&x.isolati))
        .then(x.da.cmp(&y.da))
        .is_lt()
}

/// The nodes in topological order, the smallest index first among the ready ones; nodes in a cycle (none in a graph of the engine) come last.
fn topologico(n: usize, edges: &[EdgeD]) -> Vec<usize> {
    let mut entranti = vec![0usize; n];
    let mut giu: Vec<Vec<usize>> = vec![vec![]; n];
    for a in edges {
        entranti[a.a] += 1;
        giu[a.da].push(a.a);
    }
    let mut ready: Vec<usize> = (0..n).filter(|&i| entranti[i] == 0).collect();
    let mut out = vec![];
    while !ready.is_empty() {
        let k = (0..ready.len()).min_by_key(|&k| ready[k]).unwrap_or(0);
        let i = ready.swap_remove(k);
        out.push(i);
        for &j in &giu[i] {
            entranti[j] -= 1;
            if entranti[j] == 0 {
                ready.push(j);
            }
        }
    }
    let seen: Vec<bool> = (0..n).map(|i| out.contains(&i)).collect();
    out.extend((0..n).filter(|&i| !seen[i]));
    out
}

/// `best-path/1`: dynamic programming on the DAG in topological order.
pub(super) fn paths(d: &Data) -> Vec<Best> {
    let n = d.id.len();
    let root = Best {
        da: None,
        steps: 0,
        min_rel: None,
        isolati: 0,
    };
    let mut best = vec![root; n];
    let mut done = vec![false; n];
    let mut su: Vec<Vec<&EdgeD>> = vec![vec![]; n];
    for a in &d.edges {
        su[a.a].push(a);
    }
    for i in topologico(n, &d.edges) {
        let mut chosen: Option<Best> = None;
        for a in &su[i] {
            if !done[a.da] {
                continue;
            }
            let p = best[a.da];
            let c = Best {
                da: Some(a.da),
                steps: p.steps + 1,
                min_rel: Some(p.min_rel.map_or(a.rel, |m| m.min(a.rel))),
                isolati: p.isolati + a.isolato as usize,
            };
            if chosen.as_ref().is_none_or(|s| better(&c, s)) {
                chosen = Some(c);
            }
        }
        best[i] = chosen.unwrap_or(root);
        done[i] = true;
    }
    best
}

fn id(o: &mut Out, d: &Data, i: Option<usize>) {
    match i {
        Some(i) => o.str(&d.id[i]),
        None => o.raw("null"),
    };
}

fn opt(o: &mut Out, v: Option<f64>) {
    match v {
        Some(x) => o.num(round(x, 1)),
        None => o.raw("null"),
    };
}

/// One element of `vista.nodi`: node `i` with its best path, its tree position (column, row) and its radial place (ring, angle).
fn node_json(
    o: &mut Out,
    d: &Data,
    i: usize,
    (b, pos, rad): (&Best, (usize, usize), (usize, f64)),
) {
    let b = *b;
    let entranti: Vec<&EdgeD> = d.edges.iter().filter(|a| a.a == i).collect();
    let mut da: Vec<usize> = entranti.iter().map(|a| a.da).collect();
    crate::sortc::any_by(&mut da, |a, b| a.cmp(b));
    da.dedup();
    let mut levels: Vec<u8> = entranti.iter().flat_map(|a| a.levels.clone()).collect();
    if b.da.is_none() && d.level[i] == 1 {
        levels = vec![1];
    }
    crate::sortc::any_by(&mut levels, |a, b| a.cmp(b));
    levels.dedup();
    if levels.is_empty() {
        levels.push(d.level[i]);
    }
    let rel = match b.da {
        Some(p) => d
            .edges
            .iter()
            .find(|a| a.da == p && a.a == i)
            .map(|a| a.rel),
        None => Some(d.rel_max[i]).filter(|r| *r > 0.0),
    };
    o.raw(if i > 0 { ",{\"id\":" } else { "{\"id\":" })
        .str(&d.id[i])
        .raw(",\"livelli\":[");
    for (k, l) in levels.iter().enumerate() {
        o.raw(if k > 0 { "," } else { "" }).int(*l as i64);
    }
    o.raw("],\"rel\":");
    opt(o, rel);
    o.raw(",\"rel_max\":");
    opt(o, Some(d.rel_max[i]));
    o.raw(",\"best\":{\"da\":");
    id(o, d, b.da);
    o.raw(",\"passi\":").int(b.steps as i64).raw(",\"costo\":[");
    o.int(b.steps as i64).raw(",");
    opt(o, b.min_rel);
    o.raw(",").int(b.isolati as i64).raw("]},\"entranti\":");
    o.int(da.len() as i64).raw(",\"comune\":");
    o.raw(if da.len() >= 2 { "true" } else { "false" });
    o.raw(",\"albero\":{\"col\":")
        .int(pos.0 as i64)
        .raw(",\"riga\":")
        .int(pos.1 as i64)
        .raw("},\"radiale\":{\"anello\":")
        .int(rad.0 as i64)
        .raw(",\"angolo\":")
        .num(round(rad.1, 2))
        .raw("}}");
}

/// The value of the key `vista` of the graph `d` (its keys in a fixed order).
fn view(d: &Data) -> String {
    let n = d.id.len();
    let best = paths(d);
    let pairs: Vec<(usize, usize)> = d.edges.iter().map(|a| (a.da, a.a)).collect();
    let rete = Network {
        level: &d.level,
        mz: &d.mz,
        edges: &pairs,
    };
    let pos = albero(&rete);
    let parent: Vec<Option<usize>> = best.iter().map(|b| b.da).collect();
    let steps: Vec<usize> = best.iter().map(|b| b.steps).collect();
    let rad = radiale(&parent, &steps, &d.mz);
    let mut order: Vec<usize> = (0..n).collect();
    crate::sortc::any_by(&mut order, |a, b| pos[*a].cmp(&pos[*b]).then(a.cmp(b)));
    let mut o = Out::default();
    o.raw("{\"versione\":1,\"algoritmo\":\"best-path/1\",\"regola\":")
        .str(RULE)
        .raw(",\"ordine\":[");
    for (k, &i) in order.iter().enumerate() {
        o.raw(if k > 0 { "," } else { "" }).str(&d.id[i]);
    }
    o.raw("],\"nodi\":[");
    for i in 0..n {
        node_json(&mut o, d, i, (&best[i], pos[i], rad[i]));
    }
    o.raw("],\"archi\":[");
    for (k, a) in d.edges.iter().enumerate() {
        o.raw(if k > 0 { "," } else { "" }).raw("{\"da\":");
        id(&mut o, d, Some(a.da));
        o.raw(",\"a\":");
        id(&mut o, d, Some(a.a));
        o.raw(",\"best\":");
        o.raw(if best[a.a].da == Some(a.da) {
            "true}"
        } else {
            "false}"
        });
    }
    o.raw("]}");
    o.0
}

/// `completa` for the text the engine just wrote (always a valid graph).
pub(super) fn with_view(json: String) -> String {
    completa(&json).unwrap_or(json)
}

/// The text of a graph with the key `vista` added at its end; `Err` when it is not an `msngrafo/1` or `/2` (`error.newerFormat` for a newer one). A graph that has `vista` is returned as it is.
pub fn completa(json: &str) -> Result<String> {
    let v = parse(json.as_bytes()).map_err(|_| bad())?;
    if v.get("vista").is_some() {
        read(&v)?;
        return Ok(json.to_string());
    }
    let testa = json.trim_end();
    let testa = testa.strip_suffix('}').ok_or_else(bad)?;
    Ok(format!("{testa},\"vista\":{}}}", view(&read(&v)?)))
}

/// `grafo` with the key `chiave` (its value is `valore`) added after the keys that are there, and before `vista`, which stays the last.
pub(super) fn with_key(graph: &str, key: &str, value: &str) -> String {
    let (testa, tail) = match graph.rfind(",\"vista\":{") {
        Some(p) => graph.split_at(p),
        None => (graph.strip_suffix('}').unwrap_or(graph), "}"),
    };
    format!("{testa},\"{key}\":{value}{tail}")
}
