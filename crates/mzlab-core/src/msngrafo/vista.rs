//! The key `vista` of `msngrafo/1`: what a drawer needs and the graph does not say outright, computed ONLY from the fields the JSON already has
//! (so an old graph is completed by `completa` with the same bytes): the MS levels of each ion, its intensity in the spectrum of its parent on
//! the best path, the best path itself (`best-path/1`), the number of precursors, the tree layout (barycentre plus swaps against crossings)
//! and the radial layout (ring, angle). Coordinates are abstract numbers: pixels and styles belong to the drawer. A proposal of a rule
//! stated in the JSON, never a mechanism.

use super::posa::{albero, Rete};
use super::posa_radiale::radiale;
use crate::error::{Error, Result};
use crate::json::{parse, round, Out, Value};
use std::collections::BTreeMap;

/// The rule of the best path as written in the JSON (`vista.regola`).
const REGOLA: &str = "Fewest steps from a root; ties: the highest smallest relative intensity along the path (the most intense measure of each arc), then more isolated arcs, then the smaller precursor id";

pub(super) struct ArcoD {
    pub(super) da: usize,
    pub(super) a: usize,
    pub(super) isolato: bool,
    /// The most intense measure of the arc, % of the base peak of its spectrum.
    pub(super) rel: f64,
    /// MS levels of the measures in which the fragment really is (`rel` > 0).
    pub(super) livelli: Vec<u8>,
}

pub(super) struct Dati {
    pub(super) id: Vec<String>,
    pub(super) mz: Vec<f64>,
    pub(super) livello: Vec<u8>,
    pub(super) rel_max: Vec<f64>,
    pub(super) archi: Vec<ArcoD>,
}

fn bad() -> Error {
    Error::new("err.msngrafo.badGraph")
}

fn num(v: &Value, k: &str) -> Result<f64> {
    v.get(k).and_then(|x| x.f64()).ok_or_else(bad)
}

pub(super) fn leggi(v: &Value) -> Result<Dati> {
    if v.get("formato").and_then(|f| f.str()) != Some("msngrafo/1") {
        return Err(bad());
    }
    let nodi = v.get("nodi").ok_or_else(bad)?.arr();
    let mut d = Dati {
        id: vec![],
        mz: vec![],
        livello: vec![],
        rel_max: vec![],
        archi: vec![],
    };
    let mut indice: BTreeMap<&str, usize> = BTreeMap::new();
    for (i, n) in nodi.iter().enumerate() {
        let id = n.get("id").and_then(|x| x.str()).ok_or_else(bad)?;
        indice.insert(id, i);
        d.id.push(id.to_string());
        d.mz.push(num(n, "mz")?);
        d.livello.push((num(n, "livello")? as u8).max(1));
        d.rel_max.push(num(n, "rel_max")?);
    }
    for a in v.get("archi").ok_or_else(bad)?.arr() {
        let nodo = |k: &str| {
            a.get(k)
                .and_then(|x| x.str())
                .and_then(|s| indice.get(s).copied())
                .ok_or_else(bad)
        };
        let misure = a.get("misure").map_or(&[][..], |m| m.arr());
        let mut livelli: Vec<u8> = vec![];
        let mut rel = 0.0f64;
        for m in misure {
            let r = num(m, "rel")?;
            rel = rel.max(r);
            if r > 0.0 {
                livelli.push(num(m, "livello")? as u8);
            }
        }
        d.archi.push(ArcoD {
            da: nodo("da")?,
            a: nodo("a")?,
            isolato: a.get("isolamento").and_then(|x| x.bool()).unwrap_or(false),
            rel,
            livelli,
        });
    }
    Ok(d)
}

/// The best entering arc of a node and what the path to it costs: (steps, smallest relative intensity on the path, isolated arcs).
#[derive(Clone, Copy)]
pub(super) struct Best {
    pub(super) da: Option<usize>,
    pub(super) passi: usize,
    pub(super) min_rel: Option<f64>,
    pub(super) isolati: usize,
}

/// Whether the path `x` beats `y`: fewer steps, then the higher smallest intensity, then more isolated arcs, then the smaller precursor.
fn meglio(x: &Best, y: &Best) -> bool {
    let rel = |b: &Best| b.min_rel.unwrap_or(f64::INFINITY);
    x.passi
        .cmp(&y.passi)
        .then(rel(y).total_cmp(&rel(x)))
        .then(y.isolati.cmp(&x.isolati))
        .then(x.da.cmp(&y.da))
        .is_lt()
}

/// The nodes in topological order, the smallest index first among the ready ones; nodes in a cycle (none in a graph of the engine) come last.
fn topologico(n: usize, archi: &[ArcoD]) -> Vec<usize> {
    let mut entranti = vec![0usize; n];
    let mut giu: Vec<Vec<usize>> = vec![vec![]; n];
    for a in archi {
        entranti[a.a] += 1;
        giu[a.da].push(a.a);
    }
    let mut pronti: Vec<usize> = (0..n).filter(|&i| entranti[i] == 0).collect();
    let mut out = vec![];
    while !pronti.is_empty() {
        let k = (0..pronti.len()).min_by_key(|&k| pronti[k]).unwrap_or(0);
        let i = pronti.swap_remove(k);
        out.push(i);
        for &j in &giu[i] {
            entranti[j] -= 1;
            if entranti[j] == 0 {
                pronti.push(j);
            }
        }
    }
    let visti: Vec<bool> = (0..n).map(|i| out.contains(&i)).collect();
    out.extend((0..n).filter(|&i| !visti[i]));
    out
}

