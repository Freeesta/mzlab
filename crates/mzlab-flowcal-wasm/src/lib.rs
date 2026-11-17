//! wasm-bindgen bridge of `quant.calibrate@1` and `quant.curve@1`: the texts are `mzlab_ops::run::table::quant::quant_text` and `curve::curve_text`, the
//! functions `flow run` calls, so the browser and the CLI write the same bytes. A `.wasm` of its own (`flowcal`): the `table` module has no room for them.
//! Errors are JSON `{"error_key","params"}` strings.
#![forbid(unsafe_code)]
use mzlab_core::error::Error;
use mzlab_core::json::{Out, Value};
use mzlab_ops::run::table::{curve, quant};
use wasm_bindgen::prelude::*;

/// Stable version of the bridge: the page checks that the loaded `.wasm` is the one it was built with.
const BRIDGE: &str = "flowcal/2";

fn js_err(e: Error) -> JsValue {
    JsValue::from_str(&e.to_json())
}

fn params_of(params_json: &str) -> std::result::Result<Vec<(String, Value)>, JsValue> {
    Ok(
        match mzlab_core::json::parse(params_json.as_bytes()).map_err(js_err)? {
            Value::Obj(o) => o,
            _ => Vec::new(),
        },
    )
}

/// `"warnings":[...]}` closing an answer.
fn warnings_tail(o: &mut Out, warnings: &[String]) {
    o.raw(",\"warnings\":[");
    for (k, w) in warnings.iter().enumerate() {
        if k > 0 {
            o.raw(",");
        }
        o.str(w);
    }
    o.raw("]}");
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
    let params = params_of(params_json)?;
    let (text, warnings) = quant::quant_text(&params, table, curves, samples).map_err(js_err)?;
    let mut o = Out::default();
    o.raw("{\"text\":");
    o.str(&text);
    warnings_tail(&mut o, &warnings);
    Ok(o.0)
}

/// `curve_text` for the browser: the answer is `{"calibration": <taratura/1 text>, "concentrations": <the CSV>, "warnings": [<warn.* keys>]}`.
#[wasm_bindgen]
pub fn flow_curve(
    params_json: &str,
    responses: &[u8],
    standards: &[u8],
) -> std::result::Result<String, JsValue> {
    let params = params_of(params_json)?;
    let (json, csv, warnings) = curve::curve_text(&params, responses, standards).map_err(js_err)?;
    let mut o = Out::default();
    o.raw("{\"calibration\":");
    o.str(&json);
    o.raw(",\"concentrations\":");
    o.str(&csv);
    warnings_tail(&mut o, &warnings);
    Ok(o.0)
}
