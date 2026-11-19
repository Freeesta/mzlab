//! `mzlab-cli db ref-list FILE.csv [--name N] [--match X [--set unit=da|ppm tol=N pol=+|-]] [--out PATH]`: reads a CSV of reference ions (`reflist/1`,
//! `mzlab_core::reflist`, the same bytes as the page and the app) and, with `--match`, what a number is compatible with in that list (`reflist-match/1`).

use std::path::PathBuf;

use mzlab_core::json::{parse, Value};
use mzlab_core::jsonb::{int, text, write, Obj};

use crate::Outcome;

pub const HELP: &str = "mzlab-cli db ref-list: read a CSV list of reference ions as JSON (reflist/1), optionally match a number against it.

usage: mzlab-cli db ref-list FILE.csv [--name N] [--match X] [--set unit=da|ppm] [--set tol=N] [--set pol=+|-] [--out PATH]

  FILE.csv        columns mz, name, formula, polarity, adduct, note (a header line is required; ; , or tab; # lines skipped);
                  a row needs an m/z or a formula
  --name N        the list name carried in the answer
  --match X       also answer what the number X is compatible with in the list (reflist-match/1 under \"match\": the position of each hit in items, diff in Da, err in ppm)
  --out PATH      write there instead of the standard output

The answer is {\"schema\":\"reflist/1\",\"name\",\"compounds\",\"items\":[{line,name,formula,adduct,mz,pol,note}],\"errors\":[{line,key,field,value}]}.
Exit codes: 0 all rows read; 1 some rows have errors (listed with their line, the good rows are in items); 2 usage; 4 unreadable file.";

pub fn run(args: Vec<String>) -> Outcome {
    let (mut file, mut name, mut x, mut out) = (None, String::new(), None, None);
    let (mut unit, mut tol, mut pol) = (String::new(), None, String::new());
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        let mut val = |what: &str| it.next().ok_or(format!("{what} needs a value"));
        let r: Result<(), String> = match a.as_str() {
            "--help" | "-h" => return Outcome::ok(format!("{HELP}\n").into_bytes()),
            "--name" => val("--name").map(|v| name = v),
            "--match" => val("--match").map(|v| x = Some(v)),
            "--out" => val("--out").map(|v| out = Some(PathBuf::from(v))),
            "--set" => val("--set").and_then(|kv| match kv.split_once('=') {
                Some(("unit", v @ ("da" | "ppm"))) => {
                    unit = v.to_string();
                    Ok(())
                }
                Some(("tol", v)) => v
                    .parse::<f64>()
                    .ok()
                    .filter(|t| t.is_finite() && *t > 0.0)
                    .map(|t| tol = Some(t))
                    .ok_or("--set tol needs a number above 0".to_string()),
                Some(("pol", v @ ("+" | "-" | ""))) => {
                    pol = v.to_string();
                    Ok(())
                }
                _ => Err(format!("unknown --set {kv}")),
            }),
            f if f.starts_with("--") => Err(format!("unknown option {f}")),
            _ => {
                file = Some(PathBuf::from(a));
                Ok(())
            }
        };
        if let Err(m) = r {
            return Outcome::fail(&m);
        }
    }
    let Some(file) = file else {
        return Outcome::fail("db ref-list needs a CSV file");
    };
    let csv = match std::fs::read_to_string(&file) {
        Ok(t) => t,
        Err(e) => return fail4(&format!("{}: {e}", file.display())),
    };
    let body = write(
        &Obj::new()
            .put("csv", text(&csv))
            .put("name", text(&name))
            .done(),
    );
    let list = match mzlab_core::reflist::route(&body) {
        Ok(j) => j,
        Err(e) => return Outcome::fail_error(&e),
    };
    let mut answer = list.clone();
    if let Some(x) = x {
        match with_match(&list, &x, &unit, tol, &pol) {
            Ok(a) => answer = a,
            Err(o) => return o,
        }
    }
    let bad = parse(list.as_bytes())
        .is_ok_and(|v| !v.get("errors").map_or(&[][..], Value::arr).is_empty());
    let mut o = crate::emit(format!("{answer}\n"), &out);
    if bad && o.code == 0 {
        o.code = 1;
        o.stderr = "some rows have errors (see \"errors\")\n".into();
    }
    o
}

/// The list answer with the `reflist-match/1` of the number `x` against its items added under "match".
fn with_match(
    list: &str,
    x: &str,
    unit: &str,
    tol: Option<f64>,
    pol: &str,
) -> Result<String, Outcome> {
    let Some(x) = x
        .replace(',', ".")
        .parse::<f64>()
        .ok()
        .filter(|n| n.is_finite())
    else {
        return Err(Outcome::fail("the number must be finite"));
    };
    let v = parse(list.as_bytes()).unwrap_or(Value::Null);
    // the columns the page sends for its table: one group per distinct (name, adduct, m/z), so a repeated row is one hit
    let items = v.get("items").map_or(&[][..], Value::arr);
    let col = |k: &str| {
        Value::Arr(
            items
                .iter()
                .map(|i| i.get(k).cloned().unwrap_or(Value::Null))
                .collect(),
        )
    };
    let mut seen = std::collections::HashMap::new();
    let group: Vec<Value> = items
        .iter()
        .map(|i| {
            let key = format!(
                "{}|{}|{}",
                i.get("name").and_then(Value::str).unwrap_or(""),
                i.get("adduct").and_then(Value::str).unwrap_or(""),
                i.get("mz").and_then(Value::f64).unwrap_or(0.0)
            );
            let n = seen.len();
            int(*seen.entry(key).or_insert(n) as i64)
        })
        .collect();
    let mut req = Obj::new().n("mz", x).put("pol", text(pol));
    if let (false, Some(t)) = (unit.is_empty(), tol) {
        req = req.put("unit", text(unit)).n("tol", t);
    }
    let req = req
        .put(
            "items",
            Obj::new()
                .put("mz", col("mz"))
                .put("pol", col("pol"))
                .put("group", Value::Arr(group))
                .done(),
        )
        .done();
    let m =
        mzlab_core::reflist::lookup::route(&write(&req)).map_err(|e| Outcome::fail_error(&e))?;
    Ok(format!(
        "{}{}",
        list.trim_end_matches('}'),
        format_args!(",\"match\":{m}}}")
    ))
}

fn fail4(m: &str) -> Outcome {
    let mut o = Outcome::fail(m);
    o.code = 4;
    o
}

#[cfg(test)]
#[path = "ref_list_tests.rs"]
mod tests;
