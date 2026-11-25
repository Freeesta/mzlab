//! `spectrum get --sum [--fill-ms T]`: the scans of a window added instead of averaged (the engine's `mode=sum`), with the evidence of how many scans and
//! ions the spectrum rests on (`/api/specsum`, the route the site and the app answer too); and the hint the mean of a window carries to point at it.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, Value};

use crate::extract::{self, Parsed, Sample};
use crate::spectrum_cmd::{query, At};

/// `--sum`: the scans of the window are added; `fill_ms` is the fill time the user states, as given.
pub(crate) struct SumOpt {
    pub fill_ms: Option<String>,
}

/// `--sum` and `--fill-ms` of a request: `None` without `--sum`; a usage message for a request the sum cannot answer.
pub(crate) fn options(
    p: &Parsed,
    at: &At,
    has_blank: bool,
) -> std::result::Result<Option<SumOpt>, String> {
    if !p.flag("--sum") {
        return match p.get("--fill-ms") {
            Some(_) => Err("--fill-ms goes with --sum".into()),
            None => Ok(None),
        };
    }
    if !matches!(at, At::Window(..)) {
        return Err("--sum adds the scans of a window: give --rt A,B".into());
    }
    if has_blank {
        return Err("--sum is not for --minus (a sum has no blank subtracted)".into());
    }
    p.num("--fill-ms")?;
    Ok(Some(SumOpt {
        fill_ms: p.get("--fill-ms").map(String::from),
    }))
}

/// The evidence of a sum (`/api/specsum` of the same window): the one route of the site, the app and the CLI.
pub(crate) fn evidence(
    s: &mut Sample,
    pairs: &[(&str, String)],
    fill: &Option<String>,
) -> Result<String> {
    let mut q: Vec<(&str, String)> = pairs
        .iter()
        .filter(|(k, _)| matches!(*k, "rt0" | "rt1" | "level" | "precursor" | "channel"))
        .cloned()
        .collect();
    if let Some(f) = fill {
        q.push(("fill_ms", f.clone()));
    }
    mzlab_core::rawinfo::handle(&mut s.routes, "/api/specsum", &query(&q))?
        .ok_or_else(|| Error::new("err.scan.none"))
}

/// The text of a note key for the terminal.
fn note_text(key: &str) -> String {
    crate::error_text::catalog_text(key)
        .unwrap_or(key)
        .to_string()
}

/// What follows the evidence of `spectrum get` (a member to add to it) and the lines for the terminal: the sum's evidence and the notes it carries,
/// or, for the mean of a window of two scans or more, the hint at `--sum` (in the evidence for JSON, on the terminal for CSV).
pub(crate) fn tail(
    sum: Option<&str>,
    at: &At,
    csv: bool,
    answer: &str,
) -> Result<(String, String)> {
    let (mut extra, mut notes) = (String::new(), String::new());
    if let Some(e) = sum {
        let v = parse(e.as_bytes())?;
        let fill = v.get("fill_time").and_then(|f| f.get("note"));
        for k in [fill, v.get("ions_by_quantum_note")].into_iter().flatten() {
            if let Some(k) = k.str() {
                notes += &format!("note: {}\n", note_text(k));
            }
        }
        extra = format!(",\"sum\":{e}");
    } else if matches!(at, At::Window(..)) {
        let n = parse(answer.as_bytes())?
            .get("scans")
            .and_then(Value::f64)
            .unwrap_or(0.0);
        if n >= 2.0 {
            let k = "note.spec.mean.sumHint";
            if csv {
                notes += &format!("hint: {}\n", note_text(k));
            } else {
                extra = format!(
                    ",\"hint\":{{\"key\":\"{k}\",\"text\":{},\"command\":\"spectrum get FILE --rt A,B --sum\"}}",
                    extract::jstr(&note_text(k))
                );
            }
        }
    }
    Ok((extra, notes))
}
