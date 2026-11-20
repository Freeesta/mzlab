//! The headless door: a request with a starting document or text, a command log and the output wanted gives the output text. The
//! `.wasm`, the app and the CLI call this one function, so a session of the editor replays to the same bytes anywhere.

use crate::editor::Editor;
use crate::ops::{Op, OpError};
use crate::svg;
use serde::Deserialize;

#[derive(Deserialize)]
struct Request {
    /// A structure text to start from: `mzdraw/1` JSON, a molfile or a SMILES.
    #[serde(default)]
    import: Option<String>,
    #[serde(default)]
    log: Vec<Op>,
    /// `mzdraw` (default), `molfile`, `svg` or `log`.
    #[serde(default)]
    out: Option<String>,
}

/// Runs a request `{"import", "log", "out"}`; the error is an i18n key and the id it is about.
pub fn replay(request: &str) -> Result<String, OpError> {
    let fail = |key| OpError { key, id: 0 };
    let r: Request = serde_json::from_str(request).map_err(|_| fail("error.draw2.request"))?;
    let mut e = Editor::default();
    if let Some(text) = &r.import {
        e.import(text).map_err(fail)?;
    }
    for op in &r.log {
        e.session.run(std::slice::from_ref(op))?;
    }
    Ok(match r.out.as_deref().unwrap_or("mzdraw") {
        "mzdraw" => e.doc().to_json(),
        "molfile" => e.doc().to_molfile(),
        "svg" => svg::to_svg(e.doc()),
        "log" => e.session.log_json(),
        _ => return Err(fail("error.draw2.request")),
    })
}
