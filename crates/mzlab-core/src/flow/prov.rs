//! Writer of the `prov/1` record of a run (`docs/studi/workflow/contract.schema.json`, def `prov1`): an entity for every file read or made, an activity for
//! every instance, the agent that ran them. The same inputs and parameters give the same text, but for `ms` and `heap_peak_bytes`.

use super::canon::value_text;
use super::run::Run;
use crate::json::{Out, Value};

/// The program that ran the plan.
#[derive(Debug, Clone, PartialEq)]
pub struct Agent {
    pub name: String,
    pub version: String,
    pub commit: Option<String>,
    /// `native` or `wasm`.
    pub platform: &'static str,
}

/// Why an activity stopped: an `err.*` key and its values.
#[derive(Debug, Clone, PartialEq)]
pub struct ErrorRecord {
    pub error_key: String,
    pub params: Vec<(String, String)>,
}

/// The working memory a step asked for and the way it took: `memory`, `slabs` (same bytes in less memory) or `refused`.
#[derive(Debug, Clone, PartialEq)]
pub struct Mem {
    pub mb: f64,
    pub via: &'static str,
}

/// What one instance did.
#[derive(Debug, Clone, PartialEq)]
pub struct Activity {
    pub instance: String,
    /// `id@version` of the step.
    pub step: String,
    /// `id@version` of the algorithm that ran.
    pub algorithm: Option<String>,
    pub preset: Option<String>,
    /// Canonical parameters, the measured ones included.
    pub params: Vec<(String, Value)>,
    pub params_sha256: String,
    /// Parameter, proof of the estimate.
    pub estimates: Vec<(String, Value)>,
    pub used: Vec<String>,
    pub generated: Vec<String>,
    pub ms: f64,
    pub heap_peak_bytes: Option<u64>,
    /// The estimate of the memory of the step, when it has one (feature alignment); it does not depend on the run, the way does on the budget.
    pub mem: Option<Mem>,
    /// `warn.*` keys: what the step noticed and went on with.
    pub warnings: Vec<String>,
    pub error: Option<ErrorRecord>,
    pub cached: bool,
    /// `citation_key` of the step and of the algorithm that ran.
    pub references: Vec<String>,
}

fn strs(o: &mut Out, a: &[String]) {
    o.raw("[");
    for (k, s) in a.iter().enumerate() {
        o.raw(if k > 0 { "," } else { "" }).str(s);
    }
    o.raw("]");
}

fn object(o: &mut Out, kv: &[(String, Value)]) {
    let mut kv: Vec<&(String, Value)> = kv.iter().collect();
    crate::sortc::slice_by(&mut kv, |a, b| a.0.cmp(&b.0));
    o.raw("{");
    for (k, (key, v)) in kv.into_iter().enumerate() {
        o.raw(if k > 0 { "," } else { "" })
            .str(key)
            .raw(":")
            .raw(&value_text(v));
    }
    o.raw("}");
}

fn activity(o: &mut Out, a: &Activity) {
    o.str(&a.instance).raw(":{\"step\":").str(&a.step);
    if let Some(x) = &a.algorithm {
        o.raw(",\"algorithm\":").str(x);
    }
    if let Some(x) = &a.preset {
        o.raw(",\"preset\":").str(x);
    }
    o.raw(",\"params\":");
    object(o, &a.params);
    o.raw(",\"params_sha256\":").str(&a.params_sha256);
    if !a.estimates.is_empty() {
        o.raw(",\"estimates\":");
        object(o, &a.estimates);
    }
    o.raw(",\"used\":");
    strs(o, &a.used);
    o.raw(",\"generated\":");
    strs(o, &a.generated);
    o.raw(",\"ms\":").num((a.ms * 10.0).round() / 10.0);
    if let Some(h) = a.heap_peak_bytes {
        o.raw(",\"heap_peak_bytes\":").int(h as i64);
    }
    if let Some(m) = &a.mem {
        o.raw(",\"mem_mb\":")
            .num((m.mb * 10.0).round() / 10.0)
            .raw(",\"via\":")
            .str(m.via);
    }
    if !a.warnings.is_empty() {
        o.raw(",\"warnings\":");
        strs(o, &a.warnings);
    }
    if let Some(e) = &a.error {
        o.raw(",\"error\":{\"error_key\":")
            .str(&e.error_key)
            .raw(",\"params\":{");
        for (k, (name, v)) in e.params.iter().enumerate() {
            o.raw(if k > 0 { "," } else { "" })
                .str(name)
                .raw(":")
                .str(v);
        }
        o.raw("}}");
    }
    if a.cached {
        o.raw(",\"cached\":true");
    }
    if !a.references.is_empty() {
        o.raw(",\"references\":");
        strs(o, &a.references);
    }
    o.raw("}");
}

/// The assistant that launched a run: the key `llm` of `prov/1`, absent for a run the user started.
#[derive(Debug, Clone, PartialEq)]
pub struct LlmRef {
    pub log_sha256: String,
    pub turn: u32,
    pub provider: String,
    pub model: String,
    pub level: String,
}

/// The `prov/1` text of a run; `plan_sha256` is the SHA-256 of the file that was run.
pub fn prov_json(agent: &Agent, plan_sha256: Option<&str>, run: &Run) -> String {
    prov_json_llm(agent, plan_sha256, run, None)
}

/// `prov_json` plus, when an assistant launched the run, `llm{log_sha256, turn, provider, model, level}`; with `None` the bytes are those of `prov_json`.
pub fn prov_json_llm(
    agent: &Agent,
    plan_sha256: Option<&str>,
    run: &Run,
    llm: Option<&LlmRef>,
) -> String {
    let mut o = Out::default();
    o.raw("{\"schema\":\"prov/1\",\"agent\":{\"name\":")
        .str(&agent.name);
    o.raw(",\"version\":").str(&agent.version);
    if let Some(c) = &agent.commit {
        o.raw(",\"commit\":").str(c);
    }
    o.raw(",\"platform\":").str(agent.platform).raw("}");
    if let Some(p) = plan_sha256 {
        o.raw(",\"plan_sha256\":").str(p);
    }
    o.raw(",\"entity\":{");
    for (k, (key, e)) in run.entities.iter().enumerate() {
        o.raw(if k > 0 { "," } else { "" }).str(key);
        o.raw(":{\"sha256\":")
            .str(&e.sha256)
            .raw(",\"bytes\":")
            .int(e.bytes as i64);
        o.raw(",\"type\":").str(e.ty.name());
        if let Some(l) = &e.label {
            o.raw(",\"label\":").str(l);
        }
        o.raw("}");
    }
    o.raw("},\"activity\":{");
    for (k, a) in run.activities.iter().enumerate() {
        o.raw(if k > 0 { "," } else { "" });
        activity(&mut o, a);
    }
    o.raw("}");
    if let Some(l) = llm {
        o.raw(",\"llm\":{\"log_sha256\":").str(&l.log_sha256);
        o.raw(",\"turn\":").int(i64::from(l.turn));
        o.raw(",\"provider\":").str(&l.provider);
        o.raw(",\"model\":").str(&l.model);
        o.raw(",\"level\":").str(&l.level).raw("}");
    }
    o.raw("}\n");
    o.0
}
