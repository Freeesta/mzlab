//! The route `feat/import` of `rpc`: the readers of the tables of other programs (mzMine, FBMN/GNPS2, MS-DIAL) of `mzlab_feat::import`, the same code as the `.wasm` «import».
//! `{op: "feat/import", kind: "build", files: [{name, sha256, text}], pol, max_features}` answers the table as JSON; `kind: "check_structure"` many `{formula, precursor, adduct, pol, tol}` at once (the page asks for all the annotations of a table in one request and reads the answers from a cache).
//! An error of a reader comes back as its own key («columns», «empty», «noTable») with the text in `detail`.

use mzlab_core::json::Out;
use mzlab_core::{Error, Result};
use mzlab_feat::import::{self, ImportError, InFile};
use serde_json::Value;

use crate::engine::State;
use crate::rpc::{text, Answer};

fn reader(e: ImportError) -> Error {
    Error::new(e.key).with("detail", e.msg)
}

impl State {
    pub(crate) fn op_import(&self, p: &Value) -> Result<Answer> {
        let s = |v: &Value, k: &str| v[k].as_str().unwrap_or("").to_string();
        match p["kind"].as_str().unwrap_or("") {
            "build" => {
                let files: Vec<InFile> = p["files"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .map(|f| InFile {
                                name: s(f, "name"),
                                text: s(f, "text"),
                                sha256: s(f, "sha256"),
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let pol = p["pol"].as_i64().unwrap_or(0) as i32;
                let max = p["max_features"].as_u64().unwrap_or(0) as usize;
                let t = import::build(&files, pol, max).map_err(reader)?;
                text(import::table_json(&t))
            }
            "check_structure" => {
                let mut o = Out::default();
                o.raw("[");
                for (i, c) in p["calls"].as_array().into_iter().flatten().enumerate() {
                    if i > 0 {
                        o.raw(",");
                    }
                    let r = import::check_structure(
                        &s(c, "formula"),
                        c["precursor"].as_f64(),
                        &s(c, "adduct"),
                        c["pol"].as_i64().unwrap_or(0) as i32,
                        c["tol"].as_f64().unwrap_or(20.0),
                    );
                    o.raw(&r.json());
                }
                o.raw("]");
                text(o.0)
            }
            _ => Err(Error::new("rpc.unknown")),
        }
    }
}
