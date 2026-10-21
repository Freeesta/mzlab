//! `import.raw`: the native executor that recognises each vendor dataset with the reader of the registry and checks it against the limits of the
//! desktop; it changes nothing, so the artifact it hands on is the input itself. Called by `flow.rs`.

use mzlab_core::error::Result;
use mzlab_core::flow::Job;
use mzlab_core::lettori::Limiti;
use mzlab_core::step::DataType;

use crate::flow::{FileRunner, Made};

pub(crate) fn import_job(r: &FileRunner, job: &Job) -> Result<Made> {
    let input = &job.inputs[0].1[0];
    let fs = crate::flow_set::file_set(&r.path(input))?;
    let reader = mzlab_lettori::apri(fs, &Limiti::desktop())?;
    let ids: Vec<u32> = reader.campioni().iter().map(|s| s.id).collect();
    crate::flow_set::sample_index(input.label.as_deref().unwrap_or(""), &ids)?;
    let mut artifact = input.clone();
    artifact.ty = DataType::Raw;
    Ok(Made::new("raw", artifact))
}
