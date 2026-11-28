//! `mzlab-cli table pair`: the feature tables of the positive and the negative method (the CSV of `mzlab-cli features` or a `.mzfeat`) paired by neutral
//! mass and RT (`mzlab_core::polarity_pair`, the engine of the Flow step `features.pair`: same bytes as the site's `.wasm`). The pairs are the standard
//! output as CSV (or `pair/1` with `--json`); the rows without a partner are listed with them; the rule and every automatic choice are in the document.

use std::path::PathBuf;

use mzlab_core::error::Error;
use mzlab_core::json::{parse, Value};
use mzlab_core::polarity_pair::{run, Params};

use crate::samples_import::say;
use crate::Outcome;

pub const HELP: &str = "mzlab-cli table pair: the features of a positive and a negative method paired by neutral mass and RT, to see which species show in both polarities.

usage: mzlab-cli table pair POS.csv NEG.csv [--set NAME=VALUE...] [--json] [--out PATH]

  POS, NEG         the feature table of each method: the CSV of `mzlab-cli features` (columns id, mz, rt; the others are not read) or a `.mzfeat`
  --set NAME=VALUE a parameter by its name: rt_tol (min, default 0.1; the two RT axes are compared as they are, with no map), mass_tol (default 5),
                   mass_unit (ppm, the default, of the neutral mass; or da, for unit-mass tables),
                   pos_adducts and neg_adducts (comma-separated: names of the adduct table such as [M+Na]+ or [M+HCOO]-, or LABEL:SHIFT, the mass the
                   adduct adds to M for a singly charged ion; defaults [M+H]+ and [M-H]-)
  --json           pair/1 on stdout: params, counts, pairs, unmatched rows, the adducts and rules used, csv
  --out PATH       write the output there instead of the standard output

Output: CSV pos_id,neg_id,rule,neutral_mass,err_da,err_ppm,rt_pos,rt_neg,drt,mutual, sorted by neutral mass, then the rows without a partner (rule `unmatched`).
Every pair within both tolerances is listed, none is merged; `mutual` marks the pair that is the nearest partner of both rows. Errors are negative minus positive.
Stderr: a summary and the adducts and tolerances used. Nothing is refused for a finding; nothing is identified.
Exit codes: 0 ok; 2 usage (bad option or parameter value); 3 a table that cannot be used (the row and the field are named); 4 a file missing or unreadable; 6 --out cannot be written.";

fn usage(msg: &str) -> Outcome {
    Outcome::fail_error(&Error::new("err.cli.usage").with("message", msg))
}

/// The CSV of one side: a `.mzfeat` (by its magic) is written as the CSV of `mzlab-cli features`, anything else is read as text.
fn read(path: &str) -> Result<(String, String), Outcome> {
    let bytes = std::fs::read(path).map_err(|_| {
        Outcome::fail_error(&Error::new("err.session.fileNotFound").with("name", path))
    })?;
    let name = std::path::Path::new(path)
        .file_name()
        .map_or(path.to_string(), |n| n.to_string_lossy().into_owned());
    let text = if bytes.starts_with(b"MZFEAT01") {
        mzlab_feat::union::series_csv(&bytes).map_err(|e| Outcome::fail_error(&e))?
    } else {
        String::from_utf8_lossy(&bytes).into_owned()
    };
    Ok((name, text))
}

fn text<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k).and_then(Value::str).unwrap_or("")
}

fn number(v: &Value, k: &str) -> String {
    v.get(k)
        .and_then(Value::f64)
        .map_or(String::new(), |n| n.to_string())
}

/// The stderr lines of a document: the summary, the tolerances and the adducts of each side (texts: `err.cli.pair.*` of `lang/en`).
fn lines(doc: &Value) -> String {
    let counts = doc.get("counts").cloned().unwrap_or(Value::Null);
    let params = doc.get("params").cloned().unwrap_or(Value::Null);
    let c = |k: &str| number(&counts, k);
    let mut out = say(
        "err.cli.pair.summary",
        &[
            ("pos", c("pos_rows")),
            ("neg", c("neg_rows")),
            ("pairs", c("pairs")),
            ("mutual", c("mutual")),
            ("pos_free", c("pos_unmatched")),
            ("neg_free", c("neg_unmatched")),
        ],
    ) + "\n";
    out += &(say(
        "err.cli.pair.tolerance",
        &[
            ("rt", number(&params, "rt_tol")),
            ("mass", number(&params, "mass_tol")),
            ("unit", text(&params, "mass_unit").to_string()),
        ],
    ) + "\n");
    for (side, k) in [("+", "pos_adducts"), ("-", "neg_adducts")] {
        out += &(say(
            "err.cli.pair.adducts",
            &[
                ("side", side.to_string()),
                ("list", text(&params, k).to_string()),
            ],
        ) + "\n");
    }
    out
}

pub(crate) fn run_cmd(args: Vec<String>) -> Outcome {
    let json = args.iter().any(|a| a == "--json");
    let p = match crate::extract::parse(args, &["--set", "--out"], &[]) {
        Ok(Some(p)) => p,
        Ok(None) => return Outcome::ok(format!("{HELP}\n").into_bytes()),
        Err(e) => return Outcome::fail(&e),
    };
    if p.pos.len() != 2 {
        return usage(
            "table pair needs two tables, the positive one first: mzlab-cli table pair --help",
        );
    }
    let mut params: Vec<(String, Value)> = Vec::new();
    for s in p.all("--set") {
        let Some((k, v)) = s.split_once('=') else {
            return usage(&format!("--set takes NAME=VALUE, not {s}"));
        };
        params.push((k.trim().to_string(), Value::Str(v.to_string())));
    }
    let out: Option<PathBuf> = p.out();
    if let Some(o) = out.as_ref().map(|o| o.to_string_lossy().into_owned())
        && p.pos
            .iter()
            .any(|i| crate::samples_import::same_file(i, &o))
    {
        return usage(&format!(
            "--out {o} is also an input: an input is never overwritten"
        ));
    }
    let doc = match (read(&p.pos[0]), read(&p.pos[1])) {
        (Ok(a), Ok(b)) => {
            Params::from_json(Some(&Value::Obj(params))).and_then(|q| run(&a, &b, &q))
        }
        (Err(o), _) | (_, Err(o)) => return o,
    };
    let doc = match doc {
        Ok(d) => d,
        Err(e) => return Outcome::fail_error(&e),
    };
    let Ok(v) = parse(doc.as_bytes()) else {
        return usage("the pairing answered with a document that is not JSON");
    };
    let body = if json {
        let head = doc.trim_end().strip_suffix('}').unwrap_or(&doc);
        format!("{head},{}}}\n", crate::citation::member())
    } else {
        text(&v, "csv").to_string()
    };
    let mut done = crate::emit(body, &out);
    done.stderr = lines(&v);
    done
}

#[cfg(test)]
#[path = "table_pair_cmd_tests.rs"]
mod tests;
