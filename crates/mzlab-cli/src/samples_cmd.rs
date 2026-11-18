//! `mzlab-cli samples vars|group|table`: the name operations of `samples::route` (`variabili`, `nome_gruppo`, `proponi`), printing the JSON the page gets
//! from `api/campioni/<op>`. The names come one per line from a file or the standard input. `samples list FILE...` is what the files hold, sample by
//! sample, from their index alone (`samples_list/1`: the same JSON as the site's `native_samples` and the app's native listing).

use std::io::Read;
use std::path::{Path, PathBuf};

use mzlab_core::error::Error;
use mzlab_core::json::{parse, Out, Value};
use mzlab_core::readers::Limits;

use crate::Outcome;

pub const HELP: &str = "mzlab-cli samples: how the names of a set of files split them (nomivar/1), as JSON.

usage: mzlab-cli samples vars [--names FILE|-] [--pos N [--sep CHARS]] [--has a,b [--substring]]
       mzlab-cli samples group NAME...
       mzlab-cli samples table [--names FILE|-]
       mzlab-cli samples list FILE... [--json] [--probe]   (samples list --help)
       mzlab-cli samples select FILE (--sample LIST | --default [--budget-mb N]) [--json]   (samples select --help)

  --names FILE|-  one file name per line (default: the standard input)
  --pos N         also the manual variable of the N-th token (1 = first, -1 = last), cut at --sep (default \"_-. \")
  --has a,b       also the manual variable of terms: a file goes to the first term equal to one of its tokens
  --substring     with --has: the term may be inside a token (O2 then catches H2O2)
  group           the name of a group made of NAMEs: their common tokens in order, empty when none
  table           the sample table the names propose (campioni/2): role, time point (`tempo`, `unita_tempo`, `tempo_min`), series, replicate, concentration";

fn names(src: &str) -> Result<Vec<String>, String> {
    let mut text = String::new();
    if src == "-" {
        std::io::stdin()
            .read_to_string(&mut text)
            .map_err(|e| e.to_string())?;
    } else {
        text = std::fs::read_to_string(src).map_err(|e| format!("{src}: {e}"))?;
    }
    Ok(text
        .lines()
        .map(|l| l.trim_end_matches('\r'))
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect())
}

fn list(names: &[String]) -> String {
    let mut o = Out::default();
    o.raw("[");
    for (i, n) in names.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(n);
    }
    o.raw("]");
    o.0
}

/// The body of `vars`: the names and, when asked, the manual variable.
fn variabili(args: Vec<String>) -> Result<String, String> {
    let (mut src, mut pos, mut sep, mut has, mut substring) =
        ("-".to_string(), None, None, None, false);
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--names" => src = it.next().ok_or("--names needs a file")?,
            "--pos" => {
                let p = it.next().ok_or("--pos needs a number")?;
                pos = Some(p.parse::<i64>().map_err(|_| "--pos needs a number")?);
            }
            "--sep" => sep = Some(it.next().ok_or("--sep needs characters")?),
            "--has" => has = Some(it.next().ok_or("--has needs terms")?),
            "--substring" => substring = true,
            o => return Err(format!("unknown option {o}")),
        }
    }
    if pos.is_some() && has.is_some() {
        return Err("--pos and --has are two manual variables: one at a time".into());
    }
    if (sep.is_some() && pos.is_none()) || (substring && has.is_none()) {
        return Err("--sep needs --pos, --substring needs --has".into());
    }
    let files: Vec<String> = names(&src)?
        .iter()
        .map(|n| {
            let mut o = Out::default();
            o.raw("{\"name\":").str(n).raw("}");
            o.0
        })
        .collect();
    let mut body = format!("{{\"files\":[{}]", files.join(","));
    if let Some(p) = pos {
        let mut o = Out::default();
        o.str(sep.as_deref().unwrap_or("_-. "));
        body += &format!(",\"manuale\":{{\"pos\":{p},\"sep\":{}}}", o.0);
    }
    if let Some(h) = has {
        let terms: Vec<String> = h.split(',').map(|t| t.trim().to_string()).collect();
        body += &format!(
            ",\"manuale\":{{\"has\":{},\"substring\":{substring}}}",
            list(&terms)
        );
    }
    body.push('}');
    Ok(body)
}

