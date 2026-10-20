//! wasm-bindgen bridge of the table of `mzlab-feat`: the view (normalisation and filters), the report and the files that leave the table (CSV full,
//! FBMN quantification table, MGF, SIRIUS `.ms`). A `.wasm` of its own, so the engine of the pipeline (`mzlab-feat-wasm`) does not carry it: the page loads
//! it when a table is shown or exported. The `.mzfeat` is read by offsets (`read(0, offset, length) -> Uint8Array`), the `campioni/1` and the parameters of the view
//! come as JSON text, the `.mzidx` of the files (only for MGF and `.ms`) as `read(k, offset, length)`. Errors are JSON `{"error_key", "params"}` strings.
#![forbid(unsafe_code)]
use js_sys::{Function, Uint8Array};
use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::Out;
use wasm_bindgen::prelude::*;

pub mod core;

/// Stable version of the bridge: the page checks that the loaded `.wasm` is the one it was built with.
const BRIDGE: &str = "mzfeat-table/1";

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

/// Version of the crate `mzlab-feat` inside this `.wasm` (`x.y.z`).
#[wasm_bindgen]
pub fn crate_version() -> String {
    mzlab_feat::CRATE_VERSION.to_string()
}

/// Bytes of file `k` held by JavaScript: `read(k, offset, length) -> Uint8Array`.
struct JsReader {
    read: Function,
    k: u32,
}

impl ReadAt for JsReader {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<()> {
        let io = || Error::new("error.io");
        let v = self.read.call3(&JsValue::NULL, &self.k.into(), &(off as f64).into(), &(buf.len() as f64).into()).map_err(|_| io())?;
        let a: Uint8Array = v.dyn_into().map_err(|_| io())?;
        if a.length() as usize != buf.len() {
            return Err(Error::new("error.badIndex"));
        }
        a.copy_to(buf);
        Ok(())
    }
}

fn table_reader(read: Function) -> JsReader {
    JsReader { read, k: 0 }
}

/// The view of a table: `{"digest", "righe":[id of every row kept], "fattori":[per file], "conteggi":{...}}`.
#[wasm_bindgen]
pub fn table_view(table_read: Function, size: f64, samples_json: &str, spec: &str) -> std::result::Result<String, JsValue> {
    core::view_json(&mut table_reader(table_read), size as u64, samples_json, spec).map_err(js_err)
}

/// The `report/1` of a table under the view `spec` (empty = the whole table).
#[wasm_bindgen]
pub fn table_report(table_read: Function, size: f64, samples_json: &str, spec: &str, table_sha256: &str) -> std::result::Result<String, JsValue> {
    core::report_json(&mut table_reader(table_read), size as u64, samples_json, spec, table_sha256).map_err(js_err)
}

/// The `tptab/1` of a table: the rows of the JSON list `rows_json` (indices of the table), or, empty, the rows of the view `tp/1`; areas, t0 and blank.
#[wasm_bindgen]
pub fn tp_table(table_read: Function, size: f64, samples_json: &str, rows_json: &str) -> std::result::Result<String, JsValue> {
    core::tp_table_json(&mut table_reader(table_read), size as u64, samples_json, rows_json).map_err(js_err)
}

/// `tp_table` with the conditions of the series: `campioni_json` = the `campioni/2` of the files, `infos_json` = `[{"row","neutral_formula","ion_formula","ppm"}]`.
/// With two or more conditions the `tptab/1` carries `gruppi` (`gruppi/1`); with fewer it is the one of `tp_table`.
#[wasm_bindgen]
pub fn tp_table_cond(table_read: Function, size: f64, samples_json: &str, rows_json: &str, campioni_json: &str, infos_json: &str) -> std::result::Result<String, JsValue> {
    core::tp_table_cond_json(&mut table_reader(table_read), size as u64, samples_json, rows_json, campioni_json, infos_json).map_err(js_err)
}

/// The `campioni/1` proposed from the names of the files (`{"files":[{"name","sha256"}]}`).
#[wasm_bindgen]
pub fn table_samples(files_json: &str) -> std::result::Result<String, JsValue> {
    core::samples_json(files_json).map_err(js_err)
}

