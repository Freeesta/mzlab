//! wasm-bindgen bridge of `mzlab-chem`: `reatt/1` of a SMILES (ETKDG geometry, GFN2-xTB charges, condensed Fukui functions, C–H sites;
//! `docs/agenti/reattivita.md`) for the page in administrator mode, and of the parent file of the Flow step `structure.reactivity` for the pool of the Flow.
//! A `.wasm` of its own. Memory: one call allocates its matrices and frees them on return; the output is one JSON string, no offsets
//! (`docs/agenti/memoria.md`). Errors are JSON `{"error_key", "params"}` strings.
use mzlab_core::error::Error;
use wasm_bindgen::prelude::*;

fn js_err(e: Error) -> JsValue {
    JsValue::from_str(&e.to_json())
}

/// `mzlab-chem/1`: the page checks that the loaded `.wasm` is the one it was built with.
#[wasm_bindgen]
pub fn version() -> String {
    mzlab_chem::ENGINE.to_string()
}

/// `reatt/1` of a SMILES; `opts_json` `{"solvent": "water"|"none", "conformers": 1-16, "seed"}`, every field optional (empty string for the defaults).
#[wasm_bindgen]
pub fn reactivity(smiles: &str, opts_json: &str) -> std::result::Result<String, JsValue> {
    mzlab_chem::reactivity_json(smiles, opts_json).map_err(js_err)
}

/// `structure.reactivity` of the Flow in the browser: `reatt/1` of the parent file under the step's parameters (a JSON object, defaults merged), the
/// text `flow run` writes (`mzlab_chem::flow::reactivity_text`).
#[wasm_bindgen]
pub fn flow_reactivity(params_json: &str, parent: &[u8]) -> std::result::Result<String, JsValue> {
    let params = match mzlab_core::json::parse(params_json.as_bytes()).map_err(js_err)? {
        mzlab_core::json::Value::Obj(o) => o,
        _ => Vec::new(),
    };
    mzlab_chem::flow::reactivity_text(&params, parent).map_err(js_err)
}
