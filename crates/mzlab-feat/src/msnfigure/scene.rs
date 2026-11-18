//! The model of the static figure: the scene read from `msn-graph/1` (ions, arcs, positions of the key `view`), the text pieces of the LABEL RULE of
//! `mzlab/web/msngrafo-vista.js` and the boxes of the ions. Used only by `msnfigura`.

use std::collections::{HashMap, HashSet};

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, Value};

pub(crate) const W: f64 = 176.0;
pub(crate) const WR: f64 = 150.0;
pub(crate) const LH: f64 = 14.0;
pub(crate) const GAP_X: f64 = 130.0;
pub(crate) const GAP_Y: f64 = 20.0;
pub(crate) const MAXC: usize = 4;
pub(crate) const DW: f64 = 150.0;
pub(crate) const DH: f64 = 80.0;

pub(crate) const INK: &str = "#1a1a1a";
pub(crate) const MUTED: &str = "#4d4d4d";
pub(crate) const LINE: &str = "#595959";
pub(crate) const BEST: &str = "#0072B2";
pub(crate) const WARN: &str = "#E69F00";

// ---- model -----------------------------------------------------------------------------------------------------------

pub(crate) struct Cand {
    pub(crate) formula: String,
    pub(crate) ppm: f64,
    pub(crate) evidence: Vec<String>,
}

pub(crate) struct Ion {
    pub(crate) id: String,
    pub(crate) level: i64,
    pub(crate) mz: f64,
    pub(crate) isolated: bool,
    pub(crate) cands: Vec<Cand>,
    pub(crate) choice: Option<String>,
    pub(crate) has_formula: bool,
}

pub(crate) struct Pos {
    pub(crate) livelli: Vec<i64>,
    pub(crate) rel: Option<f64>,
    pub(crate) da: Option<String>,
    pub(crate) entranti: i64,
    pub(crate) common: bool,
    pub(crate) col: i64,
    pub(crate) row: i64,
    pub(crate) ring: i64,
    pub(crate) angolo: f64,
}

pub(crate) struct Measure {
    pub(crate) livello: i64,
    pub(crate) att: Option<String>,
    pub(crate) ce: Option<f64>,
    pub(crate) rel: f64,
}

pub(crate) struct Edge {
    pub(crate) da: String,
    pub(crate) a: String,
    pub(crate) measures: Vec<Measure>,
    pub(crate) formula: Option<String>,
    pub(crate) from_mass: f64,
    pub(crate) compat: Vec<String>,
    pub(crate) rule: String,
}

/// Everything the drawing needs, read once from the graph.
pub(crate) struct Scena {
    pub(crate) ions: Vec<Ion>,
    pub(crate) pos: HashMap<String, Pos>,
    pub(crate) edges: Vec<Edge>,
    pub(crate) best: HashSet<String>,
    pub(crate) charge: String,
    pub(crate) molfile: Option<String>,
    pub(crate) smiles: bool,
}

pub(crate) fn bad() -> Error {
    Error::new("err.msnfigura.bad")
}

pub(crate) fn s(v: &Value, k: &str) -> Option<String> {
    v.get(k).and_then(|x| x.str()).map(str::to_string)
}

pub(crate) fn n(v: &Value, k: &str) -> Option<f64> {
    v.get(k).and_then(|x| x.f64())
}

/// The rule ids shown under a loss: at most two (the weak ones tagged), `+n` for the rest; empty when no rule explains it.
pub(crate) fn rule_of(j: &Value, da: &str, a: &str) -> String {
    let Some(r) = j.get("rules").and_then(|r| r.get("edges")).and_then(|l| {
        l.arr()
            .iter()
            .find(|x| s(x, "from").as_deref() == Some(da) && s(x, "to").as_deref() == Some(a))
    }) else {
        return String::new();
    };
    let reg = r.get("rules").map_or(&[][..], |x| x.arr());
    let comp = r.get("composite").map_or(&[][..], |x| x.arr());
    let ids: Vec<String> = if reg.is_empty() {
        comp.iter()
            .map(|c| {
                c.arr()
                    .iter()
                    .filter_map(|x| x.str())
                    .collect::<Vec<_>>()
                    .join(" + ")
            })
            .collect()
    } else {
        reg.iter()
            .map(|x| {
                let weak = if n(x, "priority") == Some(3.0) {
                    " (weak)"
                } else {
                    ""
                };
                format!("{}{weak}", s(x, "id").unwrap_or_default())
            })
            .collect()
    };
    if ids.is_empty() {
        return String::new();
    }
    let more = if ids.len() > 2 {
        format!(" +{}", ids.len() - 2)
    } else {
        String::new()
    };
    format!(
        "{}{more}",
        ids.iter().take(2).cloned().collect::<Vec<_>>().join(", ")
    )
}

