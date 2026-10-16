//! The route `feat/attributio/annotate` of `rpc`: the table of levels of a campaign folder of `mzlab_feat::attributio::table`, the same code as the `.wasm` «attributio».
//! `{op: "feat/attributio/annotate", root, kind: "json" | "csv" | "xlsx", files: [{path, text}]}`: the page sends the text files of the folder (the engine reads only
//! the ones `campaign.toml` names); the answer is `{summary, text}` with the summary of the run and the table in the asked rendering.
//! The route `feat/attributio/export` gives the exports (`schymanski`, `publication`, `quantify`, `gradient`, `evidence`) as the JSON of their files.
//! The route `feat/attributio/review` is the Banco 2b: `{root, files, kind: "review", pol, id}` answers the JSON of the review of one feature; `{root, files, kind: "decision",
//! decision, folder?}` answers `{path, added, replaced, unchanged, text, written}`, and writes the decisions file straight into `folder` (the campaign folder on disk) when it is given.

use mzlab_core::json::Out;
use mzlab_core::{Error, Result};
use mzlab_feat::attributio::export::run::{run, Options};
use mzlab_feat::attributio::io::{Io, MemFiles};
use mzlab_feat::attributio::review::banco::review_json;
use mzlab_feat::attributio::review::evidence::evidence_json;
use mzlab_feat::attributio::review::save::decision_text;
use mzlab_feat::attributio::table;
use serde_json::Value;

use crate::engine::State;
use crate::rpc::{text, Answer};

impl State {
    /// The Banco 2b: the review of one feature, or the saving of a decision.
    pub(crate) fn feat_attributio_review(&self, p: &Value) -> Result<Answer> {
        let files = mem_files(p);
        let read = |q: &str| files.read(q);
        let list = |d: &str| files.list(d);
        let io = Io::new(&read, &list);
        let root = p["root"].as_str().unwrap_or("");
        let fail = |e: mzlab_feat::attributio::error::CampaignError| {
            Error::new("error.attributio").with("detail", e.to_string())
        };
        if p["kind"].as_str() == Some("decision") {
            let d = p["decision"].as_str().unwrap_or("");
            let s = decision_text(&io, root, d).map_err(fail)?;
            let written = write_in(p["folder"].as_str().unwrap_or(""), &s.path, &s.text)?;
            let mut o = Out::default();
            o.raw("{\"path\":").str(&s.path);
            o.raw(",\"added\":").int(s.added as i64);
            o.raw(",\"replaced\":").int(s.replaced as i64);
            o.raw(",\"unchanged\":").int(s.unchanged as i64);
            o.raw(",\"written\":")
                .raw(if written { "true" } else { "false" });
            o.raw(",\"text\":").str(&s.text).raw("}");
            return text(o.0);
        }
        let pol = p["pol"].as_str().unwrap_or("POS");
        text(review_json(&io, root, pol, p["id"].as_i64().unwrap_or(-1)).map_err(fail)?)
    }

    /// An export of the folder: `{root, kind, matrix?, generated?, files}` answers `{files, warnings}` (see `mzlab_feat::attributio::export::run`).
    pub(crate) fn feat_attributio_export(&self, p: &Value) -> Result<Answer> {
        let files = mem_files(p);
        let read = |q: &str| files.read(q);
        let list = |d: &str| files.list(d);
        let io = Io::new(&read, &list);
        let (root, kind) = (
            p["root"].as_str().unwrap_or(""),
            p["kind"].as_str().unwrap_or(""),
        );
        let o = Options {
            matrix: p["matrix"].as_str().map(str::to_string),
            generated: p["generated"].as_str().unwrap_or("").to_string(),
        };
        let out = if kind == "evidence" {
            evidence_json(&io, root)
        } else {
            run(&io, root, kind, &o)
        };
        text(out.map_err(|e| Error::new("error.attributio").with("detail", e.to_string()))?)
    }

    pub(crate) fn feat_attributio(&self, p: &Value) -> Result<Answer> {
        let files = mem_files(p);
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
        o.raw("{\"summary\":")
            .raw(&run.summary())
            .raw(",\"text\":")
            .str(&body)
            .raw("}");
        text(o.0)
    }
}

/// `path` (relative, inside the folder) written under `folder`; refuses anything that leaves it.
fn write_in(folder: &str, path: &str, text: &str) -> Result<bool> {
    let rel = std::path::Path::new(path);
    let inside = rel
        .components()
        .all(|c| matches!(c, std::path::Component::Normal(_)));
    if folder.is_empty() || path.is_empty() || !inside {
        return Ok(false);
    }
    let dest = std::path::Path::new(folder).join(rel);
    if let Some(dir) = dest.parent() {
        std::fs::create_dir_all(dir).map_err(|_| Error::new("error.io"))?;
    }
    std::fs::write(&dest, text).map_err(|_| Error::new("error.io"))?;
    Ok(true)
}

/// The text files of the folder the page sent.
fn mem_files(p: &Value) -> MemFiles {
    let mut files = MemFiles::default();
    for f in p["files"].as_array().into_iter().flatten() {
        files.add(
            f["path"].as_str().unwrap_or(""),
            f["text"].as_str().unwrap_or("").as_bytes(),
        );
    }
    files
}
