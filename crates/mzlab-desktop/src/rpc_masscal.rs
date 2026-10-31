//! The op `masscal` of `rpc`: the mass error of the background ions in one file (`api/masscal`, the TIC menu «Check calibration»). The twin of `rust-masscal.js`: the same
//! `mzlab_feat::massacc::check` over the `.mzidx` of the file and `contaminanti.json`, so the report is the one the browser gives byte for byte.

use mzlab_core::idxread::Index;
use mzlab_core::json::parse;
use mzlab_core::Result;
use mzlab_feat::massacc::{self, Params};
use serde_json::Value;

use crate::engine::State;
use crate::rpc::{map_file, text, Answer, MapAt};

/// The lab list of background ions (one file for the page and the program).
const CONTAMINANTS: &[u8] = include_bytes!("../../../mzlab/web/contaminanti.json");

impl State {
    /// `masscal {name, params}`: `massacc/1` of the file `name`; `params` is the JSON object of the step parameters.
    pub(crate) fn op_masscal(&self, p: &Value) -> Result<Answer> {
        let name = p["name"].as_str().unwrap_or("");
        let params = Params::from_value(&parse(p["params"].as_str().unwrap_or("{}").as_bytes())?)?;
        let (map, size) = map_file(&self.indexer()?.path(name)?)?;
        let mut open = |_: usize| Index::open(MapAt(map.0.clone()), size);
        let out = massacc::check::<MapAt, Vec<u8>>(
            &mut open,
            &[(name.to_string(), String::new())],
            None,
            CONTAMINANTS,
            None,
            None,
            &params,
        )?;
        text(out.json)
    }
}
