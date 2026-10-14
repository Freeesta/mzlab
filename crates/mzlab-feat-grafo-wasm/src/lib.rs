//! wasm-bindgen bridge of the structure graph of `mzlab-feat` (`chem::grafo`): the SMILES of the parent gives its formula, its functional groups (the
//! preconditions of `regole_tp/2`) and the formulas of the products of its cleavages (the `fragments` of the hypotheses). A `.wasm` of its own, so the engine of
//! the proofs (`mzlab-feat-wasm`) does not carry it: the page loads it when the user gives a SMILES. Errors are JSON `{"error_key", "params"}` strings.
use mzlab_core::error::Error;
use mzlab_core::json::Out;
use wasm_bindgen::prelude::*;

/// Stable version of the bridge: the page checks that the loaded `.wasm` is the one it was built with.
const BRIDGE: &str = "mzfeat-grafo/3";

fn js_err(e: Error) -> JsValue {
    let mut o = Out::default();
    o.raw("{\"error_key\":").str(e.key).raw(",\"params\":{");
    for (i, (k, v)) in e.params.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(k).raw(":").str(v);
    }
    o.raw("}}");
    JsValue::from_str(&o.0)
}

#[wasm_bindgen]
pub fn version() -> String {
    format!("{BRIDGE} feat={}", mzlab_feat::FEAT_VERSION)
}

/// `grafo/1` of a SMILES: `{"formula", "atoms", "groups", "fragments", "cuts"}` (see `mzlab_feat::chem::grafo::analyse_json`).
#[wasm_bindgen]
pub fn analyse(smiles: &str) -> std::result::Result<String, JsValue> {
    mzlab_feat::chem::grafo::analyse_json(smiles).map_err(js_err)
}

/// `frag/1` of a V2000 molfile: the two pieces of the bond between atoms `a` and `b` (0-based positions in the file), or every cut outside the rings
/// when either is negative (see `mzlab_feat::chem::frammenti::fragments_json`).
#[wasm_bindgen]
pub fn fragments(molfile: &str, a: i32, b: i32) -> std::result::Result<String, JsValue> {
    mzlab_feat::chem::frammenti::fragments_json(molfile, a, b).map_err(js_err)
}

/// `parz/1` of a V2000 molfile and a delta «+O -H2»: formula, mass and m/z of [M+H]+ of the partial structure, `level` 3
/// (see `mzlab_feat::chem::frammenti::partial_json`).
#[wasm_bindgen]
pub fn partial(molfile: &str, delta: &str) -> std::result::Result<String, JsValue> {
    mzlab_feat::chem::frammenti::partial_json(molfile, delta).map_err(js_err)
}
