//! `mzlab-cli quant union`: the feature tables of several series (the CSV of `mzlab-cli features`, one per series) joined into ONE target list for
//! `quant targets` (`mzlab_core::series_union::route`, the same bytes as the app route `lr_union` and the site's `.wasm`). The target list is the
//! standard output (or `union/1` with `--json`); merges, conflicts, drops and every automatic choice are in the document and, one line each, on stderr.

use std::path::PathBuf;

use mzlab_core::error::Error;
use mzlab_core::json::{parse, Out, Value};

use crate::samples_import::say;
use crate::Outcome;

pub const HELP: &str = "mzlab-cli quant union: the feature tables of several series joined into one target list for `quant targets`.

usage: mzlab-cli quant union TABLE.csv... [--da D] [--rt-tol MIN] [--rt-warp off|piecewise] [--rt-gap MIN] [--min-series N] [--pad MIN]
                             [--polarity +|-] [--set NAME=VALUE...] [--json] [--out PATH]

  TABLE.csv        one feature table per series (the CSV of `mzlab-cli features`): columns id, mz, rt and, if there, rt_range:min and rt_range:max
                   (`rt_min`, `rt_max`), polarity; the sample columns are not read. The first table is the reference of the RT frame; run it once per polarity
  --da D           m/z agreement of two rows of one target, Da, 0.001 to 5 (default 0.5)
  --rt-tol MIN     RT agreement after the map, min, 0.001 to 5 (default 0.1)
  --rt-warp M      off (default) or piecewise: a map of every other series onto the first from the rows both hold (anchors: mutual nearest rows
                   within --da and --rt-gap); fewer than 3 anchors: no map, logged
  --rt-gap MIN     widest RT gap of an anchor pair, min (default 0.5)
  --min-series N   a target seen in fewer series is listed as dropped and not written (default 1: every row is kept)
  --pad MIN        minutes added on both sides of every window (default 0)
  --polarity P     the polarity of rows that have none of their own: + or - (default: every scan)
  --set NAME=VALUE any of the parameters above by its name (da, rt_tol, rt_warp, rt_gap, min_series, pad, polarity)
  --json           union/1 on stdout: params, counts, series (with the RT map), targets with the rows each came from, conflicts, dropped, choices, csv
  --out PATH       write the output there instead of the standard output

Output: CSV id,mz,rt_min,rt_max,polarity,role (ids U0001..., sorted by polarity, m/z, RT), the table `quant targets --targets` reads. Rows of one series are never merged; two
targets that stay within --da and --rt-tol are a conflict: both kept and listed. The window of a target covers the window of each of its rows in its own series (a row without
one: RT +- --rt-tol), then --pad. Stderr: one line per conflict, drop and automatic choice. Nothing is refused for a finding.
Exit codes: 0 ok; 2 usage (bad option or parameter value); 3 a table that cannot be used (the row and the field are named); 4 a file missing or unreadable; 6 --out cannot be written.";

/// The flags that are parameters: flag, parameter name.
const PARAMS: [(&str, &str); 7] = [
    ("--da", "da"),
    ("--rt-tol", "rt_tol"),
    ("--rt-warp", "rt_warp"),
    ("--rt-gap", "rt_gap"),
    ("--min-series", "min_series"),
    ("--pad", "pad"),
    ("--polarity", "polarity"),
];

fn usage(msg: &str) -> Outcome {
    Outcome::fail_error(&Error::new("err.cli.usage").with("message", msg))
}

fn read(path: &str) -> Result<String, Outcome> {
    std::fs::read_to_string(path).map_err(|_| {
        Outcome::fail_error(&Error::new("err.session.fileNotFound").with("name", path))
    })
}

fn request(paths: &[String], params: &[(String, String)]) -> Result<String, Outcome> {
    let mut o = Out::default();
    o.raw("{\"tables\":[");
    for (i, p) in paths.iter().enumerate() {
        let name = std::path::Path::new(p)
            .file_name()
            .map_or(p.clone(), |n| n.to_string_lossy().into_owned());
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"name\":")
            .str(&name)
            .raw(",\"text\":")
            .str(&read(p)?)
            .raw("}");
    }
    o.raw("],\"params\":{");
    for (i, (k, v)) in params.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(k).raw(":").str(v);
    }
    o.raw("}}");
    Ok(o.0)
}

fn text<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k).and_then(Value::str).unwrap_or("")
}

fn number(v: &Value, k: &str) -> String {
    v.get(k)
        .and_then(Value::f64)
        .map_or(String::new(), |n| n.to_string())
}

/// The stderr lines of a document: the summary, the conflicts, the drops, the automatic choices (texts: `err.cli.union.*` of `lang/en`).
fn lines(doc: &Value) -> String {
    let each = |k: &str| doc.get(k).map_or(&[][..], Value::arr);
    let counts = doc.get("counts").cloned().unwrap_or(Value::Null);
    let mut out = say(
        "err.cli.union.summary",
        &[
            ("rows", number(&counts, "rows_in")),
            ("series", each("series").len().to_string()),
            ("targets", number(&counts, "targets")),
            ("merged", number(&counts, "merged")),
        ],
    ) + "\n";
    for c in each("conflicts") {
        let vals = [
            ("a", text(c, "a").into()),
            ("b", text(c, "b").into()),
            ("dmz", number(c, "delta_mz")),
            ("drt", number(c, "delta_rt")),
        ];
        out += &(say("err.cli.union.conflict", &vals) + "\n");
    }
    for d in each("dropped") {
        let vals = [
            ("mz", number(d, "mz")),
            ("rt", number(d, "rt")),
            ("series", number(d, "series")),
        ];
        out += &(say("err.cli.union.dropped", &vals) + "\n");
    }
    for c in each("choices") {
        let rest: Vec<String> = c
            .entries()
            .iter()
            .filter(|(k, _)| k != "what" && k != "series")
            .map(|(k, v)| format!("{k}={}", v.str().map_or_else(|| number(c, k), String::from)))
            .collect();
        let vals = [
            ("what", text(c, "what").into()),
            ("series", text(c, "series").into()),
            ("detail", rest.join(" ")),
        ];
        out += &(say("err.cli.union.auto", &vals) + "\n");
    }
    out
}

pub(crate) fn run(args: Vec<String>) -> Outcome {
    let json = args.iter().any(|a| a == "--json");
    let mut valued: Vec<&str> = PARAMS.iter().map(|p| p.0).collect();
    valued.extend(["--set", "--out"]);
    let p = match crate::extract::parse(args, &valued, &[]) {
        Ok(Some(p)) => p,
        Ok(None) => return Outcome::ok(format!("{HELP}\n").into_bytes()),
        Err(e) => return Outcome::fail(&e),
    };
    if p.pos.is_empty() {
        return usage("quant union needs at least one table: mzlab-cli quant union --help");
    }
    let mut params: Vec<(String, String)> = PARAMS
        .iter()
        .filter_map(|(f, k)| p.get(f).map(|v| (k.to_string(), v.to_string())))
        .collect();
    for s in p.all("--set") {
        let Some((k, v)) = s.split_once('=') else {
            return usage(&format!("--set takes NAME=VALUE, not {s}"));
        };
        params.push((k.trim().to_string(), v.to_string()));
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
    let doc = match request(&p.pos, &params)
        .and_then(|b| mzlab_core::series_union::route(&b).map_err(|e| Outcome::fail_error(&e)))
    {
        Ok(d) => d,
        Err(o) => return o,
    };
    let Ok(v) = parse(doc.as_bytes()) else {
        return usage("the union answered with a document that is not JSON");
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
