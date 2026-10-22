//! `campioni/2` for the Data view (`mzlab_core::campioni::route`).

use wasm_bindgen::prelude::*;

/// `op` is `proponi`, `valida`, `da_legacy` or `per_tp`; JSON in, JSON out, an error is JSON with a key.
#[wasm_bindgen]
pub fn campioni(op: &str, input: &str) -> Result<String, JsValue> {
    mzlab_core::campioni::route(op, input).map_err(|e| crate::error_json(&e))
}

/// The script that reruns an `mzflow/2` plan on the command line (`mzlab_core::flow::command_route`): text in, text out.
#[wasm_bindgen]
pub fn flow_command(plan: &str) -> Result<String, JsValue> {
    mzlab_core::flow::command_route(plan).map_err(|e| crate::error_json(&e))
}

/// The live scheduler of the Flow (`mzlab_core::flow::sched_route`): `op` is one of its operations, JSON in, JSON out.
#[wasm_bindgen]
pub fn flow_sched(op: &str, body: &str) -> Result<String, JsValue> {
    mzlab_core::flow::sched_route(op, body).map_err(|e| crate::error_json(&e))
}
