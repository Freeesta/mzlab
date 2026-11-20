//! `mzlab-cli commands [--json] [PATH...]`: the catalog `commands/1` of every command, built from the rows of the command table. The usage line of
//! `--help` is the one source: arguments and flags are read from it, so the help and the catalog cannot drift (`catalog_tests.rs`).

use crate::cmd::{entries, Entry};
mod doc;
pub(crate) use doc::catalog_json;
#[cfg(test)]
pub(crate) use doc::exits_of;
use doc::under;

use crate::{exit, Outcome};

/// Flags every command accepts, listed once instead of on every command.
const GLOBAL: &[(&str, &str)] = &[
    ("--json", "JSON on stdout"),
    (
        "--cache",
        "index cache folder or MZLAB_CACHE; not db normative",
    ),
    ("--help", "usage on stdout, exit 0"),
];

/// What a flag means, by name; the usage line says only its value.
const FLAG_HELP: &[(&str, &str)] = &[
    ("--out", "output file, not stdout"),
    ("--check", "exit 1 if changed"),
    ("--set", "NAME=VALUE"),
    ("--sample", "ids: 1,3,5 or 2-4"),
    ("--no-points", "omit the MS2 points"),
    ("--samples", "samples/1 file"),
    ("--from", "window start, min"),
    ("--to", "window end, min"),
    ("--at", "retention time, min"),
    ("--in", "input files or folders"),
    ("--id", "flow step or channel id"),
    ("--budget", "size budget"),
    ("--hide-names", "neutral names"),
    ("--demo", "demo subset"),
    ("--preset", "polarity preset"),
    ("--turn", "turn"),
    ("--tables", "JSON tables"),
    ("--sub", "substructure query"),
    ("--dry-run", "check only, write nothing"),
    ("--summary", "numbers only, msn-summary/1"),
    ("--strict", "also check hashes"),
    ("--rules", "use the rules"),
    ("--roles", "JSON file roles"),
    ("--name", "answer name"),
    ("--match", "also list compatible"),
    ("--full", "every part + SHA-256"),
    ("--rt", "window in min: A or A,B"),
    ("--ms-level", "MS level, default 1"),
    ("--baseline", "baseline mode"),
    ("--noise", "noise method"),
    ("--split", "split rule"),
    ("--end", "the moving end, min"),
    ("--other", "the fixed end, min"),
    ("--step", "points to move by"),
    ("--tr", "transition index"),
    ("--a", "start, min"),
    ("--b", "end, min"),
    ("--depth", "valley depth, fraction of the lower peak"),
    ("--windows", "JSON file of windows"),
    ("--auto", "engine peaks, as JSON"),
    ("--traces", "JSON file of traces"),
    ("--columns", "sn, fig, quality"),
    ("--max-shift", "farthest a followed window moves, min"),
    ("--polarity", "+ or -"),
    ("--mz", "m/z"),
    ("--ppm", "ppm"),
    ("--rt-tol", "RT window, min"),
    ("--inspection", "inspection/1, or -"),
    ("--answers", "answers"),
    ("--da", "tolerance, Da"),
    ("--tol", "10ppm or 0.2da"),
    ("--smooth", "3 or 5 points"),
    ("--n", "points per extreme"),
    ("--scan", "scan number"),
    ("--minus", "blank file to subtract"),
    ("--minus-sample", "sample of the blank"),
    ("--minus-rt", "blank window, min"),
    ("--format", "output format (values in the usage line)"),
    ("--formula", "start ion formula"),
    ("--decoy", "decoy spectra seed"),
    ("--conv", "conversation"),
    ("--cache", "sources folder"),
    ("--adduct", "adduct with signed shift"),
    ("--compact", "compact form"),
    ("--list", "list names"),
    ("--template", "a one-step plan"),
    ("--replay", "recordings"),
    ("--against", "sources to compare"),
    ("--params", "Pipeline numbers"),
];

/// A value placeholder that gives its flag a type the placeholder alone cannot (`N` is a number or a name).
const FLAG_TYPE: &[(&str, &str, Option<&str>)] = &[
    ("--budget", "int", None),
    ("--turn", "int", None),
    ("--decoy", "int", None),
    ("--from", "number", Some("min")),
    ("--to", "number", Some("min")),
    ("--at", "number", Some("min")),
    ("--max-shift", "number", Some("min")),
];

