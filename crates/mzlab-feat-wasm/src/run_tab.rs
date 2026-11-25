//! The Flow table steps that do not fit the `table` module, in the browser (`.wasm` `flowtab`): `table.drift` (`mzlab_feat::flow_drift::drift_text`), `qc.run_order` (`mzlab_feat::runorder::runorder_text`),
//! `report.methods` (`mzlab_feat::methods::methods_text`), `export.annotations` (`mzlab_feat::flow_export_annot`), `chrom.transitions` (`mzlab_feat::transitions`), `features.group` (`mzlab_feat::ionnet`), `features.companions` (`mzlab_feat::companions`), `features.union` (`mzlab_feat::union`) and `table.plausible` (`mzlab_feat::plausible::plausible_text`), the functions `flow run` calls.
//! Loaded by the page only when one of these steps runs.

use mzlab_feat::flow_ms2::json_params;
use wasm_bindgen::prelude::*;

/// The `deriva/1` text of `table` (the bytes of a `.mzfeat`) under the QC files of `samples` and the batches of `design`; `params_json` is the step's parameters merged with its defaults.
/// Errors are JSON `{"error_key","params"}` texts.
#[wasm_bindgen]
pub fn flow_drift(
    params_json: &str,
    table: &[u8],
    samples: &[u8],
    design: &[u8],
) -> std::result::Result<String, JsValue> {
    let params = json_params(params_json).map_err(crate::js_err)?;
    mzlab_feat::flow_drift::drift_text(&params, table, samples, design).map_err(crate::js_err)
}

/// The `ionnet/1` text of `table` (the bytes of a per-ion `.mzfeat`) and its warnings; `params_json` is the step's parameters merged with its defaults,
/// `budget_mb` the memory of the engine (0 = the default of the page). Errors are JSON `{"error_key","params"}` texts (`error.memoryBudget`, phase `group`).
#[wasm_bindgen]
pub fn flow_group(
    params_json: &str,
    table: &[u8],
    budget_mb: f64,
) -> std::result::Result<Grouped, JsValue> {
    let params = json_params(params_json).map_err(crate::js_err)?;
    let budget = mzlab_core::budget::MemBudget::from_request(budget_mb);
    let g = mzlab_feat::ionnet::group_text(&params, table, &budget).map_err(crate::js_err)?;
    Ok(Grouped(g))
}

/// What `flow_group` made: the text (taken once, not copied) and the `warn.*` keys of the record as a JSON array.
#[wasm_bindgen]
pub struct Grouped(mzlab_feat::ionnet::Grouped);

#[wasm_bindgen]
impl Grouped {
    pub fn take_text(&mut self) -> String {
        std::mem::take(&mut self.0.text)
    }
    pub fn warnings(&self) -> String {
        let mut o = mzlab_core::json::Out::default();
        o.raw("[");
        for (k, w) in self.0.warnings.iter().enumerate() {
            o.raw(if k > 0 { "," } else { "" }).str(w);
        }
        o.raw("]");
        o.0
    }
}

/// The `runorder/1` text of the sample table `samples`; `design` holds the batches (`design/1`), empty for none (the batch of the sample table). `params_json` is the step's parameters merged with its defaults.
/// Errors are JSON `{"error_key","params"}` texts.
#[wasm_bindgen]
pub fn flow_runorder(
    params_json: &str,
    samples: &[u8],
    design: &[u8],
) -> std::result::Result<String, JsValue> {
    let params = json_params(params_json).map_err(crate::js_err)?;
    let design = (!design.is_empty()).then_some(design);
    mzlab_feat::runorder::runorder_text(&params, samples, design).map_err(crate::js_err)
}

/// The Markdown methods text of `run`, a `run-report/2` text.
#[wasm_bindgen]
pub fn flow_methods(params_json: &str, run: &[u8]) -> std::result::Result<String, JsValue> {
    let params = json_params(params_json).map_err(crate::js_err)?;
    mzlab_feat::methods::methods_text(&params, run).map_err(crate::js_err)
}

/// The CSV of `export.annotations` for `table` (the bytes of a `.mzfeat`); the upstream files are packed as `manifest` (JSON `[{"port","name","len"}]`) over the concatenated bytes `blob`.
/// Errors are JSON `{"error_key","params"}` texts.
#[wasm_bindgen]
pub fn flow_export_annot(
    params_json: &str,
    table: &[u8],
    manifest: &str,
    blob: &[u8],
) -> std::result::Result<String, JsValue> {
    mzlab_feat::flow_export_annot::export_text_packed(params_json, table, manifest, blob)
        .map_err(crate::js_err)
}

