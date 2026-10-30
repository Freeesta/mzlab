//! The query of `table.match`: the MGF of the rows of the aligned table that have an MS2 spectrum (`core::ms2_file`, so the page export and the plan pick the same scan),
//! with the id of the row as the accession the matcher names its query by. One function for `flow run` (`mzlab-cli`) and for the browser (`table_match_rows` of the bridge).

use crate::{core, pol};
use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::{Out, Value};

/// The options of `ms2_file` the parameters of the step spell: `{"ms2_spectrum":..,"ms2_background":..}`.
pub fn spec_text(params: &[(String, Value)]) -> String {
    let get = |name: &str| params.iter().find(|(k, _)| k == name).map(|(_, v)| v);
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

/// Upper bound of the files whose indexes are read for the polarity.
const MAX_FILES: usize = 4096;

/// The MGF of the rows with a spectrum. `samples` is the samples file as it is (either schema), `params` the resolved parameters of the step, `open(k)` the index of file `k`.
pub fn rows_mgf<R: ReadAt, I: ReadAt>(
    r: &mut R,
    size: u64,
    samples: &[u8],
    params: &[(String, Value)],
    table_sha256: &str,
    mut open: impl FnMut(usize) -> Result<Index<I>>,
) -> Result<String> {
    let samples = pol::samples_v1(samples)?;
    // the recipe, else the sign of the files (`pol::of_table`, the rule of the exports): a negative series is searched as negative
    let files: Vec<_> = (0..MAX_FILES).map_while(|k| open(k).ok()).collect();
    let polarity = pol::of_table(r, size, &files)?;
    drop(files);
    let mgf = core::ms2_file(
        r,
        size,
        &samples,
        &spec_text(params),
        table_sha256,
        polarity,
        "mgf",
        open,
    )?;
    Ok(mgf.replace("\nFEATURE_ID=", "\nACCESSION="))
}
