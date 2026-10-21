//! The native executor of `structure.render@1`: the `drawing` of `interp/1` for every row of a `sites/1` file (`tp.position`) over the SMILES of the parent,
//! the one form that tells the truth about the position (`interp::redraw`, the function behind the drawing stored in the block `interp`).

use mzlab_core::error::{Error, Result};
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::json::{parse, Out};
use mzlab_core::step::DataType;
use mzlab_feat::interp::redraw::drawing_of_site;

use crate::flow::{read_one, write_made, FileRunner};
use crate::flow_link::parent_of;

/// `drawings/1` text for the `sites/1` file and the parent file. The SMILES of the parent file wins over the one `sites/1` carries.
pub fn render_text(sites: &[u8], parent: &[u8]) -> Result<String> {
    let doc = parse(sites)?;
    if doc.get("schema").and_then(|v| v.str()) != Some("sites/1") {
        return Err(Error::new("err.flow.render.sites"));
    }
    let given = parent_of(parent)?.smiles;
    let carried = doc.get("parent").and_then(|v| v.str()).unwrap_or("");
    let smiles = if given.is_empty() { carried } else { &given };
    if smiles.is_empty() {
        return Err(Error::new("err.flow.position.smiles"));
    }
    let mut o = Out::default();
    o.raw("{\"schema\":\"drawings/1\",\"parent\":")
        .str(smiles)
        .raw(",\"rows\":[");
    for (i, r) in doc
        .get("rows")
        .map(|a| a.arr())
        .unwrap_or_default()
        .iter()
        .enumerate()
    {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"id\":")
            .int(r.get("id").and_then(|v| v.f64()).unwrap_or(0.0) as i64)
            .raw(",\"drawing\":")
            .raw(&drawing_of_site(smiles, r.get("site")))
            .raw("}");
    }
    o.raw("]}");
    Ok(o.0)
}

/// `structure.render`: `drawings/1` written next to the other outputs of the run.
pub(crate) fn render_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let (sites, parent) = (read_one(r, job, "sites")?, read_one(r, job, "parent")?);
    write_made(
        r,
        job,
        render_text(&sites.1, &parent.1)?,
        DataType::Structure,
    )
}
