//! `qc.mass_accuracy` in the browser: `mzlab_feat::massacc::check`, the function `flow run` calls, over the `.mzidx` of the files as `read(k, offset, length)`. One call gives the
//! report (`massacc/1`), the warning keys and, with a model and a table, the table with its m/z corrected.

use js_sys::Function;
use mzlab_core::error::Error;
use mzlab_core::idxread::Index;
use mzlab_core::json::{parse, Value};
use mzlab_feat::massacc::{self, Params};
use wasm_bindgen::prelude::*;

use crate::{js_err, table_reader, JsReader};

fn non_empty(b: &[u8]) -> Option<&[u8]> {
    (!b.is_empty()).then_some(b)
}

/// What a run made.
#[wasm_bindgen]
pub struct MassAcc {
    json: String,
    table: Option<Vec<u8>>,
    warnings: Vec<String>,
}

#[wasm_bindgen]
impl MassAcc {
    /// `massacc/1`.
    pub fn json(&self) -> String {
        self.json.clone()
    }

    /// The corrected table; empty when the run wrote none.
    pub fn table(&self) -> Vec<u8> {
        self.table.clone().unwrap_or_default()
    }

    /// Whether the run wrote a table.
    pub fn has_table(&self) -> bool {
        self.table.is_some()
    }

    /// The warning keys of the run as a JSON array.
    pub fn warnings(&self) -> String {
        let mut o = mzlab_core::json::Out::default();
        o.raw("[");
        for (i, w) in self.warnings.iter().enumerate() {
            o.raw(if i > 0 { "," } else { "" }).str(w);
        }
        o.raw("]");
        o.0
    }
}

/// The check of the files whose `.mzidx` `index_read(k, offset, length)` serves. `names_json`: `[[name, sha256], ...]` in the order of the indexes; `samples` and `csv` are empty when
/// the port is not connected (`table_read(0, offset, length)` serves the `.mzfeat`, `table_size` 0 for none); `contaminants` is `contaminanti.json`; `params_json` is the object of the resolved parameters.
#[wasm_bindgen]
pub fn mass_accuracy(
    index_read: Function,
    index_sizes: Vec<f64>,
    names_json: &str,
    samples: &[u8],
    contaminants: &[u8],
    csv: &[u8],
    table_read: Function,
    table_size: f64,
    params_json: &str,
) -> Result<MassAcc, JsValue> {
    let params =
        Params::from_value(&parse(params_json.as_bytes()).map_err(js_err)?).map_err(js_err)?;
    let text = |v: &Value, i: usize| v.at(i).and_then(Value::str).unwrap_or("").to_string();
    let list = parse(names_json.as_bytes()).map_err(js_err)?;
    let names: Vec<(String, String)> = list
        .arr()
        .iter()
        .map(|n| (text(n, 0), text(n, 1)))
        .collect();
    let mut open = |k: usize| {
        let size = *index_sizes
            .get(k)
            .ok_or_else(|| Error::new("err.flow.port.missing").with("port", "raw"))?;
        Index::open(
            JsReader {
                read: index_read.clone(),
                k: k as u32,
            },
            size as u64,
        )
    };
    let mut reader = table_reader(table_read);
    let table_in = (table_size > 0.0).then_some((&mut reader, table_size as u64));
    let out = massacc::check(
        &mut open,
        &names,
        non_empty(samples),
        contaminants,
        non_empty(csv),
        table_in,
        &params,
    )
    .map_err(js_err)?;
    Ok(MassAcc {
        json: out.json,
        table: out.table,
        warnings: out.warnings,
    })
}
