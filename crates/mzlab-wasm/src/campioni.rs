//! `campioni/2` and `/3` for the Data view (`mzlab_core::campioni::route_tutto`: also the name models, the CSV and the data of the files).

use wasm_bindgen::prelude::*;

/// `op` is `proponi`, `proponi3`, `valida`, `da_legacy`, `per_tp`, `canonico`, `digest`, `migra`, `modello`, `dal_file`, `csv_leggi` or `csv_scrivi` (see `campioni::route`); JSON in, JSON out, an error is JSON with a key.
#[wasm_bindgen]
pub fn campioni(op: &str, input: &str) -> Result<String, JsValue> {
    mzlab_core::campioni::route_tutto(op, input).map_err(|e| crate::error_json(&e))
}

/// The script that reruns an `mzflow/2` plan on the command line (`mzlab_core::flow::command_route`): text in, text out.
#[wasm_bindgen]
pub fn flow_command(plan: &str) -> Result<String, JsValue> {
    mzlab_core::flow::command_route(plan).map_err(|e| crate::error_json(&e))
}

/// The short command line of a plan, or the plan with the `--set` of a line applied (`mzlab_core::flow::line_route`): JSON in, text out.
#[wasm_bindgen]
pub fn flow_line(body: &str) -> Result<String, JsValue> {
    mzlab_core::flow::line_route(body, None).map_err(|e| crate::error_json(&e))
}

/// The live scheduler of the Flow (`mzlab_core::flow::sched_route`): `op` is one of its operations, JSON in, JSON out.
#[wasm_bindgen]
pub fn flow_sched(op: &str, body: &str) -> Result<String, JsValue> {
    mzlab_core::flow::sched_route(op, body).map_err(|e| crate::error_json(&e))
}