/// The body of `table`: the names, in the order given, as the files of `proponi` (no SHA-256, no file kind).
fn table(args: Vec<String>) -> Result<String, String> {
    let mut src = "-".to_string();
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--names" => src = it.next().ok_or("--names needs a file")?,
            o => return Err(format!("unknown option {o}")),
        }
    }
    let files: Vec<String> = names(&src)?
        .iter()
        .map(|n| {
            let mut o = Out::default();
            o.raw("{\"name\":")
                .str(n)
                .raw(",\"sha256\":\"\",\"kind\":\"\"}");
            o.0
        })
        .collect();
    Ok(format!("{{\"files\":[{}]}}", files.join(",")))
}

const LIST_HELP: &str = "mzlab-cli samples list: what each file holds, sample by sample, from its container and index alone (samples_list/1).

usage: mzlab-cli samples list FILE... [--json] [--probe]

  FILE     a .wiff (its .wiff.scan beside it is not read), another vendor file or a .D / .raw folder
  --json   one samples_list/1 JSON per line on stdout, in the order of the FILEs (a file that cannot be listed gives an errore/1 line); without it, a table
  --probe  also compare the size of the .wiff.scan with where its indexed part ends and add `probe` to the JSON: {scan_bytes, indexed_end, status}
           (ok | short: the copy is cut, exit 1 | longer: bytes after the last indexed section | n/a: no data file)
Per sample: id, name, scans (as the index counts them), size (bytes of its section), kinds (ms, ms2, sim, mrm, dad), polarity, rt [min, max] and
tic {apex_rt, apex, sum} (minutes), method (the .dam file), missing (index rows past the end of the data file), why (refused | inferred | empty),
refused. Per file: format, bytes, scan {bytes, indexed_end}. No hashes, no log text, no operator. Nothing is written.
Streams: data on stdout; progress (one line per file) and warnings on stderr.
Exit codes: 0 listed; 1 a file or sample refused, or a cut .wiff.scan with --probe (the listing is still printed); 2 usage; 4 a file missing or unreadable.";

struct Listed {
    json: String,
    refused: bool,
    cut: bool,
}

/// `{"scan_bytes", "indexed_end", "status"}` of the data file of a pair, `n/a` for a format without one.
fn probe_json(scan: Option<(u64, u64)>) -> (String, &'static str) {
    let Some((bytes, end)) = scan else {
        return ("{\"status\":\"n/a\"}".into(), "n/a");
    };
    let status = match bytes.cmp(&end) {
        std::cmp::Ordering::Less => "short",
        std::cmp::Ordering::Equal => "ok",
        std::cmp::Ordering::Greater => "longer",
    };
    (
        format!("{{\"scan_bytes\":{bytes},\"indexed_end\":{end},\"status\":\"{status}\"}}"),
        status,
    )
}

/// The `samples_list/1` of one file (a `.wiff` from its container and index alone), `Err((exit code, error))` when it cannot be listed.
fn list_one(path: &Path, probe: bool) -> Result<Listed, (u8, Error)> {
    if !path.exists() {
        let name = path.display().to_string();
        return Err((4, Error::new("err.session.fileNotFound").with("name", name)));
    }
    let fs = crate::flow_set::file_set(path).map_err(|e| (4, e))?;
    let mut l = mzlab_lettori::apri_listing(fs, &Limits::desktop()).map_err(|e| (1, e))?;
    let refused = (0..l.samples().len()).any(|i| l.refused(i).is_some());
    let mut json = mzlab_core::readers::list_samples(&mut *l);
    let mut cut = false;
    if probe {
        let (p, status) = probe_json(l.listing().scan);
        cut = status == "short";
        json.pop();
        json += &format!(",\"probe\":{p}}}");
    }
    Ok(Listed { json, refused, cut })
}

fn mb(bytes: f64) -> String {
    format!("{:.1} MB", bytes / 1e6)
}

