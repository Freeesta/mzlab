//! The native executor of `structure.render@1`: the `drawing` of `interp/1` for every row of a `sites/1` file (`tp.position`) over the SMILES of the parent,
//! the one form that tells the truth about the position (`interp::redraw::render_text`, the same text as the `.wasm` `interp` in the browser).

use mzlab_core::error::Result;
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::step::DataType;
use mzlab_feat::interp::redraw::render_text;

use crate::flow::{read_one, write_made, FileRunner};

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
