//! wasm-bindgen bridge of the routes that the engine `.wasm` (`mzlab-wasm`) does not carry: the statistics of the series (`stat_post`), `molecules/1`,
//! `campioni/2` and `/3` (the Data view) and the Flow's command, line and scheduler. A `.wasm` of its own, fetched by the worker of the engine on the
//! first request of one of them (`load("rotte")` of `mzlab/web/rust-msngrafo.js`, called by `rust-worker.js`). Errors are JSON `{"error_key", "params"}` strings, as in the engine.
#![forbid(unsafe_code)]
use wasm_bindgen::prelude::*;

/// Stable version of the bridge: the page checks that the loaded `.wasm` is the one it was built with.
const BRIDGE: &str = "rotte/1";

fn error_json(e: &mzlab_core::Error) -> JsValue {
    JsValue::from_str(&e.to_json())
}

#[wasm_bindgen]
pub fn version() -> String {
    BRIDGE.to_string()
}

/// The message of a panic goes to a global of the worker (`__mzlabPanic`) before the abort, as in the engine, so the supervisor can keep it.
#[wasm_bindgen(start)]
pub fn start() {
    std::panic::set_hook(Box::new(|info| {
        let _ = js_sys::Reflect::set(
            &js_sys::global(),
            &"__mzlabPanic".into(),
            &info.to_string().into(),
        );
    }));
}

/// `mzlab-core::statroute`: the JSON answer, prefixed with `E` for an error (status 400); empty when `path` is not one of the routes.
#[wasm_bindgen]
pub fn stat_post(path: &str, body: &str) -> String {
    match mzlab_core::statroute::stat_route(path, body) {
        None => String::new(),
        Some(Ok(json)) => json,
        Some(Err(json)) => format!("E{json}"),
    }
}

/// `op` is `carica`, `cerca`, `get`, `elenca`, `da_csv`, `valida`, `a_csv`, `a_json` or `digest`; JSON in, JSON out (CSV text for `a_csv`), an error is JSON with a key.
#[wasm_bindgen]
pub fn molecules(op: &str, input: &str) -> Result<String, JsValue> {
    mzlab_core::molecules::route(op, input).map_err(|e| error_json(&e))
}

/// `op` is `proponi`, `proponi3`, `valida`, `da_legacy`, `per_tp`, `canonico`, `digest`, `migra`, `modello`, `dal_file`, `csv_leggi` or `csv_scrivi` (see `campioni::route`); JSON in, JSON out, an error is JSON with a key.
#[wasm_bindgen]
pub fn campioni(op: &str, input: &str) -> Result<String, JsValue> {
    mzlab_core::campioni::route_tutto(op, input).map_err(|e| error_json(&e))
}

/// The script that reruns an `mzflow/2` plan on the command line (`mzlab_core::flow::command_route`): text in, text out.
#[wasm_bindgen]
pub fn flow_command(plan: &str) -> Result<String, JsValue> {
    mzlab_core::flow::command_route(plan).map_err(|e| error_json(&e))
}

/// The short command line of a plan, or the plan with the `--set` of a line applied (`mzlab_core::flow::line_route`): JSON in, text out.
#[wasm_bindgen]
pub fn flow_line(body: &str) -> Result<String, JsValue> {
    mzlab_core::flow::line_route(body, None).map_err(|e| error_json(&e))
}

/// The live scheduler of the Flow (`mzlab_core::flow::sched_route`): `op` is one of its operations, JSON in, JSON out.
#[wasm_bindgen]
pub fn flow_sched(op: &str, body: &str) -> Result<String, JsValue> {
    mzlab_core::flow::sched_route(op, body).map_err(|e| error_json(&e))
}