/// The `qc/1` of a table under the view `spec` (empty = the whole table).
#[wasm_bindgen]
pub fn table_qc(table_read: Function, size: f64, samples_json: &str, spec: &str) -> std::result::Result<String, JsValue> {
    core::qc_json(&mut table_reader(table_read), size as u64, samples_json, spec).map_err(js_err)
}

/// A table file: `kind` is `full`, `full_mzmine`, `quant`, `csv_semicolon` or `xlsx_rows` (see `core::table_file`).
#[wasm_bindgen]
pub fn export_table(table_read: Function, size: f64, samples_json: &str, spec: &str, table_sha256: &str, polarity: i32, kind: &str) -> std::result::Result<String, JsValue> {
    core::table_file(&mut table_reader(table_read), size as u64, samples_json, spec, table_sha256, polarity, kind).map_err(js_err)
}

/// An MS2 file: `kind` is `mgf` (FBMN) or `sirius` (`.ms`). The spectra are read from the `.mzidx` of the file that holds the best MS2 of each row
/// (`index_read(k, offset, length)`, `index_sizes[k]` bytes).
#[wasm_bindgen]
#[allow(clippy::too_many_arguments)]
pub fn export_ms2(table_read: Function, size: f64, samples_json: &str, spec: &str, table_sha256: &str, polarity: i32, kind: &str, index_read: Function, index_sizes: Vec<f64>) -> std::result::Result<String, JsValue> {
    let open = |k: usize| {
        let size = *index_sizes.get(k).ok_or_else(|| Error::new("error.badIndex"))?;
        Index::open(JsReader { read: index_read.clone(), k: k as u32 }, size as u64)
    };
    core::ms2_file(&mut table_reader(table_read), size as u64, samples_json, spec, table_sha256, polarity, kind, open).map_err(js_err)
}

/// The `polpair/1` of two `.mzfeat` of the same files, one per polarity: the rows of the positive table that have the same neutral mass, RT and course in
/// the negative one. Files that differ are `error.polpair.files`; `params_json` may be empty.
#[wasm_bindgen]
pub fn cross_polarity(read_pos: Function, size_pos: f64, read_neg: Function, size_neg: f64, params_json: &str) -> std::result::Result<String, JsValue> {
    mzlab_feat::polpair::cross_polarity(&mut table_reader(read_pos), size_pos as u64, &mut table_reader(read_neg), size_neg as u64, params_json).map_err(js_err)
}

/// The `riga/1` of the row with `id`: summaries, identity and one cell per file. An id the table does not have is `error.rowNotFound`.
#[wasm_bindgen]
pub fn table_row(table_read: Function, size: f64, samples_json: &str, id: u32) -> std::result::Result<String, JsValue> {
    core::row_json(&mut table_reader(table_read), size as u64, samples_json, id).map_err(js_err)
}

/// The `riga_tracce/1` of the row with `id`: XIC of every file, MS1 at the best apex and MS2, read from the `.mzidx` of the series
/// (`index_read(k, offset, length)`, `index_sizes[k]` bytes) within `ppm` of the m/z of the row.
#[wasm_bindgen]
#[allow(clippy::too_many_arguments)]
pub fn table_row_traces(table_read: Function, size: f64, samples_json: &str, id: u32, index_read: Function, index_sizes: Vec<f64>, ppm: f64) -> std::result::Result<String, JsValue> {
    let open = |k: usize| {
        let size = *index_sizes.get(k).ok_or_else(|| Error::new("error.badIndex"))?;
        Index::open(JsReader { read: index_read.clone(), k: k as u32 }, size as u64)
    };
    core::row_traces_json(&mut table_reader(table_read), size as u64, samples_json, id, ppm, open).map_err(js_err)
}

/// `campioni/2`: `op` is `proponi`, `valida`, `da_legacy` or `per_tp`; JSON in, JSON out.
#[wasm_bindgen]
pub fn campioni(op: &str, input: &str) -> std::result::Result<String, JsValue> {
    core::campioni_json(op, input).map_err(js_err)
}
