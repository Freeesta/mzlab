//! wasm-bindgen bridge of `mzlab_feat::msnfigura`: the figure of a fragmentation graph `msngrafo/1` as SVG or stand-alone HTML, the bytes `flow run` writes for
//! `msn.figure@1`. A `.wasm` of its own, loaded by the pool of the Flow on the first `msn.figure` job; errors are JSON `{"error_key", "params"}` strings.
use mzlab_core::error::Error;
use mzlab_core::json::{parse, Value};
use wasm_bindgen::prelude::*;

fn js_err(e: Error) -> JsValue {
    JsValue::from_str(&e.to_json())
}

/// `msn.figure` in the browser: the figure of `graph` under the step's parameters (a JSON object, defaults merged: `format` svg|html, `view` tree|radial).
#[wasm_bindgen]
pub fn flow_figure(params_json: &str, graph: &[u8]) -> std::result::Result<String, JsValue> {
    let params = parse(params_json.as_bytes()).map_err(js_err)?;
    let text =
        |k: &str, d: &'static str| params.get(k).and_then(Value::str).unwrap_or(d).to_string();
    mzlab_feat::msnfigura::figure_text(graph, &text("format", "svg"), &text("view", "tree"))
        .map_err(js_err)
}
