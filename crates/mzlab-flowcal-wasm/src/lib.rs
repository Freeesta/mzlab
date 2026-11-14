//! wasm-bindgen bridge of `quant.calibrate@1`: the text is `mzlab_ops::run::table::quant::quant_text`, the same function `flow run` calls, so the browser and
//! the CLI write the same bytes. A `.wasm` of its own (`flowcal`): the `table` module has no room for it. Errors are JSON `{"error_key","params"}` strings.
#![forbid(unsafe_code)]
use mzlab_core::error::Error;
use mzlab_core::json::{Out, Value};
use mzlab_ops::run::table::quant;
use wasm_bindgen::prelude::*;

/// Stable version of the bridge: the page checks that the loaded `.wasm` is the one it was built with.
const BRIDGE: &str = "flowcal/1";

fn js_err(e: Error) -> JsValue {
    JsValue::from_str(&e.to_json())
}

#[wasm_bindgen]
pub fn version() -> String {
    BRIDGE.to_string()
}

/// `quant_text` for the browser: `params_json` is the step's parameters merged with its defaults; the answer is the JSON `{"text": <the CSV>, "warnings": [<warn.* keys>]}`.
/// Errors are JSON `{"error_key","params"}` texts.
#[wasm_bindgen]
pub fn flow_quant(
    params_json: &str,
    table: &[u8],
    curves: &[u8],
    samples: &[u8],
) -> std::result::Result<String, JsValue> {
    let params = match mzlab_core::json::parse(params_json.as_bytes()).map_err(js_err)? {
        Value::Obj(o) => o,
        _ => Vec::new(),
    };
    let (text, warnings) = quant::quant_text(&params, table, curves, samples).map_err(js_err)?;
    let mut o = Out::default();
    o.raw("{\"text\":");
    o.str(&text);
    o.raw(",\"warnings\":[");
    for (k, w) in warnings.iter().enumerate() {
        if k > 0 {
            o.raw(",");
        }
        o.str(w);
    }
    o.raw("]}");
    Ok(o.0)
}