/// The table of a `samples_list/1`: one header line per file and one line per sample (`-` where the reader does not say).
fn list_table(json: &str, file: &str) -> String {
    let Ok(v) = parse(json.as_bytes()) else {
        return String::new();
    };
    let num = |x: Option<&Value>| x.and_then(Value::f64);
    let n = v.get("samples").map_or(0, |s| s.arr().len());
    let mut t = format!(
        "{file}  {}  {n} samples  {}\n",
        v.get("format").and_then(Value::str).unwrap_or("?"),
        mb(num(v.get("bytes")).unwrap_or(0.0))
    );
    for s in v.get("samples").map_or(&[][..], Value::arr) {
        let kinds: Vec<&str> = s
            .get("kinds")
            .map_or(&[][..], Value::arr)
            .iter()
            .filter_map(Value::str)
            .collect();
        let rt = s.get("rt").map(Value::arr).filter(|r| r.len() == 2);
        let rt = rt.map_or("-".to_string(), |r| {
            format!(
                "{:.2}-{:.2}",
                r[0].f64().unwrap_or(0.0),
                r[1].f64().unwrap_or(0.0)
            )
        });
        let apex = num(s.get("tic").and_then(|c| c.get("apex_rt")));
        let note = ["why", "missing"]
            .iter()
            .filter_map(|k| {
                let x = s.get(k)?;
                Some(x.str().map_or_else(
                    || format!("{k} {}", x.f64().unwrap_or(0.0)),
                    |w| w.to_string(),
                ))
            })
            .chain(s.get("refused").map(|_| "see --json".to_string()))
            .collect::<Vec<_>>()
            .join(", ");
        t += &format!(
            "  {:>4}  {}  {} scans  {}  {}  rt {rt} min  tic apex {}  {}  {note}\n",
            num(s.get("id")).unwrap_or(0.0),
            s.get("name").and_then(Value::str).unwrap_or(""),
            num(s.get("scans")).unwrap_or(0.0),
            if kinds.is_empty() {
                "-".to_string()
            } else {
                kinds.join("+")
            },
            s.get("polarity").and_then(Value::str).unwrap_or("-"),
            apex.map_or("-".to_string(), |a| format!("{a:.2}")),
            num(s.get("size")).map_or("-".to_string(), mb),
        );
    }
    t
}

const SELECT_HELP: &str = "mzlab-cli samples select: which samples of a file `--sample LIST` would open, checked against its container and index (nothing is read beyond them, nothing is written).

usage: mzlab-cli samples select FILE (--sample LIST | --default [--budget-mb N]) [--json]

  FILE         a .wiff (its .wiff.scan is not read), another vendor file or a .D / .raw folder
  --sample LIST  sample ids and ranges: 1,3,5 or 2-4 (the ids of `samples list`, not positions)
  --default    the samples the site's chooser ticks at the start (`sample_default`): all when their section bytes are small against the budget, none otherwise
  --budget-mb N  the memory budget for --default in MB (default 1024; 0 = the default)
  --json       {\"samples\":[ids]} on stdout, the ids in the order of the file: the same bytes as the site's `sample_selection` / `sample_default`
The same LIST is taken by `convert`, `channels` and `features`.
Exit codes: 0 selected; 2 usage (LIST is not a list); 4 the file is missing or unreadable; 5 an id the file does not have (the ids present are named).";

/// `samples select FILE --sample LIST [--json]`: the dry run of `--sample`.
fn select_run(args: Vec<String>) -> Outcome {
    let (mut file, mut text, mut json) = (None::<PathBuf>, None::<String>, false);
    let (mut default, mut budget) = (false, 0.0f64);
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--help" | "-h" => return Outcome::ok(format!("{SELECT_HELP}\n").into_bytes()),
            "--json" => json = true,
            "--default" => default = true,
            "--budget-mb" => match it.next().and_then(|v| v.parse::<f64>().ok()) {
                Some(v) if v >= 0.0 => budget = v,
                _ => return Outcome::fail("--budget-mb needs a number of MB"),
            },
            "--sample" => match it.next() {
                Some(v) => text = Some(v),
                None => return Outcome::fail("--sample needs a list such as 1,3,5"),
            },
            o if o.starts_with('-') && o.len() > 1 => {
                return Outcome::fail(&format!("unknown option {o} (samples select --help)"))
            }
            _ if file.is_none() => file = Some(PathBuf::from(a)),
            _ => return Outcome::fail("samples select takes one FILE"),
        }
    }
    let (Some(file), text) = (file, text) else {
        return Outcome::fail("samples select needs a FILE and --sample LIST or --default");
    };
    if text.is_some() == default {
        return Outcome::fail("samples select needs exactly one of --sample LIST and --default");
    }
    if !file.exists() {
        let e = Error::new("err.session.fileNotFound").with("name", file.display());
        let mut o = Outcome::fail_error(&e);
        o.code = 4;
        if json {
            let msg = crate::error_text::human_error(&e);
            o.stdout =
                crate::flow_json::error_json(4, &e, &msg, "check the path", &[]).into_bytes();
        }
        return o;
    }
    let chosen = match &text {
        Some(t) => {
            crate::sample_select::parse(t).and_then(|w| crate::sample_select::resolve(&file, &w))
        }
        None => crate::sample_select::sized(&file).map(|(ids, sizes)| {
            mzlab_core::readers::selection::suggest(
                &ids,
                &sizes,
                mzlab_core::readers::selection::budget_bytes(budget),
            )
        }),
    };
    match chosen {
        Err(e) => crate::sample_select::fail(&e, json),
        Ok(ids) => {
            let ids: Vec<String> = ids.iter().map(u32::to_string).collect();
            let out = if json {
                format!("{{\"samples\":[{}]}}\n", ids.join(","))
            } else {
                format!("{}\n", ids.join(" "))
            };
            Outcome::ok(out.into_bytes())
        }
    }
}

