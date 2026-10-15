//! The route `feat/attributio/annotate` of `rpc`: the table of levels of a campaign folder of `mzlab_feat::attributio::table`, the same code as the `.wasm` «attributio».
//! `{op: "feat/attributio/annotate", root, kind: "json" | "csv" | "xlsx", files: [{path, text}]}`: the page sends the text files of the folder (the engine reads only
//! the ones `campaign.toml` names); the answer is `{summary, text}` with the summary of the run and the table in the asked rendering.

use mzlab_core::json::Out;
use mzlab_core::{Error, Result};
use mzlab_feat::attributio::io::{Io, MemFiles};
use mzlab_feat::attributio::table;
use serde_json::Value;

use crate::engine::State;
use crate::rpc::{text, Answer};

impl State {
    pub(crate) fn feat_attributio(&self, p: &Value) -> Result<Answer> {
        let mut files = MemFiles::default();
        for f in p["files"].as_array().into_iter().flatten() {
            files.add(
                f["path"].as_str().unwrap_or(""),
                f["text"].as_str().unwrap_or("").as_bytes(),
            );
        }
        let read = |q: &str| files.read(q);
        let list = |d: &str| files.list(d);
        let io = Io::new(&read, &list);
        let run = table::run(&io, p["root"].as_str().unwrap_or(""))
            .map_err(|e| Error::new("error.attributio").with("detail", e.to_string()))?;
        let mut body = String::new();
        let mut emit = |s: &str| body.push_str(s);
        match p["kind"].as_str().unwrap_or("json") {
            "csv" => run.csv_to(&mut emit),
            "xlsx" => run.xlsx_rows_to(&mut emit),
            _ => run.json_to(&mut emit),
        }
        let mut o = Out::default();
        o.raw("{\"summary\":").raw(&run.summary()).raw(",\"text\":").str(&body).raw("}");
        text(o.0)
    }
}
