//! wasm-bindgen bridge of the structure rules of the fragmentation graph: a SMILES gives the expectations (`attese`) of the rules whose structure
//! conditions hold, which the page passes to `msn_grafo` as the query key `attese`. A `.wasm` of its own, so the `msngrafo` one does not carry the
//! chemistry. Errors are JSON `{"error_key", "params"}` strings.
use mzlab_core::error::Error;
use mzlab_core::json::Out;
use mzlab_core::msngrafo::tabella_regole;
use wasm_bindgen::prelude::*;

/// Stable version of the bridge: the page checks that the loaded `.wasm` is the one it was built with.
const BRIDGE: &str = "msnregole/1";

fn js_err(e: Error) -> JsValue {
    JsValue::from_str(&e.to_json())
}

#[wasm_bindgen]
pub fn version() -> String {
    BRIDGE.to_string()
}

/// `[["loss_h2o","H2O",0], ...]` (rule, formula, 1 = ion) of a SMILES, the value of the query key `attese` of `msn_grafo`.
#[wasm_bindgen]
pub fn attese(smiles: &str) -> std::result::Result<String, JsValue> {
    let a = mzlab_feat::chem::regole::attese(smiles, tabella_regole()).map_err(js_err)?;
    let mut o = Out::default();
    o.raw("[");
    for (i, x) in a.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("[")
            .str(&x.regola)
            .raw(",")
            .str(&x.formula);
        o.raw(if x.ione { ",1]" } else { ",0]" });
    }
    o.raw("]");
    Ok(o.0)
}
