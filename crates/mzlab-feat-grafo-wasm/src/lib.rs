//! wasm-bindgen bridge of the structure graph of `mzlab-feat` (`chem::graph`): the SMILES of the parent gives its formula, its functional groups (the
//! preconditions of `regole_tp/2`) and the formulas of the products of its cleavages (the `fragments` of the hypotheses). A `.wasm` of its own, so the engine of
//! the proofs (`mzlab-feat-wasm`) does not carry it: the page loads it when the user gives a SMILES. Errors are JSON `{"error_key", "params"}` strings.
use mzlab_core::error::Error;
use wasm_bindgen::prelude::*;

/// Stable version of the bridge: the page checks that the loaded `.wasm` is the one it was built with.
const BRIDGE: &str = "mzfeat-grafo/9";

fn js_err(e: Error) -> JsValue {
    JsValue::from_str(&e.to_json())
}

#[wasm_bindgen]
pub fn version() -> String {
    format!("{BRIDGE} feat={}", mzlab_feat::FEAT_VERSION)
}

/// `grafo/1` of a SMILES: `{"formula", "atoms", "groups", "fragments", "cuts"}` (see `mzlab_feat::chem::graph::analyse_json`).
#[wasm_bindgen]
pub fn analyse(smiles: &str) -> std::result::Result<String, JsValue> {
    mzlab_feat::chem::graph::analyse_json(smiles).map_err(js_err)
}

/// `frag/1` of a V2000 molfile: the two pieces of the bond between atoms `a` and `b` (0-based positions in the file), or every cut outside the rings
/// when either is negative (see `mzlab_feat::chem::fragments::fragments_json`).
#[wasm_bindgen]
pub fn fragments(molfile: &str, a: i32, b: i32) -> std::result::Result<String, JsValue> {
    mzlab_feat::chem::fragments::fragments_json(molfile, a, b).map_err(js_err)
}

/// `parz/1` of a V2000 molfile and a delta «+O -H2»: formula, mass and m/z of [M+H]+ of the partial structure, `level` 3b
/// (see `mzlab_feat::chem::fragments::partial_json`).
#[wasm_bindgen]
pub fn partial(molfile: &str, delta: &str) -> std::result::Result<String, JsValue> {
    mzlab_feat::chem::fragments::partial_json(molfile, delta).map_err(js_err)
}

/// `etich/1` of the structures and arrows of a drawing: formula with isotopes, charge, monoisotopic and nominal mass, m/z, and the change
/// across each arrow (see `mzlab_feat::chem::labels::labels_json`; the same as `mzlab-cli mol-label`).
#[wasm_bindgen]
pub fn labels(request: &str) -> std::result::Result<String, JsValue> {
    mzlab_feat::chem::labels::labels_json(request).map_err(js_err)
}

/// `sel/1` of a request `{"molfile", "part"}`: formula, charge, monoisotopic and average mass, m/z, heavy atoms, rings and aromatic rings of a selection
/// or of the whole drawing (see `mzlab_feat::chem::sel::sel_json`; the same as `mzlab-cli mol-sel`).
#[wasm_bindgen]
pub fn sel(request: &str) -> std::result::Result<String, JsValue> {
    mzlab_feat::chem::sel::sel_json(request).map_err(js_err)
}

/// `anal/1` of one drawn structure `{"atoms": [...]}`: the `etich/1` numbers plus average mass, mass % per element and the theoretical isotope pattern
/// (see `mzlab_feat::chem::analysis::analysis_json`; the same as `mzlab-cli mol-anal`).
#[wasm_bindgen]
pub fn analysis(request: &str) -> std::result::Result<String, JsValue> {
    mzlab_feat::chem::analysis::analysis_json(request).map_err(js_err)
}

/// `cut/1` of a request `{"molfile", "bonds", "mode", "cation", "gap"}`: the pieces of a cut across one or more bonds (homolytic, heterolytic or «+H»),
/// the atoms it changes and how far each piece moves (see `mzlab_feat::chem::cut::cut_json`; the same as `mzlab-cli mol-cut`).
#[wasm_bindgen]
pub fn cut(request: &str) -> std::result::Result<String, JsValue> {
    mzlab_feat::chem::cut::cut_json(request).map_err(js_err)
}

/// `mecc/1` of a request `{"molfile", "arrows", "lone"}`: whether each curved arrow of a mechanism starts at a lone pair, a bond or a radical and ends at
/// an atom or a bond, and whether the drawn lone pairs fit (see `mzlab_feat::chem::mech::mech_json`; the same as `mzlab-cli mol-mech`). Form only.
#[wasm_bindgen]
pub fn mech(request: &str) -> std::result::Result<String, JsValue> {
    mzlab_feat::chem::mech::mech_json(request).map_err(js_err)
}

/// `posiz/1` of a SMILES, a transformation («+O», «-CH2», «+O-2H», «+Cl-H», «+Br-H») and the MS2 of the product (`peaks_json` `[[mz,int],..]`): the classes of positions
/// the spectrum supports, excludes or cannot tell apart (see `mzlab_feat::chem::positions::positions_json`). `tol` in Da (`ppm` false) or ppm; `positive` is the polarity.
#[wasm_bindgen]
pub fn positions(
    smiles: &str,
    transform: &str,
    peaks_json: &str,
    tol: f64,
    ppm: bool,
    positive: bool,
) -> std::result::Result<String, JsValue> {
    mzlab_feat::chem::positions::positions_json(smiles, transform, peaks_json, tol, ppm, positive)
        .map_err(js_err)
}

/// `positions` with the `reatt/1` of the same parent (`reatt_json`, empty = none): the same `posiz/1` plus `prior`, the f(−) of every site as a «Prev.»
/// (see `mzlab_feat::chem::positions::positions_prior_json`). The values come in already computed, so this `.wasm` does not carry the engine of `mzlab-chem`.
#[wasm_bindgen]
pub fn positions_prior(
    smiles: &str,
    transform: &str,
    peaks_json: &str,
    tol: f64,
    ppm: bool,
    positive: bool,
    reatt_json: &str,
) -> std::result::Result<String, JsValue> {
    mzlab_feat::chem::positions::positions_prior_json(
        smiles, transform, peaks_json, tol, ppm, positive, reatt_json,
    )
    .map_err(js_err)
}
