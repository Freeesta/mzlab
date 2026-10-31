//! The native executor of `spectra.insilico@1`: the file wrapper of the text in `mzlab-feat` (`flow_insilico`, the one implementation the browser runs too). The `suspects` file
//! is a CSV list, a `screen/1` text or the catalogue of a candidate package (`insilico pack`), whose pieces are read from the folder of the catalogue where the mass window
//! of a query overlaps them. The `candidates` file is JSON, like `spectra.match`; its hits are `source: insilico` hypotheses.

use crate::flow::{one_input, read_one, write_made, FileRunner};
use crate::flow_match::Named;
use mzlab_core::error::Result;
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::step::DataType;
use std::path::Path;

/// Runs the step of `job` on the query spectra and the suspect list and returns the text of the `candidates` file: queries in file order, each query's candidates
/// best score first, at most `top` of them. `dir` is the folder of the pieces when `suspects` is a package catalogue.
pub fn insilico_text(
    job: &Job,
    spectra: Named,
    suspects: &[u8],
    dir: Option<&Path>,
) -> Result<String> {
    let mut piece = |file: &str| -> Result<Vec<u8>> {
        #[cfg(not(target_family = "wasm"))]
        if let Some(d) = dir {
            return crate::insilico_pack::read_piece(d, suspects, file);
        }
        let _ = dir;
        Err(mzlab_core::error::Error::new("err.flow.insilico.piece").with("file", file))
    };
    mzlab_feat::flow_insilico::insilico_text(&job.resolved.params, spectra, suspects, &mut piece)
}

pub(crate) fn insilico_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let (spectra, suspects) = (read_one(r, job, "spectra")?, read_one(r, job, "suspects")?);
    let list = one_input(r, job, "suspects")?;
    let text = insilico_text(job, (&spectra.0, &spectra.1), &suspects.1, list.parent())?;
    write_made(r, job, text, DataType::Candidates)
}

#[cfg(test)]
#[path = "flow_insilico_tests.rs"]
mod tests;
