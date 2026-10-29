//! `formula.ms1` in the browser: the rows of the features that read one mzML (`mzlab_feat::flow_formula_ms1::ms1_rows`) and their join into `formulas/1` (`ms1_join`), the
//! functions `flow run` calls. The page calls `flow_ms1_rows` once per file, in the order of the plan, and joins the rows in the order of the table.

use mzlab_feat::flow_formula_ms1::{ms1_join, ms1_rows};
use mzlab_feat::flow_ms2::json_params;
use wasm_bindgen::prelude::*;

/// The rows of the features whose largest area is in `mzml` (the `index`-th of the `names_json` array of file names), one per line as `position TAB object`. `features` is the CSV of
/// `export.table_csv`; the parameters are the ones of the step as JSON merged with its defaults. Errors are JSON `{"error_key","params"}` texts.
#[wasm_bindgen]
pub fn flow_ms1_rows(
    params_json: &str,
    features: &[u8],
    names_json: &str,
    index: usize,
    mzml: &[u8],
) -> std::result::Result<String, JsValue> {
    let params = json_params(params_json).map_err(crate::js_err)?;
    let names = json_params(&format!("{{\"n\":{names_json}}}")).map_err(crate::js_err)?;
    let names: Vec<&str> = names
        .first()
        .map(|(_, v)| v.arr())
        .unwrap_or(&[])
        .iter()
        .map(|v| v.str().unwrap_or(""))
        .collect();
    let rows = ms1_rows(&params, features, &names, (index, mzml)).map_err(crate::js_err)?;
    Ok(rows.iter().map(|(n, s)| format!("{n}\t{s}\n")).collect())
}

/// The `formulas/1` text from the objects of the features in the order of the table, one per line.
#[wasm_bindgen]
pub fn flow_ms1_join(rows: &str) -> String {
    let rows: Vec<String> = rows.lines().map(String::from).collect();
    ms1_join(&rows)
}