fn read_ions(j: &Value) -> Result<Vec<Ion>> {
    let mut ions = Vec::new();
    for x in j.get("nodes").map_or(&[][..], |a| a.arr()) {
        let f = x.get("formula").filter(|f| !f.is_null());
        let cands = f
            .and_then(|f| f.get("candidates"))
            .map_or(&[][..], |c| c.arr())
            .iter()
            .map(|c| Cand {
                formula: s(c, "formula").unwrap_or_default(),
                ppm: n(c, "ppm").unwrap_or(0.0),
                evidence: c
                    .get("proofs")
                    .map_or(&[][..], |p| p.arr())
                    .iter()
                    .filter_map(|p| p.str().map(str::to_string))
                    .collect(),
            })
            .collect();
        ions.push(Ion {
            id: s(x, "id").ok_or_else(bad)?,
            level: n(x, "level").unwrap_or(0.0) as i64,
            mz: n(x, "mz").ok_or_else(bad)?,
            isolated: x.get("isolated").and_then(|b| b.bool()).unwrap_or(false),
            cands,
            choice: f.and_then(|f| s(f, "choice")),
            has_formula: f.is_some(),
        });
    }
    Ok(ions)
}

fn read_pos(view: &Value) -> Result<HashMap<String, Pos>> {
    let mut pos = HashMap::new();
    for x in view.get("nodes").map_or(&[][..], |a| a.arr()) {
        let best = x.get("best");
        let albero = x.get("tree");
        let radial = x.get("radial").filter(|r| !r.is_null());
        pos.insert(
            s(x, "id").ok_or_else(bad)?,
            Pos {
                livelli: x
                    .get("levels")
                    .map_or(&[][..], |l| l.arr())
                    .iter()
                    .filter_map(|l| l.f64())
                    .map(|l| l as i64)
                    .collect(),
                rel: n(x, "rel"),
                da: best.and_then(|b| s(b, "from")),
                entranti: n(x, "incoming").unwrap_or(0.0) as i64,
                common: x.get("common").and_then(|b| b.bool()).unwrap_or(false),
                col: albero.and_then(|a| n(a, "col")).unwrap_or(0.0) as i64,
                row: albero.and_then(|a| n(a, "row")).unwrap_or(0.0) as i64,
                ring: radial.and_then(|a| n(a, "ring")).unwrap_or(0.0) as i64,
                angolo: radial.and_then(|a| n(a, "angle")).unwrap_or(0.0),
            },
        );
    }
    Ok(pos)
}

fn read_edges(j: &Value) -> Result<Vec<Edge>> {
    let mut edges = Vec::new();
    for a in j.get("edges").map_or(&[][..], |a| a.arr()) {
        let (da, to) = (s(a, "from").ok_or_else(bad)?, s(a, "to").ok_or_else(bad)?);
        let p = a.get("loss");
        edges.push(Edge {
            measures: a
                .get("measurements")
                .map_or(&[][..], |m| m.arr())
                .iter()
                .map(|m| Measure {
                    livello: n(m, "level").unwrap_or(0.0) as i64,
                    att: s(m, "activation"),
                    ce: n(m, "ce"),
                    rel: n(m, "rel").unwrap_or(0.0),
                })
                .collect(),
            formula: p.and_then(|p| s(p, "formula")),
            from_mass: p.and_then(|p| n(p, "da_mass")).unwrap_or(0.0),
            compat: p
                .and_then(|p| p.get("compatible"))
                .map_or(&[][..], |c| c.arr())
                .iter()
                .filter_map(|c| c.str().map(str::to_string))
                .collect(),
            rule: rule_of(j, &da, &to),
            da,
            a: to,
        });
    }
    Ok(edges)
}

pub(crate) fn scena(graph: &str) -> Result<Scena> {
    let done = mzlab_core::msngraph::completa(graph).map_err(|_| bad())?;
    let j = parse(done.as_bytes()).map_err(|_| bad())?;
    let view = j.get("view").ok_or_else(bad)?;
    let ioni = read_ions(&j)?;
    let pos = read_pos(view)?;
    let archi = read_edges(&j)?;
    let best = view
        .get("edges")
        .map_or(&[][..], |a| a.arr())
        .iter()
        .filter(|a| a.get("best").and_then(|b| b.bool()) == Some(true))
        .map(|a| {
            format!(
                "{}>{}",
                s(a, "from").unwrap_or_default(),
                s(a, "to").unwrap_or_default()
            )
        })
        .collect();
    let o = j.get("options");
    Ok(Scena {
        ions: ioni,
        pos,
        edges: archi,
        best,
        charge: if n(&j, "polarity") == Some(-1.0) {
            "-"
        } else {
            "+"
        }
        .into(),
        molfile: o
            .and_then(|o| s(o, "molfile"))
            .filter(|m| !m.trim().is_empty()),
        smiles: o
            .and_then(|o| s(o, "smiles"))
            .is_some_and(|m| !m.is_empty()),
    })
}
