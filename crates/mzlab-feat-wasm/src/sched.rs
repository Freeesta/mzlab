//! The live scheduler of a workflow (`mzlab_core::flow::sched`) for the Worker of the Flow page: one object per plan, text JSON in and out, no choice made in
//! JavaScript. Used by `flow/lazy/worker-flow-web.js` and by `tests/flow_sched_nodo.js` (parity on `sched_cases.json`). Design: `docs/agenti/flow-live.md`.

use mzlab_core::error::Error;
use mzlab_core::flow::sched::{Command, Done, JobId, Landed, Limits, Sched};
use mzlab_core::flow::{parse, Plan};
use mzlab_core::json::{self, Out, Value};
use mzlab_core::step::{DataType, Param, Port, Step};
use wasm_bindgen::prelude::*;

fn bad(msg: &str) -> JsValue {
    JsValue::from_str(msg)
}

fn read_plan(text: &str) -> Result<Plan, JsValue> {
    parse(text.as_bytes()).map_err(|e| bad(e.key))
}

fn leak(s: &str) -> &'static str {
    Box::leak(s.to_string().into_boxed_str())
}

fn ports(v: Option<&Value>) -> Vec<Port> {
    v.map(Value::arr)
        .unwrap_or_default()
        .iter()
        .filter_map(Value::str)
        .map(|n| {
            let p = Port::new(leak(n.trim_end_matches('*')), DataType::Raw, "");
            if n.ends_with('*') {
                p.many()
            } else {
                p
            }
        })
        .collect()
}

/// A registry written as data: `[{"id","ins":["port","many*"],"outs":[..],"per_item","params":{"name":default}}]`, the steps of the shared cases. Empty text
/// = the registry of the flagship chain compiled into the module. The steps of the data run on both platforms.
fn read_registry(text: &str) -> Result<Vec<Step>, JsValue> {
    if text.trim().is_empty() {
        return Ok(mzlab_feat::steps::registry());
    }
    let spec = json::parse(text.as_bytes()).map_err(|e| bad(e.key))?;
    Ok(spec
        .arr()
        .iter()
        .map(|s| {
            let mut st = Step::core(leak(s.get("id").and_then(Value::str).unwrap_or("")), "", "");
            st.per_item = s.get("per_item").and_then(Value::bool).unwrap_or(false);
            st.runs_in.native = true;
            st.runs_in.wasm = Some("hr");
            st.ins = ports(s.get("ins"));
            st.outs = ports(s.get("outs"));
            if let Some(Value::Obj(kv)) = s.get("params") {
                st.params = kv
                    .iter()
                    .map(|(k, v)| Param::number(leak(k), "").default_num(v.f64().unwrap_or(0.0)))
                    .collect();
            }
            st
        })
        .collect())
}

fn job_id(v: &Value) -> JobId {
    let text = |k: &str| v.get(k).and_then(Value::str);
    JobId::new(text("instance").unwrap_or(""), text("item"))
}

fn job_from(text: &str) -> Result<JobId, JsValue> {
    Ok(job_id(
        &json::parse(text.as_bytes()).map_err(|e| bad(e.key))?,
    ))
}

fn id_json(o: &mut Out, id: &JobId) {
    o.raw("{\"instance\":").str(&id.instance).raw(",\"item\":");
    match &id.item {
        Some(n) => o.str(n),
        None => o.raw("null"),
    };
    o.raw("}");
}

/// The scheduler of one plan.
#[wasm_bindgen]
pub struct FlowSched {
    inner: Sched,
}

#[wasm_bindgen]
impl FlowSched {
    /// `identities_json`: `{"<input name>": "<text that stands for its content>"}` for the plan inputs without a `sha256`.
    #[wasm_bindgen(constructor)]
    pub fn new(
        plan_text: &str,
        registry_text: &str,
        program_version: &str,
        identities_json: &str,
    ) -> Result<FlowSched, JsValue> {
        let ids = match json::parse(identities_json.as_bytes()) {
            Ok(Value::Obj(kv)) => kv
                .into_iter()
                .filter_map(|(k, v)| v.str().map(|t| (k, t.to_string())))
                .collect(),
            _ => Vec::new(),
        };
        let reg = read_registry(registry_text)?;
        let inner = Sched::new(&read_plan(plan_text)?, &reg, program_version, &ids);
        Ok(FlowSched { inner })
    }

    /// A new version of the plan: the jobs to cancel, `[{"instance","item"}]`.
    pub fn update(&mut self, plan_text: &str) -> Result<String, JsValue> {
        let cmds = self.inner.update(&read_plan(plan_text)?);
        let mut o = Out::default();
        o.raw("[");
        for (i, Command::Cancel(id)) in cmds.iter().enumerate() {
            o.raw(if i > 0 { "," } else { "" });
            id_json(&mut o, id);
        }
        Ok(o.raw("]").0.clone())
    }

    /// Fingerprint of a plan input without `sha256` (takes effect at the next `update`).
    pub fn set_identity(&mut self, name: &str, text: &str) {
        self.inner.set_identity(name, text);
    }

    /// Exclusive resources a job holds while it runs: `resources_json` is an array of names.
    pub fn set_resources(&mut self, job: &str, resources_json: &str) -> Result<(), JsValue> {
        let list = json::parse(resources_json.as_bytes()).map_err(|e| bad(e.key))?;
        let names = list.arr().iter().filter_map(Value::str).map(str::to_string);
        self.inner.set_resources(&job_from(job)?, names.collect());
        Ok(())
    }

