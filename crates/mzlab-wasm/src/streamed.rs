//! `build_mzidx_streamed`: the `.mzidx` of an mzML from ONE pass over the bytes its copy to OPFS already reads (`mzidx::build_mzml_streamed`).
//! `next(offset, length)` gives the bytes in order as they arrive, `read(offset, length)` the same file at any offset (its head, later rounds, fallback), `table()` the
//! eight inputs of `build_mzidx` (what `prepare_upload` answers) once the copy is done. Same bytes as `build_mzidx`.

use js_sys::{Array, Function, Uint8Array};
use mzlab_core::{budget::MemBudget, mzidx};
use wasm_bindgen::prelude::*;

use crate::{courier, error_json, f64s, js_blocks, natives, Accorpa};

/// The scan table from the first eight items of `prepare_upload`'s answer (levels, polarities, rt, tic, rows, source, run, hr1).
fn table_of(a: &Array) -> Option<mzidx::ScanTable> {
    let bytes = |k: u32| a.get(k).dyn_into::<Uint8Array>().ok().map(|u| u.to_vec());
    Some(mzidx::ScanTable {
        level: bytes(0)?,
        polarity: bytes(1)?.iter().map(|&b| b as i8).collect(),
        rt: f64s(&bytes(2)?),
        tic: f64s(&bytes(3)?),
        scans_json: bytes(4)?,
        source_json: a.get(5).as_string()?,
        run_json: a.get(6).as_string()?,
        hr1: a.get(7).as_bool()?,
        declared: 0,
    })
}

/// `build_mzidx` reading the mzML once as it is copied: the arguments of `build_mzidx`, with `next` (the stream), `read` (the file) and `table`
/// in place of the scan table. The statistics gain `one_pass` (false: pass 1 did not agree with the table and the file was read again).
#[wasm_bindgen]
#[allow(clippy::too_many_arguments)]
pub fn build_mzidx_streamed(
    size: f64,
    next: Function,
    read: Function,
    table: Function,
    write: Function,
    budget_mb: u32,
    spill_put: Option<Function>,
    spill_get: Option<Function>,
    cap_mb: f64,
    aiuti: JsValue,
) -> Result<String, JsValue> {
    let budget = (budget_mb as usize) << 20;
    let mut out = Accorpa::flussi(natives::JsOutput { write });
    let mut spill = spill_put
        .zip(spill_get)
        .map(|(put, get)| Accorpa::new(natives::JsSpill { put, get }));
    let mut ask = || -> mzlab_core::Result<mzidx::ScanTable> {
        let bad = || mzlab_core::Error::new("err.file.index").with("detail", "scan table");
        let a: Array = table
            .call0(&JsValue::NULL)
            .ok()
            .and_then(|v| v.dyn_into().ok())
            .ok_or_else(bad)?;
        let t = table_of(&a).ok_or_else(bad)?;
        // the memory check of build_mzidx, once the table is known
        mzlab_core::estimate::check_index(
            &MemBudget::from_request(cap_mb),
            t.level.len(),
            t.scans_json.len(),
            budget,
        )?;
        Ok(t)
    };
    let (st, one) = mzidx::build_mzml_streamed(
        js_blocks(next, size as u64),
        js_blocks(read, size as u64),
        &mut ask,
        &mut out,
        budget,
        spill
            .as_mut()
            .map(|s| s as &mut dyn mzlab_core::mzidx_spill::Spill),
        &mut courier::JsCorriere::new(&aiuti),
    )
    .map_err(|e| error_json(&e))?;
    out.fine().map_err(|e| error_json(&e))?;
    Ok(format!(
        "{{\"scans\":{},\"peaks\":{},\"sections\":{},\"passes\":{},\"bytes\":{},\"one_pass\":{one}}}",
        st.scans, st.peaks, st.sections, st.passes, st.bytes
    ))
}
