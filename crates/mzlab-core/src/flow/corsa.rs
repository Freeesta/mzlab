//! The `corsa/1` record: one run of a workflow plan as a report that can be printed and checked. It is built from the `prov/1` record of the run (read, never
//! changed) and the named output files of the run folder (`annot/1` for the level counts, `report/1` of the table, included as it is). Every number of the report
//! is made here; the page only lays them out. No verdicts: levels from `annot.level` are counted as `auto`, apart from the others.

use super::canon::value_text;
use crate::error::{Error, Result};
use crate::json::{parse, Out, Value};

/// A named output file of the run folder.
pub struct Uscita<'a> {
    pub name: &'a str,
    pub bytes: &'a [u8],
}

/// A file read by the plan.
#[derive(Debug, Clone, PartialEq)]
pub struct Ingresso {
    pub name: String,
    pub bytes: u64,
    pub sha256: String,
    pub ty: String,
}

/// A parameter of a step as it ran: `estimated` when the step measured it (with the proof), else `fixed`.
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub value: String,
    pub proof: Option<String>,
}

/// One instance of the plan.
#[derive(Debug, Clone, PartialEq)]
pub struct Passo {
    pub instance: String,
    pub step: String,
    pub algorithm: Option<String>,
    pub preset: Option<String>,
    /// `ok`, `cached` or `error`.
    pub status: &'static str,
    pub error_key: Option<String>,
    pub ms: f64,
    /// Memory the step asked for in MB and the way it took (`memory`, `slabs`, `refused`), when the step estimates it.
    pub mem_mb: Option<f64>,
    pub via: Option<String>,
    /// What a per-file step says of each file (`spectra.centroid`: `via` = `centroided` | `passthrough`, with the estimates), as the prov/1 holds it.
    pub files: Vec<(String, Value)>,
    /// `warn.*` keys of the step.
    pub warnings: Vec<String>,
    pub params_sha256: String,
    pub params: Vec<Param>,
}

/// Rows per identification level, `auto` (the level came from `annot.level`) apart from the others.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Livelli {
    pub files: u32,
    pub rows: u32,
    pub auto: Vec<(String, u32)>,
    pub other: Vec<(String, u32)>,
}

/// The report of a run.
#[derive(Debug, Clone, PartialEq)]
pub struct Corsa {
    pub engine: Vec<(String, String)>,
    /// `engine.modules` of the record: name -> SHA-256, in the order of the record.
    pub modules: Vec<(String, String)>,
    pub plan_sha256: Option<String>,
    pub inputs: Vec<Ingresso>,
    pub lists: Vec<Ingresso>,
    pub steps: Vec<Passo>,
    pub levels: Livelli,
    /// The `report/1` text of the table, as the table wrote it.
    pub report: Option<String>,
    /// The lists the run screened against and the candidates they name (`screen/1`, `annot/1`).
    pub regulatory: super::corsa_reg::Regulatory,
}

/// Types of file that are lists the run used (libraries, transformation and structure lists), reported apart from the data files.
const LIST_TYPES: [&str; 4] = ["library", "transform", "structure", "regulatory"];

fn text(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(|x| x.str()).map(str::to_string)
}

fn bump(list: &mut Vec<(String, u32)>, level: &str) {
    match list.iter_mut().find(|(l, _)| l == level) {
        Some(e) => e.1 += 1,
        None => list.push((level.to_string(), 1)),
    }
}

fn passo(name: &str, a: &Value) -> Passo {
    let proofs: &[(String, Value)] = match a.get("estimates") {
        Some(Value::Obj(o)) => o,
        _ => &[],
    };
    let params = match a.get("params") {
        Some(Value::Obj(o)) => o
            .iter()
            .map(|(k, v)| Param {
                name: k.clone(),
                value: value_text(v),
                proof: proofs.iter().find(|p| p.0 == *k).map(|p| value_text(&p.1)),
            })
            .collect(),
        _ => Vec::new(),
    };
    let error_key = a
        .get("error")
        .and_then(|e| e.get("error_key"))
        .and_then(|k| k.str())
        .map(str::to_string);
    let cached = a.get("cached").and_then(|c| c.bool()).unwrap_or(false);
    Passo {
        instance: name.to_string(),
        step: text(a, "step").unwrap_or_default(),
        algorithm: text(a, "algorithm"),
        preset: text(a, "preset"),
        status: if error_key.is_some() {
            "error"
        } else if cached {
            "cached"
        } else {
            "ok"
        },
        error_key,
        ms: a.get("ms").and_then(|m| m.f64()).unwrap_or(0.0),
        mem_mb: a.get("mem_mb").and_then(|m| m.f64()),
        via: text(a, "via"),
        files: match a.get("files") {
            Some(Value::Obj(o)) => o.clone(),
            _ => Vec::new(),
        },
        warnings: a
            .get("warnings")
            .map(Value::arr)
            .unwrap_or_default()
            .iter()
            .filter_map(|w| w.str().map(str::to_string))
            .collect(),
        params_sha256: text(a, "params_sha256").unwrap_or_default(),
        params,
    }
}

