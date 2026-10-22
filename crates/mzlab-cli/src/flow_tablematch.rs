//! The native executor of `table.match@1`: the MS2 spectrum of every row of the aligned table (`ms2_file`, the MGF of the export, so the page and the plan
//! pick the same scan) scored against a library with the engine of `spectra.match`. The query of a hit is the id of the row in the table.

use crate::flow::{io, read_one, Exec, FileRunner};
use crate::flow_export::polarity;
use crate::flow_match::match_text;
use mzlab_core::budget::sha256_hex;
use mzlab_core::error::{Error, Result};
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::json::Out;
use mzlab_core::step::DataType;
use mzlab_feat::store::Slice;
use mzlab_feat_table_wasm::core::ms2_file;

/// The executor of the step, read by the runner after its own list.
pub(crate) const EXECUTORS: &[(&str, Exec)] =
    &[("table.match", Exec::File("hits", table_match_job))];

/// The options of `ms2_file` the parameters of the step spell: `{"ms2_spectrum":..,"ms2_background":..}`.
fn spec_text(job: &Job) -> String {
    let get = |name: &str| {
        job.resolved
            .params
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v)
    };
    let mut o = Out::default();
    o.raw("{\"ms2_spectrum\":")
        .str(get("ms2_spectrum").and_then(|v| v.str()).unwrap_or("best"));
    o.raw(",\"ms2_background\":").raw(
        if get("ms2_background").and_then(|v| v.bool()) == Some(true) {
            "true"
        } else {
            "false"
        },
    );
    o.raw("}");
    o.0
}

/// The MGF of the rows with a spectrum, with the id of the row as the accession the matcher names its query by.
fn rows_mgf(r: &FileRunner, job: &Job) -> Result<String> {
    let (table, samples) = (read_one(r, job, "table")?, read_one(r, job, "samples")?);
    if table.1.first() == Some(&b'{') {
        return Err(Error::new("err.flow.export.view"));
    }
    let paths = crate::flow::all_inputs(r, job, "index");
    let open = |k: usize| match paths.get(k) {
        Some(p) => crate::flow_base::open(p),
        None => Err(Error::new("err.flow.tablematch.files")),
    };
    let mgf = ms2_file(
        &mut Slice(&table.1),
        table.1.len() as u64,
        &crate::flow_table::samples_text(&samples.1)?,
        &spec_text(job),
        &sha256_hex(&table.1),
        polarity(&table.1)?,
        "mgf",
        open,
    )?;
    Ok(mgf.replace("\nFEATURE_ID=", "\nACCESSION="))
}

fn table_match_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let library = read_one(r, job, "library")?;
    let mgf = rows_mgf(r, job)?;
    let text = match_text(job, ("rows.mgf", mgf.as_bytes()), (&library.0, &library.1))?;
    let out = r.work.join(format!("{}.json", job.instance.id));
    std::fs::write(&out, text).map_err(io)?;
    r.keep(out, DataType::Candidates)
}
