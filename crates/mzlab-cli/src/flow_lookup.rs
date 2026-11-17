//! The native executor of `spectra.lookup@1`: the entries of a spectral library the user declares that carry a given name, formula, InChIKey, accession
//! or precursor m/z, written as an MGF that `spectra.match` or `spectra.chimeric` read as queries. Nothing is built in and nothing is looked up online.

use mzlab_core::error::Result;
use mzlab_core::flow::Job;
use mzlab_core::step::{DataType, Param, Port, RunsIn, Step};

use crate::flow::{io, read_one, Exec, FileRunner};

/// The executor of the step, read by the runner after its own list.
pub(crate) const EXECUTORS: &[(&str, Exec)] =
    &[("spectra.lookup", Exec::File("spectra", lookup_job))];

const BY: &[&str] = &["name", "formula", "inchikey", "accession", "precursor_mz"];

/// `spectra.lookup@1`: the library entries that match a name, formula, InChIKey, accession or precursor m/z.
pub fn lookup_step() -> Step {
    Step {
        ins: vec![Port::new(
            "library",
            DataType::Library,
            "Spectral library the user declares",
        )],
        outs: vec![Port::new("spectra", DataType::Spectra, "The library entries found, as MGF")],
        params: vec![
            Param::choice("by", "Look up by", BY)
                .default_str("name")
                .doc("The field of the library entries that value is compared with; precursor_mz takes a number and the window ppm"),
            Param::text("value", "Value")
                .doc("The text to find (for precursor_mz, the m/z); required"),
            Param::boolean("exact", "Whole name")
                .default_bool(false)
                .doc("By name: the name must be the whole value, not only contain it (any case); the other fields are always compared whole"),
            Param::number("ppm", "Precursor window")
                .min(0.0)
                .default_num(10.0)
                .unit("ppm")
                .doc("By precursor_mz: the entries whose precursor is this close to the value"),
            Param::integer("top_n", "Entries kept")
                .min(1.0)
                .default_num(50.0)
                .doc("The first entries in the order of the library; the rest are not written"),
        ],
        runs_in: RunsIn {
            wasm: Some("lib"),
            native: true,
            ..RunsIn::default()
        },
        errors: vec!["err.flow.lookup.value", "err.flow.match.library"],
        ..Step::core(
            "spectra.lookup",
            "Find spectra in a library",
            "Writes the entries of the library you declare that have a given name, formula, InChIKey, accession or precursor m/z, with their peaks untouched, so that the next steps can use them as queries. The library is always the one you supply: nothing is built in and nothing is searched online.",
        )
    }
}

fn lookup_job(r: &FileRunner, job: &Job) -> Result<mzlab_core::flow::Artifact> {
    let library = read_one(r, job, "library")?;
    let out = mzlab_lib::flowmatch::lookup_text(&job.resolved.params, (&library.0, &library.1))?;
    let path = r.work.join(format!("{}.mgf", job.instance.id));
    std::fs::write(&path, out).map_err(io)?;
    r.keep(path, DataType::Spectra)
}
