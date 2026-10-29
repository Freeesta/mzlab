//! The native executor of `tp.network@1`: the network of the products (`rete/1`) of a `.mzfeat` with the blocks `tp/1` and `interp/1`. The rules are the
//! transformation table of mzFinder; the engine is `mzlab_feat::rete::network_text`, the one the `.wasm` `table` runs (`rete_build`).

use crate::flow::{read_one, write_made, FileRunner};
use mzlab_core::error::Result;
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::step::DataType;
use mzlab_feat::rete::{network_text, ReteParams};
use mzlab_feat::store::Slice;

const RULES: &[u8] = include_bytes!("../../../TP_Mine/data/regole_tp_2.json");

fn param(job: &Job, name: &str) -> Option<f64> {
    let found = job.resolved.params.iter().find(|(k, _)| k == name);
    found.and_then(|(_, v)| v.f64())
}

/// `rete/1` text from the `tp` and `table` files and the optional `campioni/2` of the series.
pub fn network_json(job: &Job, tp: &[u8], table: &[u8], samples: Option<&[u8]>) -> Result<String> {
    let d = ReteParams::default();
    let params = ReteParams {
        ppm: param(job, "ppm").unwrap_or(d.ppm),
        min_fragments: param(job, "min_fragments").map_or(d.min_fragments, |v| v as usize),
    };
    let campioni = samples.map(String::from_utf8_lossy).unwrap_or_default();
    network_text(
        (&mut Slice(table), table.len() as u64),
        (&mut Slice(tp), tp.len() as u64),
        &campioni,
        RULES,
        &params,
    )
}

pub(crate) fn network_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let (tp, table) = (read_one(r, job, "tp")?, read_one(r, job, "table")?);
    let has_samples = job.inputs.iter().any(|(p, _)| p == "samples");
    let samples = if has_samples {
        Some(read_one(r, job, "samples")?)
    } else {
        None
    };
    let text = network_json(
        job,
        &tp.1,
        &table.1,
        samples.as_ref().map(|s| s.1.as_slice()),
    )?;
    write_made(r, job, text, DataType::Transform)
}
