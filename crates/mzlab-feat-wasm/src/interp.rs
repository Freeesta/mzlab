//! The interpretation of the rows of a table (`mzlab_feat::interp`): the `.mzfeat` is read by offsets and written again, with the block `interp`, to a
//! sink. Built only into the `.wasm` of the feature `interp`, loaded by the page when a table is open and its interpretation is asked for.

use js_sys::Function;
use mzlab_core::elements::parse_formula;
use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::Index;
use mzlab_core::json::{Out, Value};
use mzlab_feat::chem::posizioni::Pol;
use mzlab_feat::interp::filtri::DecoyParams;
use mzlab_feat::interp::json::filtri_json;
use mzlab_feat::interp::ms2link::LinkParams;
use mzlab_feat::interp::refilter::{refilter, FilterParams};
use mzlab_feat::interp::reposition::reposition;
use mzlab_feat::interp::review;
use mzlab_feat::interp::run::{run_to, run_to_with};
use mzlab_feat::interp::sito::SiteParams;
use mzlab_feat::legacy::series;
use mzlab_feat::store::{read_table_from, Slice};
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

/// The thresholds of `tp.filter` over `FilterParams::default()`.
pub fn filter_params(get: &dyn Fn(&str) -> Option<f64>) -> FilterParams {
    let d = FilterParams::default();
    FilterParams {
        tol_min: get("tol_min").unwrap_or(d.tol_min),
        tol_max: get("tol_max").unwrap_or(d.tol_max),
        decoy: DecoyParams {
            step: get("decoy_step").unwrap_or(d.decoy.step),
            k: get("decoy_k").map_or(d.decoy.k, |v| v as u32),
            ppm: get("decoy_ppm").unwrap_or(d.decoy.ppm),
        },
    }
}

/// The thresholds of `tp.position` over `SiteParams::default()`.
pub fn site_params(get: &dyn Fn(&str) -> Option<f64>) -> SiteParams {
    let d = SiteParams::default();
    SiteParams {
        match_ppm: get("match_ppm").unwrap_or(d.match_ppm),
        min_rel: get("min_rel").unwrap_or(d.min_rel),
        min_informative: get("min_informative").map_or(d.min_informative, |v| v as usize),
        margin: get("margin").unwrap_or(d.margin),
        ll_support: get("ll_support").unwrap_or(d.ll_support),
        ll_against: get("ll_against").unwrap_or(d.ll_against),
    }
}

/// `filtri/1` text of the `.mzfeat` `tp` (with the block `interp`) and the `ipotesi/1` `hypotheses`, under the thresholds `get` gives (`tp.filter@2`).
pub fn refilter_text(
    get: &dyn Fn(&str) -> Option<f64>,
    tp: &[u8],
    hypotheses: &str,
) -> Result<String> {
    let (hyps, _, _) = series::registry(hypotheses)
        .map_err(|e| Error::new("error.interp.failed").with("detail", e))?;
    let mut hyps: Vec<_> = hyps.into_iter().collect();
    hyps.sort_by_key(|h| h.0);
    let hyps: Vec<_> = hyps.into_iter().map(|h| h.1).collect();
    let reg = refilter(&mut Slice(tp), tp.len() as u64, &hyps, &filter_params(get))?;
    Ok(filtri_json(&reg))
}

/// The polarity of the series, from the recipe in the meta of the table: the recipe is text (`detect=Params { .. polarity: Some(-1) .. }`), positive when it names none.
fn polarity_of(tp: &[u8]) -> Result<Pol> {
    let (_, meta) = read_table_from(&mut Slice(tp), tp.len() as u64)?;
    let recipe = meta.get("recipe").and_then(Value::str).unwrap_or("");
    Ok(if recipe.contains("polarity: Some(-1)") {
        Pol::Neg
    } else {
        Pol::Pos
    })
}

/// `sites/1` text of the `.mzfeat` `tp` (with the block `interp`) for the parent (its SMILES and neutral formula), under the thresholds `get` gives (`tp.position`).
pub fn position_text(
    get: &dyn Fn(&str) -> Option<f64>,
    tp: &[u8],
    smiles: &str,
    formula: &str,
) -> Result<String> {
    if smiles.is_empty() {
        return Err(Error::new("err.flow.position.smiles"));
    }
    let neutral = parse_formula(formula)?;
    let rows = reposition(
        &mut Slice(tp),
        tp.len() as u64,
        (smiles, &neutral),
        polarity_of(tp)?,
        &site_params(get),
    )?;
    let mut o = Out::default();
    o.raw("{\"schema\":\"sites/1\",\"parent\":")
        .str(smiles)
        .raw(",\"rows\":[");
    for (i, r) in rows.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"id\":")
            .int(i64::from(r.id))
            .raw(",\"recomputed\":")
            .raw(if r.recomputed { "true" } else { "false" });
        o.raw(",\"site\":").raw(&r.site).raw("}");
    }
    o.raw("]}");
    Ok(o.0)
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

/// `tp.review` in the browser and natively: the `review/1` of the decisions `tpdec/1` over the table `tp`.
pub fn review_text(tp: &[u8], decisions: &str) -> Result<String> {
    review::review_text(tp, decisions)
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
/// an empty `adducts` takes the default of the polarity of the products (`mzlab_feat::export_inclusione_tp`, the function `flow run` calls).
#[wasm_bindgen]
pub fn flow_inclusion(params: &str, products: &[u8]) -> std::result::Result<String, JsValue> {
    let set = getter(params)?;
    let adducts = set.get("adducts").and_then(Value::str);
    let half = set.get("rt_window").and_then(Value::f64).unwrap_or(0.5);
    mzlab_feat::export_inclusione_tp::inclusion_csv_of(products, adducts, half).map_err(js_err)
}
