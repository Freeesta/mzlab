//! The interpretation of the rows of a table (`mzlab_feat::interp`): the `.mzfeat` is read by offsets and written again, with the block `interp`, to a
//! sink. Built only into the `.wasm` of the feature `interp`, loaded by the page when a table is open and its interpretation is asked for.

use mzlab_core::elements::parse_formula;
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{Out, Value};
use mzlab_feat::chem::positions::Pol;
use mzlab_feat::interp::filters::DecoyParams;
use mzlab_feat::interp::json::filters_json;
use mzlab_feat::interp::ms2link::LinkParams;
use mzlab_feat::interp::refilter::{refilter, FilterParams};
use mzlab_feat::interp::reposition::reposition;
use mzlab_feat::interp::review;
use mzlab_feat::interp::site::SiteParams;
use mzlab_feat::legacy::series;
use mzlab_feat::sortc::SortC;
use mzlab_feat::store::{read_table_from, Slice};

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
    hyps.sortc_by_key(|h| h.0);
    let hyps: Vec<_> = hyps.into_iter().map(|h| h.1).collect();
    let reg = refilter(&mut Slice(tp), tp.len() as u64, &hyps, &filter_params(get))?;
    Ok(filters_json(&reg))
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

/// `tp.review` in the browser and natively: the `review/1` of the decisions `tpdec/1` over the table `tp`.
pub fn review_text(tp: &[u8], decisions: &str) -> Result<String> {
    review::review_text(tp, decisions)
}
