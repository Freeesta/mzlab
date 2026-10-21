//! The native executor of `tp.position@1`: the position of the modification of every row that `tp.link` interpreted (the key `site` of `interp/1`),
//! again under the thresholds of the step (`interp::reposition`). With the default thresholds it is the site the interpretation stored; the votes of the
//! MSn tree are not in the file, so a row voted by the tree keeps its site whatever the thresholds.

use mzlab_core::elements::parse_formula;
use mzlab_core::error::{Error, Result};
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::json::{Out, Value};
use mzlab_core::step::DataType;
use mzlab_feat::chem::posizioni::Pol;
use mzlab_feat::interp::reposition::reposition;
use mzlab_feat::interp::sito::SiteParams;
use mzlab_feat::store::{read_table_from, Slice};

use crate::flow::{read_one, write_made, FileRunner};
use crate::flow_link::parent_of;

fn param(job: &Job, name: &str) -> Option<f64> {
    job.resolved
        .params
        .iter()
        .find(|(k, _)| k == name)
        .and_then(|(_, v)| v.f64())
}

/// The thresholds of the step over `SiteParams::default()`.
pub fn site_params(job: &Job) -> SiteParams {
    let d = SiteParams::default();
    SiteParams {
        match_ppm: param(job, "match_ppm").unwrap_or(d.match_ppm),
        min_rel: param(job, "min_rel").unwrap_or(d.min_rel),
        min_informative: param(job, "min_informative").map_or(d.min_informative, |v| v as usize),
        margin: param(job, "margin").unwrap_or(d.margin),
        ll_support: param(job, "ll_support").unwrap_or(d.ll_support),
        ll_against: param(job, "ll_against").unwrap_or(d.ll_against),
    }
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

/// `sites/1` text of the `.mzfeat` `tp` (with the block `interp`) for the parent file `parent`.
pub fn position_text(job: &Job, tp: &[u8], parent: &[u8]) -> Result<String> {
    let p = parent_of(parent)?;
    if p.smiles.is_empty() {
        return Err(Error::new("err.flow.position.smiles"));
    }
    let neutral = parse_formula(&p.formula)?;
    let pol = polarity_of(tp)?;
    let rows = reposition(
        &mut Slice(tp),
        tp.len() as u64,
        (&p.smiles, &neutral),
        pol,
        &site_params(job),
    )?;
    let mut o = Out::default();
    o.raw("{\"schema\":\"sites/1\",\"parent\":")
        .str(&p.smiles)
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

/// `tp.position`: `sites/1` written next to the other outputs of the run.
pub(crate) fn position_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let (tp, parent) = (read_one(r, job, "tp")?, read_one(r, job, "parent")?);
    let text = position_text(job, &tp.1, &parent.1)?;
    write_made(r, job, text, DataType::Transform)
}
