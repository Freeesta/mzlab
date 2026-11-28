//! `features.pair`: the `.mzfeat` tables of the positive and the negative method written as the CSV of `mzlab-cli features` (`union::series_csv`) and
//! paired by `mzlab_core::polarity_pair` (neutral mass under each adduct of the side, RT), with the `pair/1` document. The CLI executor, `table pair` and the
//! `flowtab` `.wasm` all call `pair_text`, so the bytes are the same.

use crate::union::series_csv;
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, Value};
use mzlab_core::polarity_pair::{run, Params};

/// The pairs (CSV) and the `pair/1` document.
#[derive(Debug, Clone)]
pub struct Made {
    pub csv: String,
    pub doc: String,
}

/// `pos` and `neg`: `(name, .mzfeat bytes)` of each side; `params` the canonical parameters of the step.
pub fn pair_text(
    params: &[(String, Value)],
    pos: (&str, &[u8]),
    neg: (&str, &[u8]),
) -> Result<Made> {
    let p = Params::from_json(Some(&Value::Obj(params.to_vec())))?;
    let side = |(n, b): (&str, &[u8])| Ok((n.to_string(), series_csv(b)?));
    let doc = run(&side(pos)?, &side(neg)?, &p)?;
    let csv = parse(doc.as_bytes())
        .ok()
        .and_then(|v| v.get("csv").and_then(Value::str).map(String::from))
        .ok_or_else(|| Error::new("err.pair.request"))?;
    Ok(Made { csv, doc })
}
