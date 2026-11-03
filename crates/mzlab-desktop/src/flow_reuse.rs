//! The route `flow/reuse` of `rpc`: how many steps of a plan the cache of the last run would give back, for the line the Flow page shows before Run.
//! It stages the inputs in a folder of its own and only reads the cache of the run folder (`mzlab_cli::flow::probe_plan`); a run in progress is not touched.

use mzlab_core::flow::parse;
use mzlab_core::{Error, Result};
use serde_json::json;
use serde_json::Value;

use crate::engine::State;
use crate::flow_stage::stage_inputs;
use crate::rpc::{io_error, text, Answer};

impl State {
    /// `{plan: "<mzflow/2 text>", files?: {name: text}}` -> `{reused, compute}`: the steps taken again from the cache and the steps that would run. A file the
    /// plan names that the app does not hold is `err.flow.input.missing`, as in `flow/run`.
    pub(crate) fn flow_reuse(&self, p: &Value) -> Result<Answer> {
        let plan_text = p["plan"]
            .as_str()
            .ok_or_else(|| Error::new("err.flow.json"))?;
        let plan = parse(plan_text.as_bytes())?;
        let root = self.flow_root();
        let (inputs, work) = (root.join("probe"), root.join("out"));
        let _ = std::fs::remove_dir_all(&inputs);
        std::fs::create_dir_all(&inputs).map_err(|e| io_error(&e))?;
        let names: Vec<&str> = plan.inputs.iter().map(|f| f.name.as_str()).collect();
        let held = |name: &str| {
            self.conv_ready(name).or_else(|| {
                self.store
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .get(name)
                    .map(|e| e.path)
            })
        };
        stage_inputs(&names, &p["files"], &held, &inputs)?;
        let (reused, compute) = mzlab_cli::flow::probe_plan(plan_text.as_bytes(), &inputs, &work)?;
        text(json!({ "reused": reused, "compute": compute }).to_string())
    }
}
