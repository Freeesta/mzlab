//! JS exports of `interp`; the plain functions are `mzlab_ops::run::feat::interp`.

use crate::interp::{link_params, position_text, refilter_text, review_text};
use crate::{js_err, to_sink, JsReader};
use js_sys::Function;
use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::Index;
use mzlab_core::json::Value;
use mzlab_feat::interp::run::{run_to, run_to_with};
use wasm_bindgen::prelude::*;

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

fn getter(params: &str) -> std::result::Result<Value, JsValue> {
    mzlab_core::json::parse(params.as_bytes()).map_err(js_err)
}

/// `tp.filter@2` in the browser: the `filtri/1` of the table `tp` under the thresholds in `params` (a JSON object) and the hypotheses of the parent.
#[wasm_bindgen]
pub fn tp_refilter(
    tp: &[u8],
    hypotheses: &str,
    params: &str,
) -> std::result::Result<String, JsValue> {
    let set = getter(params)?;
    refilter_text(&|n| set.get(n).and_then(Value::f64), tp, hypotheses).map_err(js_err)
}

#[wasm_bindgen]
pub fn tp_review(tp: &[u8], decisions: &str) -> std::result::Result<String, JsValue> {
    review_text(tp, decisions).map_err(js_err)
}

/// `tp.position` in the browser: the `sites/1` of the table `tp` for the parent `smiles` / `formula` (neutral) under the thresholds in `params`.
#[wasm_bindgen]
pub fn tp_position(
    tp: &[u8],
    smiles: &str,
    formula: &str,
    params: &str,
) -> std::result::Result<String, JsValue> {
    let set = getter(params)?;
    position_text(&|n| set.get(n).and_then(Value::f64), tp, smiles, formula).map_err(js_err)
}

/// `export.inclusion` in the browser: the inclusion list of `products` (a JSON list, or the bytes of the `.mzfeat` of `tp.link`) under the step's parameters merged with its defaults;
/// an empty `adducts` takes the default of the polarity of the products (`mzlab_feat::export_inclusion_tp`, the function `flow run` calls).
#[wasm_bindgen]
pub fn flow_inclusion(params: &str, products: &[u8]) -> std::result::Result<String, JsValue> {
    let set = getter(params)?;
    let adducts = set.get("adducts").and_then(Value::str);
    let half = set.get("rt_window").and_then(Value::f64).unwrap_or(0.5);
    mzlab_feat::export_inclusion_tp::inclusion_csv_of(products, adducts, half).map_err(js_err)
}

/// `structure.render` in the browser: `drawings/1` of the `sites/1` file over the parent file (`mzlab_feat::interp::redraw::render_text`, the function
/// `flow run` calls).
#[wasm_bindgen]
pub fn flow_render(sites: &[u8], parent: &[u8]) -> std::result::Result<String, JsValue> {
    mzlab_feat::interp::redraw::render_text(sites, parent).map_err(js_err)
}
