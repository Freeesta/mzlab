//! `mzlab-cli table compare`: every feature of a time-course table against its reference (t0) and its blank columns, to flag candidate transformation products.
//! The JSON is `compare/1`, with no final newline, the same bytes the Flow step `table.compare@1` writes (`mzlab_feat::flow_compare`); `--format csv` is one row per feature of the same
//! document; `--dry-run` prints `compare-plan/1` (the columns of every role with their times, the parameters and the choices) and computes nothing.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, Value};
use mzlab_core::jsonb::{num, text, write};

use crate::extract;
use crate::Outcome;

pub const HELP: &str = "mzlab-cli table compare: each feature against the reference (t0) and the blank, to flag candidate transformation products.

usage: mzlab-cli table compare TABLE --ref SEL --blank SEL [--samples FILE.json] [--quantity area|height] [--factor F] [--pseudocount auto|N]
                               [--format json|csv] [--json] [--dry-run] [--out PATH]

  TABLE         the `features` CSV (mzmine full table, areas) or a .mzfeat
  --ref SEL     the reference columns; --blank SEL the blank columns. SEL: comma list of case-insensitive globs over the column names (t0*, MQ*,blank*),
                or role:blank, role:reference, time:0 (these need --samples). Every other column is treated. A column in both lists is exit 2
  --samples F   a samples/1 document (`sample table`): the roles and times; without it the times are parsed from the column names
  --quantity Q  area (default) or height (.mzfeat only)
  --factor F    ratio a feature needs against the reference and the blank (default 3)
  --pseudocount auto (default: 3 x the median of the positive blank values, else of the reference, else 1000) or a number
  --format F    json (default, compare/1) or csv (one row per feature, reasons joined by ;)
  --json        the same as --format json (it cannot be given with --format csv)
  --dry-run     print compare-plan/1 and compute nothing
  --out PATH    write there instead of the standard output

Class (sorts and filters only, every row is kept): no_signal, blank_artefact, candidate, decreasing, unchanged; the first rule that matches decides (see `choices`).
Exit codes: 0 ok; 2 usage (bad option, --json with --format csv, a column in both lists, a role: selector without --samples); 4 table or samples missing or unreadable, or two columns with one name; 5 selector matches no column; 6 --out cannot be written.";

struct Job {
    table: std::path::PathBuf,
    samples: Option<std::path::PathBuf>,
    params: Vec<(String, Value)>,
    csv: bool,
    dry: bool,
    out: Option<std::path::PathBuf>,
}

fn job(args: Vec<String>) -> std::result::Result<std::result::Result<Job, &'static str>, String> {
    let valued = [
        "--ref",
        "--blank",
        "--samples",
        "--quantity",
        "--factor",
        "--pseudocount",
        "--format",
        "--out",
    ];
    // `parse` drops `--json` (it is the default here); it only matters next to `--format csv`
    let json = args.iter().any(|a| a == "--json");
    let Some(p) = extract::parse(args, &valued, &["--dry-run"])? else {
        return Ok(Err(HELP));
    };
    let table = match p.pos.as_slice() {
        [t] => std::path::PathBuf::from(t),
        [] => return Err("table compare needs a table".into()),
        _ => return Err("table compare takes one table".into()),
    };
    let mut params: Vec<(String, Value)> = Vec::new();
    for k in ["ref", "blank"] {
        let v = p
            .get(&format!("--{k}"))
            .ok_or(format!("table compare needs --{k}"))?;
        params.push((k.into(), text(v)));
    }
    if let Some(q) = p.get("--quantity") {
        params.push(("quantity".into(), text(q)));
    }
    if let Some(v) = p.num("--factor")? {
        params.push(("factor".into(), num(v)));
    }
    match p.get("--pseudocount") {
        None | Some("auto") => {}
        Some(_) => params.push((
            "pseudocount".into(),
            num(p.num("--pseudocount")?.unwrap_or(f64::NAN)),
        )),
    }
    let csv = p.csv()?;
    if json && csv {
        return Err("--json and --format csv exclude each other".into());
    }
    Ok(Ok(Job {
        table,
        samples: p.get("--samples").map(Into::into),
        params,
        csv,
        dry: p.flag("--dry-run"),
        out: p.out(),
    }))
}

fn read(path: &std::path::Path) -> Result<Vec<u8>> {
    std::fs::read(path)
        .map_err(|e| Error::new("err.flow.io").with("detail", format!("{}: {e}", path.display())))
}

const COLUMNS: [&str; 17] = [
    "id",
    "mz",
    "rt_min",
    "ref_level",
    "blank_level",
    "max_treated",
    "max_file",
    "t_max_min",
    "ratio_ref",
    "ratio_blank",
    "log2fc_ref",
    "log2fc_blank",
    "absent_in_reference",
    "shape",
    "rho",
    "blank_rho",
    "class",
];

fn cell(v: Option<&Value>) -> String {
    match v {
        None | Some(Value::Null) => String::new(),
        Some(Value::Str(s)) if s.contains([',', '"', '\n']) => {
            format!("\"{}\"", s.replace('"', "\"\""))
        }
        Some(Value::Str(s)) => s.clone(),
        Some(other) => write(other),
    }
}

/// The CSV of a `compare/1` text: one row per feature, in the order of the table; `reasons` joined by `;`.
fn to_csv(doc: &str) -> Result<String> {
    let v = parse(doc.as_bytes())?;
    let mut out = COLUMNS.join(",") + ",reasons\n";
    for r in v.get("rows").map(Value::arr).unwrap_or(&[]) {
        let reasons: Vec<&str> = r
            .get("reasons")
            .map(Value::arr)
            .unwrap_or(&[])
            .iter()
            .filter_map(Value::str)
            .collect();
        let cells: Vec<String> = COLUMNS.iter().map(|k| cell(r.get(k))).collect();
        out += &format!("{},{}\n", cells.join(","), reasons.join(";"));
    }
    Ok(out)
}

pub fn run(args: Vec<String>) -> Outcome {
    let j = match job(args) {
        Ok(Ok(j)) => j,
        Ok(Err(h)) => return Outcome::ok(format!("{h}\n").into_bytes()),
        Err(e) => return Outcome::fail(&e),
    };
    let go = || -> Result<String> {
        let bytes = read(&j.table)?;
        let samples = j.samples.as_deref().map(read).transpose()?;
        let name = j
            .table
            .file_name()
            .map_or(String::new(), |n| n.to_string_lossy().into_owned());
        if j.dry {
            return mzlab_feat::flow_compare::plan_text(
                &j.params,
                &name,
                &bytes,
                samples.as_deref(),
            );
        }
        let doc =
            mzlab_feat::flow_compare::compare_text(&j.params, &name, &bytes, samples.as_deref())?;
        if j.csv {
            to_csv(&doc)
        } else {
            Ok(doc)
        }
    };
    match go() {
        Err(e) if e.key == "err.compare.duplicate" => Outcome::fail_error(&e).with_hint(
            "make the table again with `mzlab-cli features FILE --sample LIST` (one column per sample; the ids: `mzlab-cli sample list FILE`)",
        ),
        Err(e) => crate::sample_select::fail(&e),
        Ok(t) => crate::emit(t, &j.out),
    }
}