/// `best-path/1`: dynamic programming on the DAG in topological order.
pub(super) fn percorsi(d: &Dati) -> Vec<Best> {
    let n = d.id.len();
    let radice = Best {
        da: None,
        passi: 0,
        min_rel: None,
        isolati: 0,
    };
    let mut best = vec![radice; n];
    let mut fatto = vec![false; n];
    let mut su: Vec<Vec<&ArcoD>> = vec![vec![]; n];
    for a in &d.archi {
        su[a.a].push(a);
    }
    for i in topologico(n, &d.archi) {
        let mut scelto: Option<Best> = None;
        for a in &su[i] {
            if !fatto[a.da] {
                continue;
            }
            let p = best[a.da];
            let c = Best {
                da: Some(a.da),
                passi: p.passi + 1,
                min_rel: Some(p.min_rel.map_or(a.rel, |m| m.min(a.rel))),
                isolati: p.isolati + a.isolato as usize,
            };
            if scelto.as_ref().is_none_or(|s| meglio(&c, s)) {
                scelto = Some(c);
            }
        }
        best[i] = scelto.unwrap_or(radice);
        fatto[i] = true;
    }
    best
}

fn id(o: &mut Out, d: &Dati, i: Option<usize>) {
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
fn nodo_json(
    o: &mut Out,
    d: &Dati,
    i: usize,
    (b, pos, rad): (&Best, (usize, usize), (usize, f64)),
) {
    let b = *b;
    let entranti: Vec<&ArcoD> = d.archi.iter().filter(|a| a.a == i).collect();
    let mut da: Vec<usize> = entranti.iter().map(|a| a.da).collect();
    crate::sortc::any_by(&mut da, |a, b| a.cmp(b));
    da.dedup();
    let mut livelli: Vec<u8> = entranti.iter().flat_map(|a| a.livelli.clone()).collect();
    if b.da.is_none() && d.livello[i] == 1 {
        livelli = vec![1];
    }
    crate::sortc::any_by(&mut livelli, |a, b| a.cmp(b));
    livelli.dedup();
    if livelli.is_empty() {
        livelli.push(d.livello[i]);
    }
    let rel = match b.da {
        Some(p) => d
            .archi
            .iter()
            .find(|a| a.da == p && a.a == i)
            .map(|a| a.rel),
        None => Some(d.rel_max[i]).filter(|r| *r > 0.0),
    };
    o.raw(if i > 0 { ",{\"id\":" } else { "{\"id\":" })
        .str(&d.id[i])
        .raw(",\"livelli\":[");
    for (k, l) in livelli.iter().enumerate() {
        o.raw(if k > 0 { "," } else { "" }).int(*l as i64);
    }
    o.raw("],\"rel\":");
    opt(o, rel);
    o.raw(",\"rel_max\":");
    opt(o, Some(d.rel_max[i]));
    o.raw(",\"best\":{\"da\":");
    id(o, d, b.da);
    o.raw(",\"passi\":").int(b.passi as i64).raw(",\"costo\":[");
    o.int(b.passi as i64).raw(",");
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
fn vista(d: &Dati) -> String {
    let n = d.id.len();
    let best = percorsi(d);
    let pairs: Vec<(usize, usize)> = d.archi.iter().map(|a| (a.da, a.a)).collect();
    let rete = Rete {
        livello: &d.livello,
        mz: &d.mz,
        archi: &pairs,
    };
    let pos = albero(&rete);
    let padre: Vec<Option<usize>> = best.iter().map(|b| b.da).collect();
    let passi: Vec<usize> = best.iter().map(|b| b.passi).collect();
    let rad = radiale(&padre, &passi, &d.mz);
    let mut ordine: Vec<usize> = (0..n).collect();
    crate::sortc::any_by(&mut ordine, |a, b| pos[*a].cmp(&pos[*b]).then(a.cmp(b)));
    let mut o = Out::default();
    o.raw("{\"versione\":1,\"algoritmo\":\"best-path/1\",\"regola\":")
        .str(REGOLA)
        .raw(",\"ordine\":[");
    for (k, &i) in ordine.iter().enumerate() {
        o.raw(if k > 0 { "," } else { "" }).str(&d.id[i]);
    }
    o.raw("],\"nodi\":[");
    for i in 0..n {
        nodo_json(&mut o, d, i, (&best[i], pos[i], rad[i]));
    }
    o.raw("],\"archi\":[");
    for (k, a) in d.archi.iter().enumerate() {
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
pub(super) fn con_vista(json: String) -> String {
    completa(&json).unwrap_or(json)
}

/// The text of a graph with the key `vista` added at its end; `Err` when it is not an `msngrafo/1`. A graph that has `vista` is returned as it is.
pub fn completa(json: &str) -> Result<String> {
    let v = parse(json.as_bytes()).map_err(|_| bad())?;
    if v.get("vista").is_some() {
        leggi(&v)?;
        return Ok(json.to_string());
    }
    let testa = json.trim_end();
    let testa = testa.strip_suffix('}').ok_or_else(bad)?;
    Ok(format!("{testa},\"vista\":{}}}", vista(&leggi(&v)?)))
}

/// `grafo` with the key `chiave` (its value is `valore`) added after the keys that are there, and before `vista`, which stays the last.
pub(super) fn con_chiave(grafo: &str, chiave: &str, valore: &str) -> String {
    let (testa, coda) = match grafo.rfind(",\"vista\":{") {
        Some(p) => grafo.split_at(p),
        None => (grafo.strip_suffix('}').unwrap_or(grafo), "}"),
    };
    format!("{testa},\"{chiave}\":{valore}{coda}")
}
