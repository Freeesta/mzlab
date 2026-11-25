//! `mzlab-cli file method FILE [--sample LIST] [--json] [--out PATH]`: the acquisition parameters of the tandem-MS segments of a file (`file_method/1`):
//! precursor, collision energy and its spread, declustering potential, fill time, Q1 resolution, each `null` with its reason when the format does not hold it,
//! and the check of a mass in the name against the precursors (a warning, never a blocked result). The document is `mzlab_core::filemethod`, the same bytes the site gives.

use std::path::PathBuf;

use mzlab_core::error::Result;
use mzlab_core::json::{fmt_g, parse, Value};

use crate::Outcome;

pub const HELP: &str = "mzlab-cli file method: per sample and scan segment, the precursor, collision energy (CE), its spread (CES), declustering potential (DP), fill time and Q1 resolution a file holds, as JSON (file_method/1).

usage: mzlab-cli file method FILE [--sample LIST] [--json] [--out PATH]

  FILE        an mzML or any single file the app reads (.wiff, .raw, ...)
  --sample LIST the samples of a file with several, by id (1,3 or 2-4): one document per line, in the order of the file; default the first; an id the file
              does not have: exit 5
  --json      accepted for scripts: the output is always JSON
  --out PATH  write there instead of the standard output

segments[]: experiment, scan, ms_level, precursor_mz, q3_mz, mz_range, ce_eV, ce_list_eV, ces_eV, dp_V, dwell_ms, fill_time_ms, q1_resolution.
A parameter the file does not give is null and `unavailable` says why: not_decoded (the format holds it, mzLab does not read it yet), not_in_method
(the method has no such parameter), not_in_file (the format has no field for it). `acquired` is what the scans hold (precursor, CE, scan count).
warnings[] names a mass in the file or sample name that is not within 1 m/z of any precursor (also on the standard error); nothing is blocked.
exit codes: 0 ok, 2 usage, 4 file unreadable, 5 unknown sample, 6 no usable data";

struct Request {
    file: PathBuf,
    sample: Option<Vec<u32>>,
    out: Option<PathBuf>,
}

fn parse_args(args: Vec<String>) -> std::result::Result<Option<Request>, String> {
    let (mut file, mut out, mut sample) = (None, None, None);
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--help" | "-h" => return Ok(None),
            "--json" => {}
            "--out" => out = Some(PathBuf::from(it.next().ok_or("--out needs a path")?)),
            "--sample" => {
                let v = it.next().ok_or("--sample needs a sample id or list")?;
                sample = Some(
                    crate::sample_select::parse(&v)
                        .map_err(|e| crate::error_text::human_error(&e))?,
                )
            }
            o if o.starts_with("--") => return Err(format!("unknown option {o}")),
            _ if file.is_none() => file = Some(PathBuf::from(a)),
            _ => return Err("method takes one file".into()),
        }
    }
    if sample.is_some() && file.as_ref().is_some_and(|f| crate::extract::is_mzml(f)) {
        return Err(
            "--sample is for a file of several samples (.wiff, ...), an mzML holds one".into(),
        );
    }
    Ok(Some(Request {
        file: file.ok_or("method needs a file")?,
        sample,
        out,
    }))
}

/// The document of one sample and the line of its name warnings for stderr.
fn read(c: &Request, id: Option<u32>, tmp: &std::path::Path) -> Result<(String, String)> {
    let mut s = crate::extract::open_sample(&c.file, id, tmp)?;
    let items = mzlab_core::lr::open_items(
        &mut s.routes.idx,
        &mut s.mzml,
        &s.name,
        &mzlab_core::lr::Sample::default(),
    )?;
    let items = crate::items_cmd::document(&items, true)?;
    let method = s
        .mzml
        .method(&s.name, mzlab_core::method_fit::DOC_MAX as u64)?;
    let doc = mzlab_core::filemethod::document(s.sample, method.as_deref(), &items)?;
    Ok((doc.clone(), s.note + &warning_lines(&doc)))
}

/// The stderr line that points `file items` and `sample list` at `file method` (the parameters of the tandem segments are not in their documents).
pub(crate) fn hint_line(file: &str, more: &str) -> String {
    format!("hint: precursor, collision energy (CE, CES), declustering potential and fill time of each segment: mzlab-cli file method {file}{more}\n")
}

/// The warnings of a document as terminal lines (catalogue text with the numbers).
pub(crate) fn warning_lines(doc: &str) -> String {
    let Ok(v) = parse(doc.as_bytes()) else {
        return String::new();
    };
    let num = |w: &Value, k: &str| w.get(k).and_then(Value::f64).map_or(String::new(), fmt_g);
    v.get("warnings")
        .map(Value::arr)
        .unwrap_or_default()
        .iter()
        .map(|w| {
            let key = w.get("key").and_then(Value::str).unwrap_or("");
            let par = vec![
                (
                    "name".to_string(),
                    w.get("name").and_then(Value::str).unwrap_or("").to_string(),
                ),
                ("name_mz".to_string(), num(w, "nearest_name_mz")),
                ("precursor".to_string(), num(w, "nearest_precursor")),
                ("delta".to_string(), num(w, "delta_mz")),
            ];
            format!("warning: {}\n", crate::error_text::notice_text(key, par))
        })
        .collect()
}

pub fn run(args: Vec<String>) -> Outcome {
    let c = match parse_args(args) {
        Ok(Some(a)) => a,
        Ok(None) => return Outcome::ok(format!("{HELP}\n").into_bytes()),
        Err(e) => return Outcome::fail(&e),
    };
    let io = crate::extract::io;
    let r = crate::extract::with_tmp("method", |tmp| {
        let mzml = crate::extract::is_mzml(&c.file);
        // the whole list is checked before the first sample is read: no partial answer for a list with a wrong id
        let ids: Vec<Option<u32>> = match &c.sample {
            Some(w) if !mzml => crate::sample_select::resolve(&c.file, w)?
                .into_iter()
                .map(Some)
                .collect(),
            _ => vec![None],
        };
        let (mut text, mut notes) = (Vec::new(), String::new());
        for (n, id) in ids.into_iter().enumerate() {
            let sub = tmp.join(n.to_string());
            std::fs::create_dir_all(&sub).map_err(io)?;
            let (t, note) = read(&c, id, &sub)?;
            text.push(t);
            notes += &note;
        }
        Ok((text.join("\n"), notes))
    });
    match r {
        Err(e) => crate::sample_select::fail(&e),
        Ok((t, note)) => {
            let mut o = crate::emit(t + "\n", &c.out);
            if o.code == 0 {
                o.stderr = note;
            }
            o
        }
    }
}

#[cfg(test)]
#[path = "method_tests.rs"]
mod tests;
