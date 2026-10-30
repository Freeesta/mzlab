//! `spectra.insilico` in the browser (`flow-web-insilico.js`): the two exports of the `attributio-all` .wasm that read the candidate package by mass window.
//! Both are thin: the text is `mzlab_feat::flow_insilico`, the function `flow run` calls.

use crate::js_err;
use js_sys::{Function, Uint8Array};
use mzlab_core::error::Error;
use mzlab_core::json::Out;
use wasm_bindgen::prelude::*;

/// `spectra.insilico` in the browser, first step: the files of the pieces of the candidate package (`catalogue` = its `catalogo.json`) that the queries of the MGF need, as a JSON
/// array of names (`mzlab_feat::flow_insilico::pieces_needed`). The page inflates exactly those and hands them to `flow_insilico`.
#[wasm_bindgen]
pub fn flow_insilico_pieces(
    params_json: &str,
    name: &str,
    mgf: &[u8],
    catalogue: &[u8],
) -> Result<String, JsValue> {
    let params = mzlab_feat::flow_ms2::json_params(params_json).map_err(js_err)?;
    let files = mzlab_feat::flow_insilico::pieces_needed(&params, (name, mgf), catalogue)
        .map_err(js_err)?;
    let mut o = Out::default();
    o.raw("[");
    for (i, f) in files.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(f);
    }
    Ok(o.raw("]").0.clone())
}

/// `spectra.insilico` in the browser: the text `flow run` writes (`mzlab_feat::flow_insilico::insilico_text`). `suspects` is a CSV list, a `screen/1` text or the catalogue of the
/// package; for a package `piece(file) -> Uint8Array` gives the inflated CSV of a piece the page has already checked (only the ones `flow_insilico_pieces` named are asked for).
/// Errors are JSON `{"error_key","params"}` texts.
#[wasm_bindgen]
pub fn flow_insilico(
    params_json: &str,
    name: &str,
    mgf: &[u8],
    suspects: &[u8],
    piece: Function,
) -> Result<String, JsValue> {
    let params = mzlab_feat::flow_ms2::json_params(params_json).map_err(js_err)?;
    let mut get = |file: &str| -> Result<Vec<u8>, Error> {
        let missing = || Error::new("err.flow.insilico.piece").with("file", file);
        let v = piece
            .call1(&JsValue::NULL, &JsValue::from_str(file))
            .map_err(|_| missing())?;
        if v.is_undefined() || v.is_null() {
            return Err(missing());
        }
        Ok(Uint8Array::new(&v).to_vec())
    };
    mzlab_feat::flow_insilico::insilico_text(&params, (name, mgf), suspects, &mut get)
        .map_err(js_err)
}