/// `samples list FILE... [--json] [--probe]`.
fn list_run(args: Vec<String>) -> Outcome {
    let (mut files, mut json, mut probe) = (Vec::<PathBuf>::new(), false, false);
    for a in args {
        match a.as_str() {
            "--help" | "-h" => return Outcome::ok(format!("{LIST_HELP}\n").into_bytes()),
            "--json" => json = true,
            "--probe" => probe = true,
            o if o.starts_with('-') && o.len() > 1 => {
                return Outcome::fail(&format!("unknown option {o} (samples list --help)"))
            }
            _ => files.push(PathBuf::from(a)),
        }
    }
    if files.is_empty() {
        return Outcome::fail("samples list needs at least one FILE");
    }
    let (mut out, mut err, mut code) = (String::new(), String::new(), 0u8);
    for f in &files {
        err += &format!("listing {}\n", f.display());
        match list_one(f, probe) {
            Ok(l) => {
                if json {
                    out += &(l.json.clone() + "\n");
                } else {
                    out += &list_table(&l.json, &f.display().to_string());
                }
                if l.refused {
                    err += &format!(
                        "warning: {}: a sample is refused (see its `refused`)\n",
                        f.display()
                    );
                }
                if l.cut {
                    err += &format!("warning: {}: the .wiff.scan is shorter than the index says: a copy cut short\n", f.display());
                }
                if l.json.contains("\"status\":\"longer\"") {
                    err += &format!("warning: {}: the .wiff.scan holds bytes after the last indexed section (a copy made while acquiring, or padding)\n", f.display());
                }
                code = code.max(u8::from(l.refused || l.cut));
            }
            Err((c, e)) => {
                let msg = crate::error_text::human_error(&e);
                err += &format!("error: {msg}\n");
                if json {
                    out += &crate::flow_json::error_json(
                        c,
                        &e,
                        &msg,
                        "check the path and that the whole pair was copied",
                        &[],
                    );
                }
                code = if c == 4 || code == 4 { 4 } else { code.max(c) };
            }
        }
    }
    Outcome {
        code,
        stdout: out.into_bytes(),
        stderr: err,
    }
}

pub fn run(args: Vec<String>) -> Outcome {
    let mut it = args.into_iter();
    let first = it.next();
    let sub = first.as_deref();
    let (op, body) = match sub {
        Some("vars") => ("variabili", variabili(it.collect())),
        Some("table") => ("proponi", table(it.collect())),
        Some("group") => {
            let n: Vec<String> = it.collect();
            ("nome_gruppo", Ok(format!("{{\"names\":{}}}", list(&n))))
        }
        Some("list") => return list_run(it.collect()),
        Some("select") => return select_run(it.collect()),
        Some("--help" | "-h") | None => return Outcome::ok(format!("{HELP}\n").into_bytes()),
        Some(c) => return Outcome::fail(&format!("unknown samples operation {c}")),
    };
    match body.and_then(|b| {
        mzlab_core::samples::route_all(op, &b).map_err(|e| format!("{} {:?}", e.key, e.params))
    }) {
        Ok(t) => Outcome::ok((t + "\n").into_bytes()),
        Err(e) => Outcome::fail(&e),
    }
}

#[cfg(test)]
#[path = "samples_cmd_tests.rs"]
mod tests;
