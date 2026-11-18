//! `mzlab-cli ref-list FILE.csv [--name N] [--match X [--set unit=da|ppm tol=N pol=+|-]] [--out PATH]`: reads a CSV of reference ions (`reflist/1`,
//! `mzlab_core::reflist`, the same bytes as the page and the app) and, with `--match`, what a number is compatible with in that list (`ricerca/1`).

use std::path::PathBuf;

use mzlab_core::json::{parse, Value};
use mzlab_core::jsonb::{text, write, Obj};

use crate::Outcome;

pub const HELP: &str = "mzlab-cli ref-list: read a CSV list of reference ions as JSON (reflist/1), optionally match a number against it.

usage: mzlab-cli ref-list FILE.csv [--name N] [--match X] [--set unit=da|ppm] [--set tol=N] [--set pol=+|-] [--out PATH]

  FILE.csv        columns mz, name, formula, polarity, adduct, note (a header line is required; ; , or tab; # lines skipped);
                  a row needs an m/z or a formula
  --name N        the list name carried in the answer
  --match X       also answer what the number X is compatible with in the list (ricerca/1 under \"match\")
  --out PATH      write there instead of the standard output

The answer is {\"format\":\"reflist/1\",\"name\",\"items\":[{line,name,formula,adduct,mz,pol,note}],\"errors\":[{line,key,field,value}]}.
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
                Some(("unit", v @ ("da" | "ppm"))) => Ok(unit = v.to_string()),
                Some(("tol", v)) => v
                    .parse::<f64>()
                    .ok()
                    .filter(|t| t.is_finite() && *t > 0.0)
                    .map(|t| tol = Some(t))
                    .ok_or("--set tol needs a number above 0".to_string()),
                Some(("pol", v @ ("+" | "-" | ""))) => Ok(pol = v.to_string()),
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
        return Outcome::fail("ref-list needs a CSV file");
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
        let Some(x) = x
            .replace(',', ".")
            .parse::<f64>()
            .ok()
            .filter(|n| n.is_finite())
        else {
            return Outcome::fail("the number must be finite");
        };
        let v = parse(list.as_bytes()).unwrap_or(Value::Null);
        let contam: Vec<Value> = v
            .get("items")
            .map_or(&[][..], Value::arr)
            .iter()
            .map(|i| {
                Obj::new()
                    .put("id", text(i.get("name").and_then(Value::str).unwrap_or("")))
                    .put("mz", i.get("mz").cloned().unwrap_or(Value::Null))
                    .put("pol", i.get("pol").cloned().unwrap_or(Value::Null))
                    .done()
            })
            .collect();
        let mut req = Obj::new().n("x", x).b("hr", true).put("pol", text(&pol));
        if let (false, Some(t)) = (unit.is_empty(), tol) {
            req = req.put("tol_unit", text(&unit)).n("tol", t);
        }
        let req = req
            .put("losses", Value::Arr(vec![]))
            .put("adducts", Value::Arr(vec![]))
            .put("contam", Value::Arr(contam))
            .done();
        match mzlab_core::search::match_number(&write(&req)) {
            Ok(m) => {
                answer = format!(
                    "{}{}",
                    answer.trim_end_matches('}'),
                    format_args!(",\"match\":{m}}}")
                )
            }
            Err(e) => return Outcome::fail_error(&e),
        }
    }
    let bad = list.contains("\"errors\":[{");
    let mut o = crate::emit(format!("{answer}\n"), &out);
    if bad && o.code == 0 {
        o.code = 1;
        o.stderr = "some rows have errors (see \"errors\")\n".into();
    }
    o
}

fn fail4(m: &str) -> Outcome {
    let mut o = Outcome::fail(m);
    o.code = 4;
    o
}

#[cfg(test)]
#[path = "ref_list_tests.rs"]
mod tests;
