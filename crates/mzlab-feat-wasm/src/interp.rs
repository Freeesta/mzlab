//! The interpretation of the rows of a table (`mzlab_feat::interp`): the `.mzfeat` is read by offsets and written again, with the block `interp`, to a
//! sink. Built only into the `.wasm` of the feature `interp`, loaded by the page when a table is open and its interpretation is asked for.

use js_sys::Function;
use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::Index;
use mzlab_core::json::Value;
use mzlab_feat::interp::ms2link::LinkParams;
use mzlab_feat::interp::run::{run_to, run_to_with};
use wasm_bindgen::prelude::*;

use crate::{js_err, to_sink, JsReader};

/// The `.mzfeat` of `table_read` (`size` bytes) with the block `interp`, written to `write(offset, bytes)`: `{"bytes", "sha256"}`. The `.mzidx` of the
/// series come as `index_read(k, offset, length)` with `index_sizes[k]` bytes; `infusion` is the position of the infusion file in the series (-1 when there
/// is none). `parent_json` is the parent of the series and `registry` the `ipotesi/1` JSON (may be empty).
#[wasm_bindgen]
#[allow(clippy::too_many_arguments)]
pub fn interp_build(
    table_read: Function,
    size: f64,
    parent_json: &str,
    registry: &str,
    polarity: i32,
    index_read: Function,
    index_sizes: Vec<f64>,
    infusion: i32,
    write: Function,
) -> std::result::Result<String, JsValue> {
    let open = |k: usize| -> Result<Index<JsReader>> {
        let size = *index_sizes
            .get(k)
            .ok_or_else(|| Error::new("error.badIndex"))?;
        Index::open(
            JsReader {
                read: index_read.clone(),
                k: k as u32,
            },
            size as u64,
        )
    };
    let mut table = JsReader {
        read: table_read,
        k: 0,
    };
    let mut inf = if infusion >= 0 {
        Some(open(infusion as usize).map_err(js_err)?)
    } else {
        None
    };
    to_sink(write, |sink| {
        run_to(
            &mut table,
            size as u64,
            parent_json,
            registry,
            polarity.signum() as i8,
            &mut |k| open(k),
            inf.as_mut(),
            sink,
        )
    })
}

/// The `interp/1` of the row with `id` of the `.mzfeat` (`size` bytes); `null` when the row was not interpreted or the file has no block.
#[wasm_bindgen]
pub fn table_interp(
    table_read: Function,
    size: f64,
    id: u32,
) -> std::result::Result<String, JsValue> {
    let mut table = JsReader {
        read: table_read,
        k: 0,
    };
    let row = mzlab_feat::interp::store::read_row(&mut table, size as u64, id).map_err(js_err)?;
    Ok(row.unwrap_or_else(|| "null".into()))
}

/// The thresholds of `tp.link` over `LinkParams::default()`; `get` gives the value a plan or a job set for a name.
pub fn link_params(get: &dyn Fn(&str) -> Option<f64>) -> LinkParams {
    let d = LinkParams::default();
    LinkParams {
        select_ppm: get("select_ppm").unwrap_or(d.select_ppm),
        purity_ppm: get("purity_ppm").unwrap_or(d.purity_ppm),
        purity_min: get("purity_min").unwrap_or(d.purity_min),
        match_ppm: get("match_ppm").unwrap_or(d.match_ppm),
        iso_corr: get("iso_corr").unwrap_or(d.iso_corr),
        iso_min_scans: get("iso_min_scans").map_or(d.iso_min_scans, |v| v as usize),
        iso_fragments: get("iso_fragments").map_or(d.iso_fragments, |v| v as usize),
    }
}

/// `tp.link@2` after the proofs, in the browser: `interp_build` over the table `table` that `tp_link_evidence` made, with the thresholds of the step
/// (`params`, a JSON object; a missing name keeps the default). `parent`, `registry`, `polarity` and `infusion` are what `tp_link_evidence` gave.
#[wasm_bindgen]
#[allow(clippy::too_many_arguments)]
pub fn tp_link_interp(
    table: &[u8],
    parent: &str,
    registry: &str,
    polarity: i32,
    params: &str,
    index_read: Function,
    index_sizes: Vec<f64>,
    infusion: i32,
    write: Function,
) -> std::result::Result<String, JsValue> {
    let set = mzlab_core::json::parse(params.as_bytes()).map_err(js_err)?;
    let link = link_params(&|n| set.get(n).and_then(Value::f64));
    let open = |k: usize| -> Result<Index<JsReader>> {
        let size = *index_sizes
            .get(k)
            .ok_or_else(|| Error::new("error.badIndex"))?;
        Index::open(
            JsReader {
                read: index_read.clone(),
                k: k as u32,
            },
            size as u64,
        )
    };
    let mut inf = if infusion >= 0 {
        Some(open(infusion as usize).map_err(js_err)?)
    } else {
        None
    };
    to_sink(write, |sink| {
        run_to_with(
            &mut mzlab_feat::store::Slice(table),
            table.len() as u64,
            (parent, registry, polarity.signum() as i8),
            &link,
            &mut |k| open(k),
            inf.as_mut(),
            sink,
        )
    })
}
