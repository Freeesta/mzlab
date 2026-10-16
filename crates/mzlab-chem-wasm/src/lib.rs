//! wasm-bindgen bridge of `mzlab-chem`: the SMILES of the parent gives its `reatt/1` (ETKDG geometry, GFN2-xTB charges and condensed Fukui
//! functions of the neutral molecule, the cation and the anion, C–H sites; `docs/agenti/reattivita.md`). A `.wasm` of its own, loaded only in
//! administrator mode. Memory: one call allocates its matrices and frees them on return; the output is one JSON string, no offsets
//! (`docs/agenti/memoria.md`). Errors are JSON `{"error_key", "params"}` strings.
use mzlab_core::error::Error;
use mzlab_core::json::Out;
use wasm_bindgen::prelude::*;

fn js_err(e: Error) -> JsValue {
    let mut o = Out::default();
    o.raw("{\"error_key\":").str(e.key).raw(",\"params\":{");
    for (i, (k, v)) in e.params.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(k).raw(":").str(v);
    }
    o.raw("}}");
    JsValue::from_str(&o.0)
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
