//! `structure.moiety`: the rows of a candidate table (a CSV with a SMILES column) whose structure contains a moiety, or lacks it, with the number of its
//! matches as a last column `moiety_matches`. The matcher is the one of `substructure.rs` (`mzlab_mol::SubQuery`); a row kept is still a candidate, never an
//! identification. Rows without a structure or with one that cannot be read are left out either way. The same bytes from `flow run` and the `molkit` `.wasm`.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::Value;
use mzlab_mol::{read_target, SubOptions, SubQuery};

/// Most rows of a table.
const MAX_ROWS: usize = 200_000;

/// The step's parameters: the moiety, the column of the SMILES, whether stereo must match and which rows stay.
struct Params {
    query: String,
    column: String,
    stereo: bool,
    lacking: bool,
}

fn read_params(v: &Value) -> Result<Params> {
    let bad = || Error::new("error.sub.request");
    let s = |k: &str, d: &str| {
        v.get(k)
            .and_then(|x| x.str())
            .unwrap_or(d)
            .trim()
            .to_string()
    };
    let keep = s("keep", "containing");
    if keep != "containing" && keep != "lacking" {
        return Err(bad());
    }
    Ok(Params {
        query: s("query", ""),
        column: s("column", "smiles"),
        stereo: v.get("stereo").and_then(|x| x.bool()).unwrap_or(false),
        lacking: keep == "lacking",
    })
}

/// A cell written back: in double quotes when it holds the delimiter, a quote or a line break, or starts with `#` (a comment line otherwise).
fn cell(s: &str, d: char) -> String {
    if s.contains([d, '"', '\n', '\r']) || s.starts_with('#') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// One row written back with `width` cells at least (a short row gets empty ones), then `extra`.
fn line(out: &mut String, cells: &[String], width: usize, extra: &str, d: char) {
    for k in 0..width.max(cells.len()) {
        out.push_str(&cell(cells.get(k).map_or("", |c| c.as_str()), d));
        out.push(d);
    }
    out.push_str(extra);
    out.push('\n');
}

/// The table of the rows that contain (or lack) the moiety, in their order, with the delimiter of the input and `moiety_matches` added. `params` is
/// the object of the step's parameters (`query`, `column`, `stereo`, `keep`).
pub fn moiety_text(params: &Value, table: &[u8]) -> Result<String> {
    let p = read_params(params)?;
    if p.query.is_empty() {
        return Err(Error::new("error.sub.query").with("detail", "empty"));
    }
    let q = SubQuery::from_text(&p.query)
        .map_err(|e| Error::new("error.sub.query").with("detail", e))?;
    let text = std::str::from_utf8(table).map_err(|_| Error::new("error.sub.db"))?;
    let rows = mzlab_core::molecules::table(text);
    let Some(((_, head), body)) = rows.split_first() else {
        return Err(Error::new("error.sub.db"));
    };
    let Some(col) = head.iter().position(|h| h.eq_ignore_ascii_case(&p.column)) else {
        return Err(Error::new("error.sub.column").with("column", p.column.as_str()));
    };
    if body.len() > MAX_ROWS {
        return Err(Error::new("error.sub.dbSize")
            .with("n", body.len().to_string())
            .with("max", MAX_ROWS.to_string()));
    }
    let opts = SubOptions {
        stereo: p.stereo,
        max_matches: 1000,
        ..SubOptions::default()
    };
    let d = mzlab_core::molecules::delimiter(text);
    let mut out = String::new();
    line(&mut out, head, 0, "moiety_matches", d);
    for (_, r) in body {
        let smiles = r.get(col).map_or("", |s| s.as_str());
        if smiles.is_empty() {
            continue;
        }
        let Ok(n) = read_target(smiles)
            .and_then(|m| q.matches(&m, &opts))
            .map(|m| m.len())
        else {
            continue;
        };
        if (n == 0) == p.lacking {
            line(&mut out, r, head.len(), &n.to_string(), d);
        }
    }
    Ok(out)
}

#[cfg(test)]
#[path = "moiety_tests.rs"]
mod tests;
