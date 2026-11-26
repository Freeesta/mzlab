//! The route `feat/estimate` of mzFinder in the app: the `estimates/1` of one file, the native twin of `flow_estimate` of the `.wasm`.

use mzlab_core::Result;
use serde_json::Value;

use crate::engine::State;
use crate::rpc::{text, Answer};

impl State {
    /// `feat/estimate {name, params}`: the `estimates/1` of one file as `features.estimate` writes it, with the `nominal` member the Pipeline panel shows beside the
    /// profile choice (`params` is the JSON object text of the step's parameters).
    pub(crate) fn feat_estimate(&self, p: &Value) -> Result<Answer> {
        let name = p["name"].as_str().unwrap_or("");
        let mut ix = self.open_idx(name)?;
        let given = mzlab_feat::flow_ms2::json_params(p["params"].as_str().unwrap_or("{}"))?;
        text(mzlab_ops::run::feat::run_estimate::estimate_of(&mut ix, name, &given)?.0)
    }
}