/// The string keys of `agent` that `engine` copies, in this order.
const ENGINE_KEYS: [&str; 7] = [
    "name",
    "version",
    "commit",
    "platform",
    "target",
    "rustc",
    "cargo_lock_sha256",
];

/// `agent.modules` of the record: name -> SHA-256.
fn modules_of(agent: Option<&Value>) -> Vec<(String, String)> {
    match agent.and_then(|a| a.get("modules")) {
        Some(Value::Obj(m)) => m
            .iter()
            .filter_map(|(k, v)| v.str().map(|v| (k.clone(), v.to_string())))
            .collect(),
        _ => Vec::new(),
    }
}

/// `"modules":{...}` inside `engine`, when there are any.
fn modules_json(o: &mut Out, modules: &[(String, String)], comma: bool) {
    if modules.is_empty() {
        return;
    }
    o.raw(if comma {
        ",\"modules\":{"
    } else {
        "\"modules\":{"
    });
    for (k, (name, v)) in modules.iter().enumerate() {
        o.raw(if k > 0 { "," } else { "" })
            .str(name)
            .raw(":")
            .str(v);
    }
    o.raw("}");
}

/// Builds the report from the `prov/1` of a run and the output files that go with it; a file that is not JSON of a known format is ignored.
pub fn build(prov: &Value, uscite: &[Uscita]) -> Result<Corsa> {
    if prov.get("schema").and_then(|s| s.str()) != Some("prov/1") {
        return Err(Error::new("err.corsa.prov"));
    }
    let agent = prov.get("agent");
    let mut engine = Vec::new();
    for k in ENGINE_KEYS {
        if let Some(v) = agent.and_then(|a| text(a, k)) {
            engine.push((k.to_string(), v));
        }
    }
    let modules = modules_of(agent);
    let (mut inputs, mut lists) = (Vec::new(), Vec::new());
    if let Some(Value::Obj(entities)) = prov.get("entity") {
        for (key, e) in entities {
            let Some(name) = key.strip_prefix("file:") else {
                continue;
            };
            let f = Ingresso {
                name: name.to_string(),
                bytes: e.get("bytes").and_then(|b| b.f64()).unwrap_or(0.0) as u64,
                sha256: text(e, "sha256").unwrap_or_default(),
                ty: text(e, "type").unwrap_or_default(),
            };
            if LIST_TYPES.contains(&f.ty.as_str()) {
                lists.push(f);
            } else {
                inputs.push(f);
            }
        }
    }
    let steps = match prov.get("activity") {
        Some(Value::Obj(a)) => a.iter().map(|(k, v)| passo(k, v)).collect(),
        _ => Vec::new(),
    };
    let mut levels = Livelli::default();
    let (mut report, mut regulatory) = (None, super::corsa_reg::Regulatory::default());
    for u in uscite {
        let Ok(v) = parse(u.bytes) else { continue };
        regulatory.read(&v);
        if v.get("schema").and_then(|s| s.str()) == Some("report/1") {
            report = Some(String::from_utf8_lossy(u.bytes).trim_end().to_string());
        }
        if v.get("format").and_then(|s| s.str()) == Some("annot/1") {
            levels.files += 1;
            for row in v.get("rows").map_or(&[][..], |r| r.arr()) {
                let Some(level) = row.get("level").and_then(|l| l.str()) else {
                    continue;
                };
                levels.rows += 1;
                let auto = row.get("auto").and_then(|a| a.bool()).unwrap_or(true);
                bump(
                    if auto {
                        &mut levels.auto
                    } else {
                        &mut levels.other
                    },
                    level,
                );
            }
        }
    }
    crate::sortc::slice_ord(&mut levels.auto);
    crate::sortc::slice_ord(&mut levels.other);
    Ok(Corsa {
        engine,
        modules,
        plan_sha256: text(prov, "plan_sha256"),
        inputs,
        lists,
        steps,
        levels,
        report,
        regulatory,
    })
}

