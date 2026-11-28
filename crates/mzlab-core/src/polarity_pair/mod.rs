//! `pair` (`pair/1`): the features of the positive and the negative method of one sample set paired by their neutral mass and RT, to see which species
//! are seen in both polarities (an acid shows as [M-H]- and as [M+H]+ at the same RT). Each ion gives a neutral mass under every adduct of its side
//! (a list the user edits: names of the adduct table or `LABEL:SHIFT`); two ions are a pair within `mass_tol` and `rt_tol`. Nothing is merged and
//! every pair, the rule behind it and the rows without a partner are in the answer.

mod pairs;
mod params;
mod report;
mod table;

use crate::error::{Error, Result};
use crate::json::{parse, Value};

pub use params::{Params, NEG_DEFAULT, POS_DEFAULT};

pub const SCHEMA: &str = "pair/1";

/// `pair` over the request `{"pos": {"name", "text"}, "neg": {"name", "text"}, "params": {...}}`: the `pair/1` document (the pairs as CSV in `csv`).
pub fn route(input: &str) -> Result<String> {
    let req = parse(input.as_bytes()).map_err(|_| Error::new("err.pair.request"))?;
    let side = |k: &str| -> Result<(String, String)> {
        let t = req.get(k).ok_or_else(|| Error::new("err.pair.request"))?;
        let text = |f: &str| t.get(f).and_then(Value::str).unwrap_or("").to_string();
        Ok((text("name"), text("text")))
    };
    let (pos, neg) = (side("pos")?, side("neg")?);
    run(&pos, &neg, &Params::from_json(req.get("params"))?)
}

/// The same over two tables in memory, `(name, text)` each (the CSV of `mzlab-cli features`).
pub fn run(pos: &(String, String), neg: &(String, String), p: &Params) -> Result<String> {
    let name = |n: &str, d: &str| {
        if n.is_empty() {
            d.to_string()
        } else {
            n.to_string()
        }
    };
    let a = table::read(&name(&pos.0, "pos"), &pos.1)?;
    let b = table::read(&name(&neg.0, "neg"), &neg.1)?;
    if a.is_empty() && b.is_empty() {
        return Err(Error::new("err.pair.empty"));
    }
    let m = pairs::run(&a, &b, p);
    Ok(report::document(&a, &b, &m, p))
}

#[cfg(test)]
mod tests;
