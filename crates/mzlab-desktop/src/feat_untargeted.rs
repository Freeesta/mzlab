//! The routes `feat/untargeted/detect` and `feat/untargeted/align` of `rpc`: the pipeline after mzmine 3.9.0 (`mzlab_feat_wasm::untargeted`, the steps the `.wasm` `untargeted`
//! runs) over the `.mzidx` of the index folder, with the same state as `feat/detect` and `feat/align` (the `.mzfeatf` of each file, the table), so that the views of the Pipeline
//! panel read its table unchanged. `feat/formula_id` (`{mz[], y[], feature_mz, params}`) answers the JSON `formula_id/1` of `mzlab_feat::untargeted::formula_id`. `{op: "feat/untargeted/detect", name, params}` and `{op: "feat/untargeted/align", names, params}`; `params` is the JSON text of the recipe.

use mzlab_core::budget::MemBudget;
use mzlab_core::{Error, Result};
use mzlab_feat::untargeted::formula_id::{identify, to_json, Opts};
use mzlab_feat::untargeted::pipeline::UntParams;
use mzlab_feat_wasm::untargeted;
use serde_json::Value;

use crate::engine::State;
use crate::feat::{names, written};
use crate::rpc::Answer;

impl State {
    /// Candidate compositions of one feature from its MS1 spectrum (`formula_id/1`): `{mz: [..], y: [..], feature_mz, params}`, `params` the JSON text of the options.
    pub(crate) fn feat_formula_id(&self, p: &Value) -> Result<Answer> {
        let list = |k: &str| -> Vec<f64> {
            p[k].as_array()
                .map(|a| a.iter().filter_map(Value::as_f64).collect())
                .unwrap_or_default()
        };
        let (mz, y) = (list("mz"), list("y"));
        if mz.len() != y.len() {
            return Err(Error::new("error.badRequest"));
        }
        let o = Opts::from_json(p["params"].as_str().unwrap_or(""))?;
        let r = identify(&mz, &y, p["feature_mz"].as_f64().unwrap_or(f64::NAN), &o)?;
        crate::rpc::text(to_json(&r))
    }

    pub(crate) fn feat_unt_detect(&self, p: &Value) -> Result<Answer> {
        let name = p["name"].as_str().unwrap_or("");
        let params = UntParams::from_json(p["params"].as_str().unwrap_or(""))?;
        let mut ix = self.open_idx(name)?;
        let mut bytes = Vec::new();
        self.indexer()?
            .pool
            .install(|| untargeted::detect_to(&mut ix, name, &params, &mut bytes))?;
        let a = written(&bytes);
        self.feat_lock().files.insert(name.to_string(), bytes);
        a
    }

    pub(crate) fn feat_unt_align(&self, p: &Value) -> Result<Answer> {
        let ns = names(p);
        if ns.is_empty() {
            return Err(Error::new("error.featState"));
        }
        let params = UntParams::from_json(p["params"].as_str().unwrap_or(""))?;
        let mut src = self.feat_stored(&ns)?;
        let budget = MemBudget::mb(mzlab_core::budget::NATIVE_BUDGET_MB as f64);
        let mut out = Vec::new();
        untargeted::align_to(
            &mut src,
            &budget,
            &params,
            |k| self.open_idx(&ns[k]),
            &mut out,
        )?;
        let a = written(&out);
        let mut g = self.feat_lock();
        g.table = Some(out);
        g.hypo = None;
        g.dig = None;
        g.lr = None;
        a
    }
}
