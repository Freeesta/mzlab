//! `union` (`union/1`): the rows of the feature tables of several series (conditions, replicates, polarities) joined into ONE target list for
//! `quant.targets`, so a whole campaign is measured once, every cell integrated or explicitly absent. Rows of different series are one target when
//! their m/z agree within `da` and their RT, after an optional per-series RT map (`rt_warp`), within `rt_tol`; rows of one series are never merged
//! (two rows that stay inside one window are reported as a conflict and both are kept). Every merge, drop and automatic choice is in the answer.

mod merge;
mod params;
mod read;
mod report;
mod warp;

use crate::error::{Error, Result};
use crate::json::{parse, Value};

pub use params::Params;

pub const SCHEMA: &str = "union/1";

/// `union` over the request `{"tables": [{"name", "text"}], "params": {...}}`: the `union/1` document (the target list as CSV in `csv`).
pub fn route(input: &str) -> Result<String> {
    let req = parse(input.as_bytes()).map_err(|_| Error::new("err.union.request"))?;
    let tables = req
        .get("tables")
        .map(Value::arr)
        .filter(|t| !t.is_empty())
        .ok_or_else(|| Error::new("err.union.empty"))?;
    let params = Params::from_json(req.get("params"))?;
    let mut series = Vec::new();
    for (k, t) in tables.iter().enumerate() {
        let name = t.get("name").and_then(Value::str).unwrap_or("");
        let text = t.get("text").and_then(Value::str).unwrap_or("");
        let name = if name.is_empty() {
            format!("series{}", k + 1)
        } else {
            name.to_string()
        };
        series.push(read::read(&name, text, &params)?);
    }
    if series.iter().all(|s| s.rows.is_empty()) {
        return Err(Error::new("err.union.empty"));
    }
    let merged = merge::merge(&series, &params);
    Ok(report::document(&series, &merged, &params))
}

#[cfg(test)]
mod tests;
