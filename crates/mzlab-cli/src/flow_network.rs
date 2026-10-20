//! The native executor of `tp.network@1`: the network of the products (`rete/1`) of a `.mzfeat` with the blocks `tp/1` and `interp/1`. The rules are the
//! transformation table of mzFinder; the engine is `mzlab_feat::rete::network_text`, the one the `.wasm` `interp` runs.

use mzlab_core::error::Result;
use mzlab_core::flow::Job;
use mzlab_feat::rete::{network_text, ReteParams};
use mzlab_feat::store::Slice;

const RULES: &[u8] = include_bytes!("../../../TP_Mine/data/regole_tp.json");

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
