//! `mzlab-cli sample vars|group|table`: the name operations of `samples::route` (`variables`, `group_name`, `propose`), printing the JSON the page gets
//! from `api/samples/<op>`. The names come one per line from a file or the standard input. `sample list FILE...` is what the files hold, sample by
//! sample, from their index alone (`samples_list/1`: the same JSON as the site's `native_samples` and the app's native listing).

use std::io::Read;
use std::path::{Path, PathBuf};

use mzlab_core::error::Error;
use mzlab_core::json::{parse, Out, Value};
use mzlab_core::readers::Limits;

use crate::Outcome;

pub const HELP: &str = "mzlab-cli sample: how the names of a set of files split them (sample-vars/1), as JSON.

usage: mzlab-cli sample vars [--names FILE|-] [--pos N [--sep CHARS]] [--has a,b [--substring]]
       mzlab-cli sample group NAME...
       mzlab-cli sample table [--names FILE|- | FILE...] [--text]
       mzlab-cli sample import META.csv (FILE|DIR)... [--dry-run] [--json]   (sample import --help)
       mzlab-cli sample list FILE... [--json] [--probe]   (sample list --help)
       mzlab-cli sample select FILE (--sample LIST | --default [--budget-mb N]) [--json]   (sample select --help)
       mzlab-cli sample design propose|validate|canonical ...   the experimental design (design/1)   (sample design --help)

  --names FILE|-  one file name per line (default: the standard input)
  --pos N         also the manual variable of the N-th token (1 = first, -1 = last), cut at --sep (default \"_-. \")
  --has a,b       also the manual variable of terms: a file goes to the first term equal to one of its tokens
  --substring     with --has: the term may be inside a token (O2 then catches H2O2)
  group           the name of a group made of NAMEs: their common tokens in order, empty when none
  FILE...         with table: real files, named by their file name and hashed (SHA-256), so `flow record` can match the roles
  table           the sample table the names propose (samples/1): role (`kind`, and `blank_kind` for a wash), time point (`time`, `time_unit`, `time_min`), series, replicate, concentration
  --text          with table: a plain table (file, role, time_min, series, replicate) in place of the JSON
  Every role and time the names decide is logged on stderr (`auto: NAME kind=dark \"BLKD\" ...`: the field, its value and the part of the name that decided it);
  the user overrides one by editing `kind` (or `time`) of the samples/1 before the next step. A dark control (`COND-D`, `H2O2D`) and a dark matrix blank
  (`BLKD`) are role `dark`, never `t0`; `Wash` is a `blank` of `blank_kind` `wash`; `std`, `blk`/`blank`, `QC` are read by name.
example: mzlab-cli sample table s/*.mzML > samples.json, then the `samples` input of tp.dig_lr or tp.link (a file the samples lack takes the role of its name, logged).";

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
fn variables_args(args: Vec<String>) -> Result<String, String> {
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
        body += &format!(",\"manual\":{{\"pos\":{p},\"sep\":{}}}", o.0);
    }
    if let Some(h) = has {
        let terms: Vec<String> = h.split(',').map(|t| t.trim().to_string()).collect();
        body += &format!(
            ",\"manual\":{{\"has\":{},\"substring\":{substring}}}",
            list(&terms)
        );
    }
    body.push('}');
    Ok(body)
}

/// The body of `table`: the names, in the order given, as the files of `propose` (no file kind). The SHA-256 is empty for a name and the real one for a
/// FILE path (hashed here, in the order given), which is what `flow record` matches a role by.
pub(crate) fn table(args: Vec<String>) -> Result<String, String> {
    let mut src = None;
    let mut paths = Vec::new();
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--names" => src = Some(it.next().ok_or("--names needs a file")?),
            o if o.starts_with("--") => return Err(format!("unknown option {o}")),
            _ => paths.push(a),
        }
    }
    if src.is_some() && !paths.is_empty() {
        return Err("--names and FILE paths do not go together".into());
    }
    let mut named: Vec<(String, String)> = Vec::new();
    for p in &paths {
        let (_, sha) =
            crate::flow_hash::file_sha256(Path::new(p)).map_err(|e| format!("{p}: {e}"))?;
        let name = Path::new(p)
            .file_name()
            .map_or(p.clone(), |n| n.to_string_lossy().into_owned());
        named.push((name, sha));
    }
    if paths.is_empty() {
        named = names(src.as_deref().unwrap_or("-"))?
            .into_iter()
            .map(|n| (n, String::new()))
            .collect();
    }
    let files: Vec<String> = named
        .iter()
        .map(|(n, sha)| {
            let mut o = Out::default();
            o.raw("{\"name\":")
                .str(n)
                .raw(",\"sha256\":")
                .str(sha)
                .raw(",\"kind\":\"\"}");
            o.0
        })
        .collect();
    Ok(format!("{{\"files\":[{}]}}", files.join(",")))
}

const LIST_HELP: &str = "mzlab-cli sample list: what each file holds, sample by sample, from its container and index alone (samples_list/1).

usage: mzlab-cli sample list FILE... [--json] [--probe]

  FILE     a .wiff (its .wiff.scan beside it is not read), another vendor file or a .D / .raw folder
  --json   one sample-list/1 document on stdout, {\"schema\",\"files\":[samples_list/1 per FILE, in order]}; a file the reader refuses has its error/1 in its place (exit 1); without it, a table
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
    crate::sample_select::refuse_mzml(path).map_err(|e| (1, e))?;
    let fs = crate::flow_set::file_set(path).map_err(|e| {
        let code = if e.key == "err.file.wiffGrowing" {
            crate::exit::GROWING
        } else {
            4
        };
        (code, e)
    })?;
    let mut l = mzlab_lettori::open_listing(fs, &Limits::desktop()).map_err(|e| (1, e))?;
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

const SELECT_HELP: &str = "mzlab-cli sample select: which samples of a file `--sample LIST` would open, or which ones the chooser of the site ticks first (`--default`), checked against its container and index (nothing is read beyond them, nothing is written).

usage: mzlab-cli sample select FILE (--sample LIST [--ranges] | --default [--budget-mb N]) [--json]

  FILE         a .wiff (its .wiff.scan is not read), another vendor file or a .D / .raw folder
  --sample LIST  sample ids and ranges: 1,3,5 or 2-4 (the ids of `sample list`, not positions)
  --ranges     with --sample: also the byte ranges of every member that those samples need (`bytes`, `total`, `tag`, `members[].ranges`, `[start, end)`): what the site copies into OPFS, the head of the .wiff.scan, the chosen sections and what follows the indexed part; any other member whole (the same bytes as the site's `sample_ranges`)
  --default    the samples the site's chooser ticks at the start (`sample_default`): all when their section bytes are small against the budget, none otherwise
  --budget-mb N  the memory budget for --default in MB (default 1024; 0 = the default)
  --json       {\"schema\":\"sample-select/1\",\"samples\":[ids]} on stdout, the ids in the order of the file: the same bytes as the site's `sample_selection` / `sample_default`
The same LIST is taken by `convert`, `channels` and `features`.
Exit codes: 0 selected; 2 usage (LIST is not a list, both or neither of --sample and --default, --ranges without --sample, a --budget-mb that is not a number of MB); 4 the file is missing or unreadable; 5 an id the file does not have (the ids present are named).";

/// `sample select FILE (--sample LIST | --default [--budget-mb N]) [--json]`: the dry run of `--sample`, or the first ticks of the page chooser.
fn select_run(args: Vec<String>) -> Outcome {
    let (mut file, mut text, mut json) = (None::<PathBuf>, None::<String>, false);
    let (mut default, mut budget, mut ranges) = (false, 0.0f64, false);
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--help" | "-h" => return Outcome::ok(format!("{SELECT_HELP}\n").into_bytes()),
            "--json" => json = true,
            "--default" => default = true,
            "--ranges" => ranges = true,
            "--budget-mb" => match it.next().and_then(|v| v.parse::<f64>().ok()) {
                Some(v) if v >= 0.0 => budget = v,
                _ => return Outcome::fail("--budget-mb needs a number of MB"),
            },
            "--sample" => match it.next() {
                Some(v) => text = Some(v),
                None => return Outcome::fail("--sample needs a list such as 1,3,5"),
            },
            o if o.starts_with('-') && o.len() > 1 => {
                return Outcome::fail(&format!("unknown option {o} (sample select --help)"))
            }
            _ if file.is_none() => file = Some(PathBuf::from(a)),
            _ => return Outcome::fail("sample select takes one FILE"),
        }
    }
    let (Some(file), text) = (file, text) else {
        return Outcome::fail("sample select needs a FILE and --sample LIST or --default");
    };
    if text.is_some() == default {
        return Outcome::fail("sample select needs exactly one of --sample LIST and --default");
    }
    if ranges && text.is_none() {
        return Outcome::fail("--ranges goes with --sample LIST");
    }
    if !file.exists() {
        let e = Error::new("err.session.fileNotFound").with("name", file.display());
        return Outcome::fail_error(&e).with_hint("check the path");
    }
    if let Err(e) = crate::sample_select::refuse_mzml(&file) {
        return crate::sample_select::fail(&e);
    }
    if ranges {
        return select_ranges(&file, text.as_deref().unwrap_or_default(), json);
    }
    select_ids(&file, text.as_deref(), budget, json)
}

/// `sample select FILE --sample LIST --ranges`: the byte ranges to copy for the chosen samples.
fn select_ranges(file: &Path, text: &str, json: bool) -> Outcome {
    match crate::sample_select::parse(text).and_then(|w| crate::sample_select::plan(file, &w)) {
        Err(e) => crate::sample_select::fail(&e),
        Ok(p) if json => Outcome::ok(format!("{}\n", p.json()).into_bytes()),
        Ok(p) => Outcome::ok(p.text().into_bytes()),
    }
}

/// The sample numbers `sample select` prints: the typed list resolved, or the suggestion within the budget.
fn select_ids(file: &Path, text: Option<&str>, budget: f64, json: bool) -> Outcome {
    let chosen = match text {
        Some(t) => {
            crate::sample_select::parse(t).and_then(|w| crate::sample_select::resolve(file, &w))
        }
        None => crate::sample_select::sized(file).map(|(ids, sizes)| {
            mzlab_core::readers::selection::suggest(
                &ids,
                &sizes,
                mzlab_core::readers::selection::budget_bytes(budget),
            )
        }),
    };
    match chosen {
        Err(e) => crate::sample_select::fail(&e),
        Ok(ids) => {
            let ids: Vec<String> = ids.iter().map(u32::to_string).collect();
            let out = if json {
                format!(
                    "{{\"schema\":\"sample-select/1\",\"samples\":[{}]}}\n",
                    ids.join(",")
                )
            } else {
                format!("{}\n", ids.join(" "))
            };
            Outcome::ok(out.into_bytes())
        }
    }
}

/// `sample list FILE... [--json] [--probe]`.
fn list_run(args: Vec<String>) -> Outcome {
    let (mut files, mut json, mut probe) = (Vec::<PathBuf>::new(), false, false);
    for a in args {
        match a.as_str() {
            "--help" | "-h" => return Outcome::ok(format!("{LIST_HELP}\n").into_bytes()),
            "--json" => json = true,
            "--probe" => probe = true,
            o if o.starts_with('-') && o.len() > 1 => {
                return Outcome::fail(&format!("unknown option {o} (sample list --help)"))
            }
            _ => files.push(PathBuf::from(a)),
        }
    }
    if files.is_empty() {
        return Outcome::fail("sample list needs at least one FILE");
    }
    let (mut docs, mut table, mut err, mut code) =
        (Vec::<String>::new(), String::new(), String::new(), 0u8);
    for f in &files {
        err += &format!("listing {}\n", f.display());
        match list_one(f, probe) {
            Ok(l) => {
                if json {
                    docs.push(l.json.clone());
                } else {
                    table += &list_table(&l.json, &f.display().to_string());
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
            Err((c, e)) if c == crate::exit::DATA => {
                // a file the reader refuses is part of the answer: its error/1 takes its place among the listings
                err += &format!("error: {}\n", crate::error_text::human_error(&e));
                let hint = if e.key == "err.file.alreadyMzml" {
                    "an mzML has one sample and no container to list; `mzlab-cli file info` describes it"
                } else {
                    "check the path and that the whole pair was copied"
                };
                docs.push(crate::contract::error_line(c, &e, hint, "sample list"));
                code = code.max(c);
            }
            Err((_, e)) => {
                // missing or unreadable: no listing is the answer, one error/1 carries the files listed so far
                let mut o = Outcome::fail_error(&e).with_hint(if e.key == "err.file.wiffGrowing" {
                    "open it again when the acquisition has ended"
                } else {
                    "check the path and that the whole pair was copied"
                });
                o.stderr += &err;
                o.stdout = list_document(&docs).into_bytes();
                return o;
            }
        }
    }
    Outcome {
        code,
        stdout: if json { list_document(&docs) } else { table }.into_bytes(),
        stderr: err,
        error: None,
    }
}

/// The `sample-list/1` document: one `samples_list/1` per file, in the order given; a file the reader refuses has its `error/1` in its place.
fn list_document(docs: &[String]) -> String {
    format!(
        "{{\"schema\":\"sample-list/1\",\"files\":[{}]}}\n",
        docs.join(",")
    )
}

pub fn run(args: Vec<String>) -> Outcome {
    let mut it = args.into_iter();
    let first = it.next();
    let sub = first.as_deref();
    let (op, body) = match sub {
        Some("vars") => ("variables", variables_args(it.collect())),
        Some("table") => return crate::samples_table::run(it.collect()),
        Some("import") => return crate::samples_import::run(it.collect()),
        Some("group") => {
            let n: Vec<String> = it.collect();
            if let Err(e) = crate::argv::no_options(&n) {
                return Outcome::fail_error(&e);
            }
            ("group_name", Ok(format!("{{\"names\":{}}}", list(&n))))
        }
        Some("list") => return list_run(it.collect()),
        Some("select") => return select_run(it.collect()),
        Some("design") => return crate::design_cmd::run(it.collect()),
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
