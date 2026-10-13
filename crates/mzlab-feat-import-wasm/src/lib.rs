//! wasm-bindgen bridge of the readers of external tables of `mzlab-feat` (`import`): the page stages the files with `add` (one copy each, freed by `build`)
//! and `build` gives ONE table as JSON. A `.wasm` of its own, loaded only when the user opens a table of mzMine, FBMN/GNPS2 or MS-DIAL.
//! Errors are JSON `{"error_key", "params"}` strings (`columns`, `empty`, `noTable`).
use mzlab_core::json::Out;
use mzlab_feat::import::{self, ImportError, InFile};
use std::cell::RefCell;
use wasm_bindgen::prelude::*;

/// Stable version of the bridge: the page checks that the loaded `.wasm` is the one it was built with.
const BRIDGE: &str = "mzfeat-import/1";

thread_local! {
    static STAGED: RefCell<Vec<InFile>> = const { RefCell::new(Vec::new()) };
}

fn js_err(e: ImportError) -> JsValue {
    let mut o = Out::default();
    o.raw("{\"error_key\":")
        .str(e.key)
        .raw(",\"params\":{\"message\":")
        .str(&e.msg)
        .raw("}}");
    JsValue::from_str(&o.0)
}

#[wasm_bindgen]
pub fn version() -> String {
    format!("{BRIDGE} feat={}", mzlab_feat::FEAT_VERSION)
}

/// The kind of a file from its name and the beginning of its text (`mgf`, `msdial`, `gnps-library`, `gnps-network`, `mzmine-edges`, `mzmine-quant`,
/// `mzmine-annotations`, `mzmine-full`, or an empty string).
#[wasm_bindgen]
pub fn detect(name: &str, head: &str) -> String {
    import::detect(name, head).to_string()
}

/// Stages one file of the table (its SHA-256 is kept in the table).
#[wasm_bindgen]
pub fn add(name: &str, sha256: &str, text: String) {
    STAGED.with(|s| {
        s.borrow_mut().push(InFile {
            name: name.into(),
            text,
            sha256: sha256.into(),
        })
    });
}

/// ONE table (JSON, the model of the untargeted view) from the staged files; `pol` 1, -1 or 0, `max_features` 0 = no limit. The staged files are dropped.
#[wasm_bindgen]
pub fn build(pol: i32, max_features: u32) -> std::result::Result<String, JsValue> {
    let files = STAGED.with(|s| std::mem::take(&mut *s.borrow_mut()));
    import::build(&files, pol, max_features as usize)
        .map(|t| import::table_json(&t))
        .map_err(js_err)
}

/// Drops the staged files without building (the user closed the dialog).
#[wasm_bindgen]
pub fn clear() {
    STAGED.with(|s| s.borrow_mut().clear());
}

/// Does the formula explain the precursor? `{"ok": true|false|null, "ppm", "adduct"}`; `precursor` NaN = none.
#[wasm_bindgen]
pub fn check_structure(
    formula: &str,
    precursor: f64,
    adduct: &str,
    pol: i32,
    tol_ppm: f64,
) -> String {
    let c = import::check_structure(
        formula,
        (!precursor.is_nan()).then_some(precursor),
        adduct,
        pol,
        tol_ppm,
    );
    let mut o = Out::default();
    o.raw("{\"ok\":").raw(match c.ok {
        Some(true) => "true",
        Some(false) => "false",
        None => "null",
    });
    if let Some(p) = c.ppm {
        o.raw(",\"ppm\":").num(p);
    }
    if let Some(a) = &c.adduct {
        o.raw(",\"adduct\":").str(a);
    }
    o.raw("}");
    o.0
}

/// «M+H» → «[M+H]+».
#[wasm_bindgen]
pub fn norm_adduct(a: &str) -> String {
    import::norm_adduct(a)
}

/// The name of a library entry without prefix, «|…», suffix, accession and energy.
#[wasm_bindgen]
pub fn clean_name(name: &str) -> String {
    import::clean_name(name)
}

/// Monoisotopic mass of a plain formula, NaN when it cannot be read.
#[wasm_bindgen]
pub fn formula_mass(f: &str) -> f64 {
    import::formula_mass(f).unwrap_or(f64::NAN)
}

/// A GNPS2 propagated name: never a source.
#[wasm_bindgen]
pub fn is_propagated(name: &str) -> bool {
    import::is_propagated(name)
}

/// A number from a cell (decimal comma, «na», «-» as missing), NaN when it is missing.
#[wasm_bindgen]
pub fn num(v: &str) -> f64 {
    import::num(v).unwrap_or(f64::NAN)
}
