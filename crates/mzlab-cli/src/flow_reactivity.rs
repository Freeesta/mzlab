//! The native executor of `structure.reactivity@1`: `reatt/1` of the parent (`mzlab_chem::reatt::reactivity_json`, the engine of the `.wasm` `chem`),
//! with the solvent, the conformers, the form of the parent and the seed of the step as its options.

use mzlab_chem::reatt::reactivity_json;
use mzlab_core::error::{Error, Result};
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::json::Out;
use mzlab_core::step::DataType;

use crate::flow::{read_one, write_made, FileRunner};
use crate::flow_link::parent_of;

/// The options the engine reads (`Opts::parse`), from the parameters the step resolved.
fn opts_json(job: &Job) -> String {
    let given = |k: &str| job.resolved.params.iter().find(|(n, _)| n == k);
    let mut o = Out::default();
    o.raw("{");
    let mut first = true;
    for k in ["solvent", "protomer"] {
        if let Some(s) = given(k).and_then(|(_, v)| v.str()) {
            o.raw(if first { "" } else { "," }).str(k).raw(":").str(s);
            first = false;
        }
    }
    for k in ["conformers", "seed"] {
        if let Some(n) = given(k).and_then(|(_, v)| v.f64()) {
            o.raw(if first { "" } else { "," })
                .str(k)
                .raw(":")
                .int(n as i64);
            first = false;
        }
    }
    o.raw("}");
    o.0
}

/// `reatt/1` text of the parent file under the options of the step.
pub fn reactivity_text(job: &Job, parent: &[u8]) -> Result<String> {
    let smiles = parent_of(parent)?.smiles;
    if smiles.is_empty() {
        return Err(Error::new("err.flow.position.smiles"));
    }
    reactivity_json(&smiles, &opts_json(job))
}

/// `structure.reactivity`: `reatt/1` written next to the other outputs of the run.
pub(crate) fn reactivity_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let parent = read_one(r, job, "parent")?;
    write_made(r, job, reactivity_text(job, &parent.1)?, DataType::Annot)
}