/// The CSV of `chrom.transitions` for the transition table `table` (a CSV) over the mzML files packed as `manifest` (JSON `[{"name","len"}]`) over their
/// concatenated bytes `blob`, within `budget_mb` (0 = the default of the page). Errors are JSON `{"error_key","params"}` texts (`error.memoryBudget`, phase `read`).
#[wasm_bindgen]
pub fn flow_transitions(
    params_json: &str,
    table: &str,
    manifest: &str,
    blob: &[u8],
    budget_mb: f64,
) -> std::result::Result<String, JsValue> {
    let cap = mzlab_core::budget::MemBudget::from_request(budget_mb).cap();
    mzlab_feat::transitions::transitions_packed(params_json, table, manifest, blob, cap)
        .map_err(crate::js_err)
}

/// The CSV of `chrom.uv_peaks` over the mzML files packed as `manifest` (JSON `[{"name","len"}]`) over their concatenated bytes `blob`, within `budget_mb`
/// (0 = the default of the page). Errors are JSON `{"error_key","params"}` texts (`error.memoryBudget`, phase `read`; `err.pda.none`).
#[wasm_bindgen]
pub fn flow_uv_peaks(
    params_json: &str,
    manifest: &str,
    blob: &[u8],
    budget_mb: f64,
) -> std::result::Result<String, JsValue> {
    let cap = mzlab_core::budget::MemBudget::from_request(budget_mb).cap();
    mzlab_feat::uv_text::uv_packed(params_json, manifest, blob, cap).map_err(crate::js_err)
}

/// The `plausible/1` text of `table` (the bytes of a `.mzfeat`); `params_json` is the step's parameters merged with its defaults (the limits of `plausible::Limits`).
/// Errors are JSON `{"error_key","params"}` texts.
#[wasm_bindgen]
pub fn flow_plausible(params_json: &str, table: &[u8]) -> std::result::Result<String, JsValue> {
    let params = json_params(params_json).map_err(crate::js_err)?;
    mzlab_feat::plausible::plausible_text(&params, table).map_err(crate::js_err)
}

/// The `companions/1` text of `table` (the bytes of a unit-mass `.mzfeat`) and its warnings; `params_json` is the step's parameters merged with its defaults.
/// Errors are JSON `{"error_key","params"}` texts (`err.flow.companions.list`).
#[wasm_bindgen]
pub fn flow_companions(
    params_json: &str,
    table: &[u8],
) -> std::result::Result<Companioned, JsValue> {
    let params = json_params(params_json).map_err(crate::js_err)?;
    let m = mzlab_feat::companions::companions_text(&params, table).map_err(crate::js_err)?;
    Ok(Companioned(m))
}

/// What `flow_companions` made: the text (taken once, not copied) and the `warn.*` keys of the record as a JSON array.
#[wasm_bindgen]
pub struct Companioned(mzlab_feat::companions::Made);

#[wasm_bindgen]
impl Companioned {
    pub fn take_text(&mut self) -> String {
        std::mem::take(&mut self.0.text)
    }
    pub fn warnings(&self) -> String {
        let mut o = mzlab_core::json::Out::default();
        o.raw("[");
        for (k, w) in self.0.warnings.iter().enumerate() {
            o.raw(if k > 0 { "," } else { "" }).str(w);
        }
        o.raw("]");
        o.0
    }
}

/// The target list and the `union/1` document of the series tables: `names_json` is a JSON array of the table names (`<id>.mzfeat`), `lens_json` one byte length per table and
/// `tables` the `.mzfeat` bytes of all of them one after the other, in plan order; `params_json` is the step's parameters merged with its defaults.
/// Errors are JSON `{"error_key","params"}` texts (`err.union.param`, `err.union.table`, `err.union.empty`).
#[wasm_bindgen]
pub fn flow_union(
    params_json: &str,
    names_json: &str,
    lens_json: &str,
    tables: &[u8],
) -> std::result::Result<Joined, JsValue> {
    let params = json_params(params_json).map_err(crate::js_err)?;
    let list = |t: &str| mzlab_core::json::parse(t.as_bytes()).map(|v| v.arr().to_vec());
    let (names, lens) = (
        list(names_json)
            .map_err(|_| crate::js_err(mzlab_core::error::Error::new("err.union.request")))?,
        list(lens_json)
            .map_err(|_| crate::js_err(mzlab_core::error::Error::new("err.union.request")))?,
    );
    let mut at = 0usize;
    let mut series = Vec::new();
    for (n, l) in names.iter().zip(&lens) {
        let end = at + l.f64().unwrap_or(0.0) as usize;
        let bytes = tables.get(at..end).unwrap_or_default().to_vec();
        series.push((n.str().unwrap_or("").to_string(), bytes));
        at = end;
    }
    let m = mzlab_feat::union::union_text(&params, &series).map_err(crate::js_err)?;
    Ok(Joined(m))
}

/// What `flow_union` made: the CSV target list and the `union/1` text, each taken once (not copied).
#[wasm_bindgen]
pub struct Joined(mzlab_feat::union::Made);

#[wasm_bindgen]
impl Joined {
    pub fn take_csv(&mut self) -> String {
        std::mem::take(&mut self.0.csv)
    }
    pub fn take_doc(&mut self) -> String {
        std::mem::take(&mut self.0.doc)
    }
}