/// Defaults the commands apply when the flag is absent (only those the code states).
const FLAG_DEFAULT: &[(&str, &str, &str)] = &[
    ("features", "--preset", "auto"),
    ("llm context", "--budget", "8000"),
    ("llm inspect", "--budget", "30000"),
    ("llm params", "--budget", "30000"),
];

/// Commands whose stdout is a CSV table.
const CSV: &[&str] = &["features", "peak table"];

/// What the exit codes mean (`exit.rs`).
pub(crate) const EXIT_MEANING: &[(u8, &str)] = &[
    (exit::OK, "ok"),
    (exit::DATA, "data error, partial outputs written"),
    (exit::USAGE, "usage"),
    (exit::INTEGRITY, "plan, schema or integrity"),
    (exit::INPUT, "input missing or unreadable"),
    (
        exit::SELECTOR,
        "selector not found (sample, channel, node, step)",
    ),
    (exit::OUTPUT, "output write or external source failure"),
    (exit::INTERNAL, "internal error (a bug): any command"),
    (exit::GROWING, "input still being written"),
];

#[derive(Debug, Clone, Default)]
pub(crate) struct Flag {
    pub name: String,
    pub kind: &'static str,
    pub unit: Option<&'static str>,
    pub default: Option<String>,
    pub values: Vec<String>,
    pub repeat: bool,
    pub required: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct Arg {
    pub name: String,
    pub kind: &'static str,
    pub required: bool,
    pub repeat: bool,
}

#[derive(Debug, Default)]
pub(crate) struct Usage {
    pub synopsis: String,
    pub summary: String,
    pub args: Vec<Arg>,
    pub flags: Vec<Flag>,
    pub stdin: bool,
}

/// The words of `s` split at spaces outside square brackets.
fn tokens(s: &str) -> Vec<String> {
    let (mut out, mut cur, mut depth) = (Vec::new(), String::new(), 0i32);
    for c in s.chars() {
        match c {
            '[' => depth += 1,
            ']' => depth -= 1,
            _ => {}
        }
        if c == ' ' && depth == 0 {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
        } else {
            cur.push(c);
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn flag_of(name: &str, value: Option<&str>, required: bool) -> Flag {
    let mut f = Flag {
        name: name.to_string(),
        kind: "bool",
        required,
        ..Flag::default()
    };
    let Some(v) = value else { return f };
    f.repeat = v.ends_with("...");
    let ph = v.trim_end_matches("...");
    let alts: Vec<&str> = ph.split('|').collect();
    f.kind = if ph.contains('=') {
        "set"
    } else if alts.len() > 1
        && alts
            .iter()
            .all(|a| !a.is_empty() && a.chars().all(|c| c.is_ascii_lowercase()))
    {
        f.values = alts.iter().map(|a| a.to_string()).collect();
        "enum"
    } else if ph == "LIST" {
        "list"
    } else if ph.contains("FILE")
        || ph.contains("PATH")
        || ph.contains("DIR")
        || ph.contains("FOLDER")
        || ph.contains(".json")
        || ph.contains(".wiff")
    {
        "path"
    } else {
        "string"
    };
    if let Some((_, k, u)) = FLAG_TYPE.iter().find(|(n, _, _)| *n == name) {
        f.kind = k;
        f.unit = *u;
    }
    f
}

/// Reads the tokens of a usage line into `u`; `optional` is true inside square brackets.
fn read(toks: &[String], optional: bool, u: &mut Usage) {
    let mut i = 0;
    while i < toks.len() {
        let t = toks[i].as_str();
        i += 1;
        if let Some(inner) = t.strip_prefix('[').and_then(|x| x.strip_suffix(']')) {
            read(&tokens(inner), true, u);
        } else if t.starts_with('(') {
            // `(--ppm P | --da D)`: alternatives, one of which is needed (the synopsis says so); none is required by itself
            let end = toks[i - 1..]
                .iter()
                .position(|w| w.ends_with(')'))
                .map_or(toks.len(), |k| i + k);
            let mut group: Vec<String> = toks[i - 1..end].to_vec();
            group[0] = group[0].trim_start_matches('(').to_string();
            if let Some(last) = group.last_mut() {
                *last = last.trim_end_matches(')').to_string();
            }
            read(&group, true, u);
            i = end;
        } else if t == "|" {
        } else if t == "..." {
            u.args.push(Arg {
                name: "...".into(),
                kind: "rest",
                required: false,
                repeat: true,
            });
        } else if t.starts_with("--") {
            let mut value = None;
            if toks
                .get(i)
                .is_some_and(|n| !n.starts_with('-') && n != "|" && n != "...")
            {
                value = Some(toks[i].as_str());
                i += 1;
                // `--set hr=1 ppm=N`: the next words of the same setting
                while toks
                    .get(i)
                    .is_some_and(|n| n.contains('=') && !n.starts_with(['-', '[']))
                {
                    i += 1;
                }
            }
            if !u.flags.iter().any(|f| f.name == t) {
                u.flags.push(flag_of(t, value, !optional));
            }
        } else {
            let repeat = t.ends_with("...");
            let mut name = t.trim_end_matches("...");
            if let Some(n) = name.strip_suffix("|-") {
                u.stdin = true;
                name = n;
            }
            let path = [
                "FILE", "PATH", "DIR", "TRACE", "RECORD", "PLAN", "GRAPH", "MOLFILE", "REQUEST",
                "CSV", "INPUT", "OUT", "RUNDIR", "TABLE", "REG", "CASES", "LINES", ".json", ".csv",
                ".mzML", ".wiff",
            ]
            .iter()
            .any(|k| name.contains(k));
            u.args.push(Arg {
                name: name.to_string(),
                kind: if path { "path" } else { "value" },
                required: !optional,
                repeat,
            });
        }
    }
}

/// The usage of `e` read from its help line (`mzlab-cli PATH ARGS...   summary`); a hidden command has only its path.
pub(crate) fn usage_of(e: &Entry) -> Usage {
    let mut u = Usage::default();
    let (line, summary) = e.help.split_once("   ").unwrap_or((e.help, ""));
    u.summary = summary.trim().to_string();
    let line = line.strip_prefix("mzlab-cli ").unwrap_or(e.name);
    u.synopsis = if e.help.is_empty() {
        e.name.to_string()
    } else {
        line.to_string()
    };
    let words = e.name.split(' ').count();
    let toks = tokens(line);
    read(toks.get(words..).unwrap_or(&[]), false, &mut u);
    for f in &mut u.flags {
        if let Some((_, _, d)) = FLAG_DEFAULT
            .iter()
            .find(|(c, n, _)| *c == e.name && *n == f.name)
        {
            f.default = Some(d.to_string());
        }
    }
    u
}

/// `mzlab-cli commands [--json] [PATH...]`.
pub(crate) fn run(args: Vec<String>) -> Outcome {
    let mut json = false;
    let mut path = Vec::new();
    for a in args {
        match a.as_str() {
            "--json" => json = true,
            s if s.starts_with('-') => {
                return Outcome::fail(&format!("unknown option {s}"));
            }
            _ => path.push(a),
        }
    }
    if !entries().any(|e| under(e, &path)) {
        let err = mzlab_core::Error::new("err.cli.command.unknown").with("path", path.join(" "));
        let first: Vec<String> = path.first().map(|w| w.to_string()).into_iter().collect();
        let hint = match crate::renamed::fate(&first) {
            Some((old, crate::renamed::Fate::Now(now))) => {
                format!("{old} is now {now}: mzlab-cli commands --json {now}")
            }
            _ => "mzlab-cli commands lists every command".to_string(),
        };
        return Outcome::fail_error(&err).with_hint(&hint);
    }
    if json {
        return Outcome::ok(catalog_json(&path).into_bytes());
    }
    let lines: String = entries()
        .filter(|e| under(e, &path) && !e.help.is_empty())
        .map(|e| format!("{}  {}\n", e.name, usage_of(e).summary))
        .collect();
    Outcome::ok(lines.into_bytes())
}