fn files(o: &mut Out, list: &[Ingresso]) {
    o.raw("[");
    for (k, f) in list.iter().enumerate() {
        o.raw(if k > 0 { ",{\"name\":" } else { "{\"name\":" })
            .str(&f.name)
            .raw(",\"bytes\":")
            .int(f.bytes as i64)
            .raw(",\"sha256\":")
            .str(&f.sha256)
            .raw(",\"type\":")
            .str(&f.ty)
            .raw("}");
    }
    o.raw("]");
}

fn counts(o: &mut Out, list: &[(String, u32)]) {
    o.raw("{");
    for (k, (level, n)) in list.iter().enumerate() {
        o.raw(if k > 0 { "," } else { "" })
            .str(level)
            .raw(":")
            .int(*n as i64);
    }
    o.raw("}");
}

fn param(o: &mut Out, p: &Param) {
    o.raw("{\"name\":")
        .str(&p.name)
        .raw(",\"value\":")
        .raw(&p.value)
        .raw(",\"state\":")
        .str(if p.proof.is_some() {
            "estimated"
        } else {
            "fixed"
        });
    if let Some(proof) = &p.proof {
        o.raw(",\"proof\":").raw(proof);
    }
    o.raw("}");
}

/// The memory and the warnings of a step, when it has any.
fn notes(o: &mut Out, s: &Passo) {
    if let (Some(mb), Some(via)) = (s.mem_mb, &s.via) {
        o.raw(",\"mem_mb\":").num(mb).raw(",\"via\":").str(via);
    }
    if !s.files.is_empty() {
        o.raw(",\"files\":{");
        for (j, (name, v)) in s.files.iter().enumerate() {
            o.raw(if j > 0 { "," } else { "" })
                .str(name)
                .raw(":")
                .raw(&value_text(v));
        }
        o.raw("}");
    }
    if !s.warnings.is_empty() {
        o.raw(",\"warnings\":[");
        for (j, w) in s.warnings.iter().enumerate() {
            o.raw(if j > 0 { "," } else { "" }).str(w);
        }
        o.raw("]");
    }
}

fn step(o: &mut Out, s: &Passo) {
    o.raw("{\"instance\":")
        .str(&s.instance)
        .raw(",\"step\":")
        .str(&s.step);
    if let Some(a) = &s.algorithm {
        o.raw(",\"algorithm\":").str(a);
    }
    if let Some(p) = &s.preset {
        o.raw(",\"preset\":").str(p);
    }
    o.raw(",\"status\":").str(s.status);
    if let Some(e) = &s.error_key {
        o.raw(",\"error_key\":").str(e);
    }
    o.raw(",\"ms\":").num(s.ms);
    notes(o, s);
    o.raw(",\"params_sha256\":")
        .str(&s.params_sha256)
        .raw(",\"params\":[");
    for (j, p) in s.params.iter().enumerate() {
        o.raw(if j > 0 { "," } else { "" });
        param(o, p);
    }
    o.raw("]}");
}

impl Corsa {
    /// The `corsa/1` text: fixed key order, shortest numbers, one line and a newline; the same run gives the same bytes.
    pub fn json(&self) -> String {
        let mut o = Out::default();
        o.raw("{\"schema\":\"corsa/1\",\"engine\":{");
        for (k, (name, v)) in self.engine.iter().enumerate() {
            o.raw(if k > 0 { "," } else { "" })
                .str(name)
                .raw(":")
                .str(v);
        }
        modules_json(&mut o, &self.modules, !self.engine.is_empty());
        o.raw("}");
        if let Some(p) = &self.plan_sha256 {
            o.raw(",\"plan_sha256\":").str(p);
        }
        o.raw(",\"inputs\":");
        files(&mut o, &self.inputs);
        o.raw(",\"lists\":");
        files(&mut o, &self.lists);
        o.raw(",\"steps\":[");
        for (k, s) in self.steps.iter().enumerate() {
            o.raw(if k > 0 { "," } else { "" });
            step(&mut o, s);
        }
        let total: f64 = self.steps.iter().map(|s| s.ms).sum();
        o.raw("],\"total_ms\":")
            .num((total * 10.0).round() / 10.0)
            .raw(",\"levels\":{\"files\":")
            .int(self.levels.files as i64)
            .raw(",\"rows\":")
            .int(self.levels.rows as i64)
            .raw(",\"auto\":");
        counts(&mut o, &self.levels.auto);
        o.raw(",\"other\":");
        counts(&mut o, &self.levels.other);
        o.raw("}");
        if let Some(r) = &self.report {
            o.raw(",\"report\":").raw(r);
        }
        self.regulatory.write(&mut o);
        o.raw("}\n");
        o.0
    }
}

#[cfg(test)]
#[path = "corsa_tests.rs"]
mod tests;