    /// `limits_json`: `{"slots","mem_mb","mb":{"<instance>|<item>": estimate},"mb_default"}`; returns `[{"instance","item","key","mb","resources"}]`.
    pub fn next(&self, limits_json: &str) -> Result<String, JsValue> {
        let v = json::parse(limits_json.as_bytes()).map_err(|e| bad(e.key))?;
        let num = |k: &str, d: f64| v.get(k).and_then(Value::f64).unwrap_or(d);
        let limits = Limits {
            slots: num("slots", 99.0) as usize,
            mem_mb: num("mem_mb", 1e9),
        };
        let fallback = num("mb_default", 1.0);
        let table = v.get("mb");
        let estimate = |id: &JobId| {
            let name = match &id.item {
                Some(n) => format!("{}|{n}", id.instance),
                None => id.instance.clone(),
            };
            table
                .and_then(|t| t.get(&name))
                .and_then(Value::f64)
                .unwrap_or(fallback)
        };
        let mut o = Out::default();
        o.raw("[");
        for (i, j) in self.inner.next(&limits, estimate).iter().enumerate() {
            o.raw(if i > 0 { "," } else { "" });
            o.raw("{\"instance\":")
                .str(&j.id.instance)
                .raw(",\"item\":");
            match &j.id.item {
                Some(n) => o.str(n),
                None => o.raw("null"),
            };
            o.raw(",\"key\":")
                .str(&j.key)
                .raw(",\"mb\":")
                .num(j.mb)
                .raw(",\"resources\":[");
            for (k, r) in j.resources.iter().enumerate() {
                o.raw(if k > 0 { "," } else { "" }).str(r);
            }
            o.raw("]}");
        }
        Ok(o.raw("]").0.clone())
    }

    pub fn started(&mut self, job: &str) -> Result<bool, JsValue> {
        Ok(self.inner.started(&job_from(job)?))
    }

    pub fn progress(&mut self, job: &str, frac: f64) -> Result<(), JsValue> {
        self.inner.progress(&job_from(job)?, frac);
        Ok(())
    }

    /// `result_json`: `{"ms"}` for a job that is done, `{"error_key","params":{}}` for one that failed; returns `accepted`, `discarded:<key>` or `unknown`.
    pub fn finished(&mut self, job: &str, result_json: &str) -> Result<String, JsValue> {
        let r = json::parse(result_json.as_bytes()).map_err(|e| bad(e.key))?;
        let result = match r.get("error_key").and_then(Value::str) {
            Some(k) => {
                let mut e = Error::new(leak(k));
                if let Some(Value::Obj(kv)) = r.get("params") {
                    for (name, v) in kv {
                        e = e.with(leak(name), v.str().unwrap_or(""));
                    }
                }
                Err(e)
            }
            None => Ok(Done {
                ms: r.get("ms").and_then(Value::f64).unwrap_or(0.0),
            }),
        };
        Ok(match self.inner.finished(&job_from(job)?, result) {
            Landed::Accepted => "accepted".to_string(),
            Landed::Discarded { key } => format!("discarded:{key}"),
            Landed::Unknown => "unknown".to_string(),
        })
    }

    /// The caller found the `plan_key` of the job in its cache.
    pub fn cached(&mut self, job: &str) -> Result<bool, JsValue> {
        Ok(self.inner.cached(&job_from(job)?))
    }

    pub fn block(&mut self, job: &str, by: &str) -> Result<bool, JsValue> {
        Ok(self.inner.block(&job_from(job)?, by))
    }

    pub fn unblock(&mut self, job: &str) -> Result<(), JsValue> {
        self.inner.unblock(&job_from(job)?);
        Ok(())
    }

    pub fn retry(&mut self, job: &str) -> Result<(), JsValue> {
        self.inner.retry(&job_from(job)?);
        Ok(())
    }

    pub fn pause(&mut self) {
        self.inner.pause();
    }

    pub fn resume(&mut self) {
        self.inner.resume();
    }

    /// `[{"instance","item","state"}]` in the order of the plan.
    pub fn jobs(&self) -> String {
        let mut o = Out::default();
        o.raw("[");
        for (i, (id, st)) in self.inner.jobs().iter().enumerate() {
            o.raw(if i > 0 { "," } else { "" });
            id_json(&mut o, id);
            o.0.pop();
            o.raw(",\"state\":").str(st.name()).raw("}");
        }
        o.raw("]").0.clone()
    }

    /// The cards of the instances: `[{"instance","state","frac","parte","di","ms"}]`.
    pub fn nodes(&self) -> String {
        let mut o = Out::default();
        o.raw("[");
        for (i, n) in self.inner.nodes().iter().enumerate() {
            o.raw(if i > 0 { "," } else { "" });
            o.raw("{\"instance\":")
                .str(&n.instance)
                .raw(",\"state\":")
                .str(n.state.name());
            o.raw(",\"frac\":").opt(n.frac);
            o.raw(",\"parte\":")
                .int(n.parte as i64)
                .raw(",\"di\":")
                .int(n.di as i64);
            o.raw(",\"ms\":").num(n.ms).raw("}");
        }
        o.raw("]").0.clone()
    }

    /// The `flowlive/1` events with `seq` above the given one.
    pub fn events_since(&self, seq: f64) -> String {
        self.inner.events_since(seq as u64)
    }

    /// Number of `update` calls so far.
    pub fn rev(&self) -> f64 {
        self.inner.rev() as f64
    }
}
