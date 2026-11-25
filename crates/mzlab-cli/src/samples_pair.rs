//! `mzlab-cli sample pair`: the check of the pairing of a `samples/1` with the files of a series (`samples::order_for_roles`, the function the
//! transformation-product steps run), so the automatic role choices are seen before a run. It writes nothing.

use std::path::Path;

use mzlab_core::error::Error;
use mzlab_core::json::Out;

use crate::Outcome;

pub const HELP: &str = "mzlab-cli sample pair: how a samples/1 pairs with the files of a series, as the transformation-product steps (tp.dig_lr, tp.link, tp.network) pair it (sample-pair/1).

usage: mzlab-cli sample pair SAMPLES.json [FILE...] [--table TABLE.mzfeat] [--json]

  SAMPLES.json   the samples/1 (`sample table`, `sample import`)
  FILE...        the files of the series, in the order of the table (or --table, one of the two); only their names are used, they need not exist
  --table T      the files of an aligned table (.mzfeat) instead
  --json         the sample-pair/1 document on stdout ({schema, status, samples, pairing}); without it the paired samples/1 alone, so `> samples.json` works as with `sample table`
A sample whose file is not in the series is left out; a file with no sample takes the role its name proposes. Nothing is refused for it: every choice is a `pairing` entry
({action: left_out|proposed, file, role, evidence}) and one line on stderr; edit `kind` in the samples/1 (or the CSV of `sample import`) and run it again.
Exit codes: 0 every file has its sample and no sample is left over; 1 automatic choices were made (the document is still written); 2 usage; 3 the samples file is not a valid samples/1; 4 a file missing or unreadable.";

/// The names of the files of the series: the file names of the arguments, or the inputs of the table.
fn series(files: &[String], table: Option<&str>) -> Result<Vec<String>, Outcome> {
    let Some(t) = table else {
        let name = |f: &String| Path::new(f).file_name().map_or(f.clone(), |n| n.to_string_lossy().into_owned());
        return Ok(files.iter().map(name).collect());
    };
    let bytes = std::fs::read(t).map_err(|_| missing(t))?;
    mzlab_feat::store::table_names(&mut mzlab_feat::store::Slice(&bytes), bytes.len() as u64)
        .map_err(|e| Outcome::fail_error(&e))
}

fn missing(path: &str) -> Outcome {
    Outcome::fail_error(&Error::new("err.session.fileNotFound").with("name", path))
}

/// `sample import` and `sample pair`: the two routes that lay a table or a pairing over the samples (one arm of the dispatch of `samples_cmd`).
pub(crate) fn route(sub: &str, args: Vec<String>) -> Outcome {
    if sub == "import" {
        crate::samples_import::run(args)
    } else {
        run(args)
    }
}

pub(crate) fn run(args: Vec<String>) -> Outcome {
    let json = args.iter().any(|a| a == "--json");
    let rest: Vec<String> = args.into_iter().filter(|a| a != "--json").collect();
    let p = match crate::extract::parse(rest, &["--table"], &[]) {
        Ok(Some(p)) => p,
        Ok(None) => return Outcome::ok(format!("{HELP}\n").into_bytes()),
        Err(e) => return Outcome::fail(&e),
    };
    let table = p.get("--table");
    let Some((samples, files)) = p.pos.split_first() else {
        return Outcome::fail_error(&Error::new("err.cli.pair.needsInput"));
    };
    if table.is_some() == !files.is_empty() {
        return Outcome::fail_error(&Error::new("err.cli.pair.oneSource"));
    }
    let text = match std::fs::read_to_string(samples) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::InvalidData => {
            return crate::samples_import::engine(&Error::new("error.badSamples"))
        }
        Err(_) => return missing(samples),
    };
    let names = match series(files, table) {
        Ok(n) => n,
        Err(o) => return o,
    };
    let paired = match mzlab_core::samples::order_for_roles(&text, &names) {
        Ok(p) => p,
        Err(e) => {
            let hint = crate::flow_digl::pairnotes::hint(e.key);
            return crate::samples_import::engine(&e).with_hint(&hint);
        }
    };
    let shown = if json {
        let mut o = Out::default();
        o.raw("{\"schema\":\"sample-pair/1\",\"status\":")
            .str(if paired.notes.is_empty() { "complete" } else { "adjusted" })
            .raw(",\"samples\":")
            .raw(&paired.text)
            .raw(",\"pairing\":");
        let pairing = mzlab_core::samples::notes_json(&paired.notes);
        o.raw(if pairing.is_empty() { "[]" } else { &pairing }).raw("}");
        o.0
    } else {
        paired.text.clone()
    };
    let mut stderr = String::new();
    for l in crate::flow_digl::pairnotes::lines(&paired.notes) {
        stderr += &format!("{l}\n");
    }
    Outcome {
        code: u8::from(!paired.notes.is_empty()),
        stdout: format!("{shown}\n").into_bytes(),
        stderr,
        error: None,
    }
}

#[cfg(test)]
#[path = "samples_pair_tests.rs"]
pub(crate) mod tests;
