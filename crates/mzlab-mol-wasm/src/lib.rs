//! wasm-bindgen bridge of the molecule kit of Draw on the mol/1 record (`mzlab-mol` linked once): `check` gives `check/1`, what is wrong in the
//! drawing; `substructure` gives `sub/1` (`subdb/1` over a structure list), where a drawn moiety occurs; `cip` gives `cip/1`, its R/S and E/Z;
//! `sdf` gives the `sdf/1` file (molfile or SDF with data items). A `.wasm` of its own (`molkit`), so the grafo one does not carry the record;
//! the CLI (`mol-check`, `mol-search`, `mol-cip`, `mol-export`) and the app (`feat/grafo`, kinds `check`, `sub`, `cip`, `sdf`) run the same
//! `mzlab_feat::chem` functions. `flow_moiety` is the Flow step `structure.moiety` (`moiety::moiety_text`, the function `flow run` calls).
//! Errors are JSON `{"error_key", "params"}` strings.
use mzlab_core::error::Error;
use wasm_bindgen::prelude::*;

/// Stable version of the bridge: the page checks that the loaded `.wasm` is the one it was built with.
const BRIDGE: &str = "molkit/1";

fn js_err(e: Error) -> JsValue {
    JsValue::from_str(&e.to_json())
}

#[wasm_bindgen]
pub fn version() -> String {
    BRIDGE.to_string()
}

/// `check/1` of a request `{"molfile"}`, the same bytes as `mzlab-cli mol-check`.
#[wasm_bindgen]
pub fn check(request: &str) -> std::result::Result<String, JsValue> {
    mzlab_feat::chem::check::check_json(request).map_err(js_err)
}

/// `sub/1` of a request `{"query", "targets"}`, or `subdb/1` of `{"query", "db"}`: the same bytes as `mzlab-cli mol-search --sub`.
#[wasm_bindgen]
pub fn substructure(request: &str) -> std::result::Result<String, JsValue> {
    mzlab_feat::chem::substructure::search_json(request).map_err(js_err)
}

/// `sdf/1` text of a request `{"molfile", "format", "ion", "name"}`, the same bytes as `mzlab-cli mol-export`.
#[wasm_bindgen]
pub fn sdf(request: &str) -> std::result::Result<String, JsValue> {
    mzlab_feat::chem::sdf::sdf_json(request).map_err(js_err)
}

/// `cip/1` of a request `{"molfile"}`, the same bytes as `mzlab-cli mol-cip`.
#[wasm_bindgen]
pub fn cip(request: &str) -> std::result::Result<String, JsValue> {
    mzlab_feat::chem::cip::cip_json(request).map_err(js_err)
}

/// `hilite/1` of a request `{"molfile", "world", "selected", "layers"}`: the atoms and bonds a piece of evidence colours on the drawing, the same
/// bytes as `mzlab-cli mol-hilite`.
#[wasm_bindgen]
pub fn hilite(request: &str) -> std::result::Result<String, JsValue> {
    mzlab_feat::chem::hilite::hilite_json(request).map_err(js_err)
}

/// `structure.moiety`: the rows of the candidate table `table` that contain (or lack) the moiety; `params_json` is the step's parameters merged with
/// its defaults. The same bytes as `flow run`.
#[wasm_bindgen]
pub fn flow_moiety(params_json: &str, table: &[u8]) -> std::result::Result<String, JsValue> {
    let params = mzlab_core::json::parse(params_json.as_bytes())
        .map_err(|_| js_err(Error::new("error.sub.request")))?;
    mzlab_feat::chem::moiety::moiety_text(&params, table).map_err(js_err)
}
