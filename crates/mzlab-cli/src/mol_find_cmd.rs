//! `mzlab-cli mol find QUERY [--db CSV|-] [--user FILE] [--set limit=N ppm=X wide=1] [--out PATH]`: the compound search box of the pages, from the command line. The query is a
//! name, a synonym, a formula or a mass; the answer is the JSON of the route `api/molecules/search` (`molecules::route`), byte for byte what the `.wasm` and the app give.
//! Without `--db` the built-in environmental catalogue is searched (the file the pages load, compiled in).

use std::borrow::Cow;
use std::path::PathBuf;

use mzlab_core::json::Out;
use mzlab_core::molecules::{from_csv, from_json, route, to_json, RowError};

use crate::Outcome;

/// The catalogue of the pages (`mzlab/web/molecules/catalogue.csv`), the one list of the site, the app and the CLI.
const CATALOGUE: &str = include_str!("../../../mzlab/web/molecules/catalogue.csv");

pub const HELP: &str = "mzlab-cli mol find: search the compound catalogue by name, synonym, formula or mass, as JSON.

usage: mzlab-cli mol find QUERY [--db CSV|-] [--user FILE] [--set NAME=VALUE...] [--out PATH]

  QUERY         a name or synonym (2 characters at least), a molecular formula (Hill order or not) or a mass (a number)
  --db CSV      a molecules/1 CSV instead of the built-in environmental catalogue; - reads it from the standard input
  --user FILE   also search the molecules you saved: a molecules/1 JSON document or a CSV (the page's export); a molecule with the id of a catalogue
                one replaces it. Every hit then carries a \"level\" key: catalog, imported or user
  --set limit   at most this many answers (1-1000, default 8)
  --set ppm     the window of a mass search in ppm (default 10)
  --set wide    1 = also whole InChIKeys, their first block, and names one letter away (default 0)
  --out PATH    write there instead of the standard output

The answer is a JSON array, best first, of {id, name, formula, charge, mass, smiles, adducts, score}: the score is 0 for an equal name or
formula, 1 starts with, 2 starts a word, 3 contains, 4-5 the wide matches; for a mass it is the deviation in ppm. An empty array is exit 0.
Names that start with the text come before names that only contain it; at the same rank the shorter name comes first.";

/// The value of `--set NAME=VALUE` that must be a whole number in `lo..=hi`.
fn number(v: &str, name: &str, lo: u32, hi: u32) -> Result<u32, String> {
    v.parse::<u32>()
        .ok()
        .filter(|n| (lo..=hi).contains(n))
        .ok_or_else(|| format!("--set {name} needs a whole number from {lo} to {hi}, not {v:?}"))
}

/// The catalogue to search: the one built into the binary, or the CSV of `--db` (a path, or `-` for the standard input), checked row by row.
fn catalogue_text(db: Option<&str>) -> Result<Cow<'static, str>, Outcome> {
    let Some(path) = db else {
        return Ok(Cow::Borrowed(CATALOGUE));
    };
    let mut text = String::new();
    if path == "-" {
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut text)
            .map_err(|e| Outcome::fail(&e.to_string()))?;
    } else {
        text = std::fs::read_to_string(path).map_err(|e| Outcome::fail(&format!("{path}: {e}")))?;
    }
    if let Err(rows) = from_csv(&text, "catalog", None) {
        let e = &rows[0];
        return Err(Outcome::fail(&format!(
            "{} line {}: the catalogue has {} wrong rows ({})",
            e.key,
            e.line,
            rows.len(),
            e.detail
        )));
    }
    Ok(Cow::Owned(text))
}

/// The molecules of `--user FILE` as a `molecules/1` document: a JSON document is read as it is (each record keeps its level), a CSV gets the level `user`.
fn user_document(path: &str) -> Result<String, Outcome> {
    let text = std::fs::read_to_string(path).map_err(|e| Outcome::fail(&format!("{path}: {e}")))?;
    let rows: Result<_, Vec<RowError>> = if text.trim_start().starts_with('{') {
        from_json(text.as_bytes())
    } else {
        from_csv(&text, "user", None)
    };
    match rows {
        Ok(ms) => Ok(to_json(&ms)),
        Err(rows) => {
            let e = &rows[0];
            Err(Outcome::fail(&format!(
                "{} line {}: {path} has {} wrong rows ({})",
                e.key,
                e.line,
                rows.len(),
                e.detail
            )))
        }
    }
}

/// One `--set NAME=VALUE`: limit, ppm or wide.
fn setting(
    kv: Option<&str>,
    limit: &mut u32,
    ppm: &mut String,
    wide: &mut bool,
) -> Result<(), String> {
    match kv.and_then(|kv| kv.split_once('=')) {
        Some(("limit", v)) => *limit = number(v, "limit", 1, 1000)?,
        Some(("ppm", v)) => match v.parse::<f64>() {
            Ok(x) if x.is_finite() && x > 0.0 => *ppm = v.to_string(),
            _ => return Err(format!("--set ppm needs a positive number, not {v:?}")),
        },
        Some(("wide", v)) => *wide = number(v, "wide", 0, 1)? == 1,
        Some((k, _)) => return Err(format!("unknown setting {k} (limit, ppm, wide)")),
        None => return Err("--set needs NAME=VALUE".to_string()),
    }
    Ok(())
}

pub fn run(args: Vec<String>) -> Outcome {
    let (mut pos, mut out, mut db, mut user) = (Vec::new(), None, None, None);
    let (mut limit, mut ppm, mut wide) = (8u32, "10".to_string(), false);
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--help" | "-h" => return Outcome::ok(format!("{HELP}\n").into_bytes()),
            "--db" => match it.next() {
                Some(p) => db = Some(p),
                None => return Outcome::fail("--db needs a path (or - for the standard input)"),
            },
            "--user" => match it.next() {
                Some(p) => user = Some(p),
                None => return Outcome::fail("--user needs a path"),
            },
            "--out" => match it.next() {
                Some(p) => out = Some(PathBuf::from(p)),
                None => return Outcome::fail("--out needs a path"),
            },
            "--set" => {
                if let Err(e) = setting(it.next().as_deref(), &mut limit, &mut ppm, &mut wide) {
                    return Outcome::fail(&e);
                }
            }
            o if o.starts_with("--") => return Outcome::fail(&format!("unknown option {o}")),
            _ => pos.push(a),
        }
    }
    let [query] = pos.as_slice() else {
        return Outcome::fail("mol find needs one query (a name, a formula or a mass)");
    };
    let csv = match catalogue_text(db.as_deref()) {
        Ok(c) => c,
        Err(o) => return o,
    };
    let mut load = Out::default();
    load.raw("{\"catalog\":").str(&csv);
    // Always sent, empty without `--user`: the engine keeps the last user list between calls of one process.
    let doc = match &user {
        Some(path) => match user_document(path) {
            Ok(doc) => doc,
            Err(o) => return o,
        },
        None => to_json(&[]),
    };
    load.raw(",\"user\":").str(&doc);
    load.raw("}");
    let mut ask = Out::default();
    ask.raw("{\"q\":").str(query).raw(&format!(
        ",\"limit\":{limit},\"ppm\":{ppm},\"wide\":{wide},\"levels\":{}}}",
        user.is_some()
    ));
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner()); // the archive of the engine is one per process
    if let Err(e) = route("load", &load.0) {
        return Outcome::fail_error(&e);
    }
    match route("search", &ask.0) {
        Ok(j) => crate::emit(format!("{j}\n"), &out),
        Err(e) => Outcome::fail_error(&e),
    }
}

static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
#[path = "mol_find_tests.rs"]
mod tests;
