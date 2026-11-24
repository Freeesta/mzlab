//! `quant.targets` in the browser (`.wasm` `quanttargets`): `mzlab_feat::quant_targets::run` over the `.mzidx` files of OPFS, the function
//! `flow run` and `mzlab-cli quant targets` call (one pass per file, in file order), so the table and the log are the same bytes.
//! A module of its own so that no other module grows: it is loaded only when the step runs (`flow-web-quanttargets.js`).

use js_sys::Function;
use mzlab_core::error::Error;
use mzlab_core::json::{parse, Out};
use mzlab_feat::flow_ms2::json_params;
use wasm_bindgen::prelude::*;

/// What a `quant.targets` run made: the long table (CSV), the `quant-targets-log/1` log and the warning keys.
#[wasm_bindgen]
pub struct QuantTargets {
    csv: String,
    log: String,
    warnings: Vec<String>,
}

#[wasm_bindgen]
impl QuantTargets {
    pub fn csv(&self) -> String {
        self.csv.clone()
    }

    pub fn log(&self) -> String {
        self.log.clone()
    }

    /// The warning keys as a JSON array of strings.
    pub fn warnings(&self) -> String {
        let mut o = Out::default();
        o.raw("[");
        for (i, w) in self.warnings.iter().enumerate() {
            o.raw(if i > 0 { "," } else { "" }).str(w);
        }
        o.raw("]");
        o.0
    }
}

/// `quant.targets` over the indexes `index_read(k, offset, length)` serves (`index_sizes[k]` bytes each; `names_json`: the file names in the same
/// order, as a JSON array), the targets table `targets` (CSV or JSON bytes) and `params_json` (the step's parameters merged with its defaults).
/// Errors are JSON `{"error_key","params"}` texts.
#[wasm_bindgen]
pub fn flow_quant_targets(
    index_read: Function,
    index_sizes: Vec<f64>,
    names_json: &str,
    targets: &[u8],
    params_json: &str,
) -> Result<QuantTargets, JsValue> {
    let params = json_params(params_json).map_err(crate::js_err)?;
    let list = parse(names_json.as_bytes()).map_err(crate::js_err)?;
    let names: Vec<String> = list
        .arr()
        .iter()
        .map(|n| n.str().unwrap_or("").to_string())
        .collect();
    let mut open = |k: usize| {
        let size = *index_sizes
            .get(k)
            .ok_or_else(|| Error::new("err.flow.port.missing").with("port", "raw"))?;
        crate::open_one(&index_read, k, size)
    };
    let out = mzlab_feat::quant_targets::run(&mut open, &names, targets, &params)
        .map_err(crate::js_err)?;
    Ok(QuantTargets {
        csv: out.csv,
        log: out.log,
        warnings: out.warnings,
    })
}
