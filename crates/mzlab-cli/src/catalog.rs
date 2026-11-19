//! `mzlab-cli commands [--json] [PATH...]`: the catalog `commands/1` of every command, built from the rows of the command table. The usage line of
//! `--help` is the one source: arguments and flags are read from it, so the help and the catalog cannot drift (`catalog_tests.rs`).

use mzlab_core::json::Value;
use mzlab_core::jsonb::{text, write, Obj};

use crate::cmd::{entries, Entry};
use crate::{exit, Outcome};

/// Flags every command accepts, listed once instead of on every command.
const GLOBAL: &[(&str, &str)] = &[
    (
        "--json",
        "the data as JSON on stdout; a failure is one error/1 document",
    ),
    ("--help", "the usage of the command on stdout, exit 0"),
];

/// What a flag means, by name; the usage line says only its value.
const FLAG_HELP: &[(&str, &str)] = &[
    ("--out", "write to this file, not stdout"),
    ("--set", "one NAME=VALUE setting; repeat the flag for more"),
    ("--sample", "sample ids: 1,3,5 or 2-4"),
    ("--from", "start of the window, minutes"),
    ("--to", "end of the window, minutes"),
    ("--at", "retention time, minutes"),
    ("--in", "input files, or folders that hold them"),
    ("--id", "a flow step or a channel id"),
    (
        "--budget",
        "the size budget of the document the assistant reads",
    ),
    (
        "--hide-names",
        "replace file and sample names by neutral labels",
    ),
    ("--demo", "the subset of the public demo"),
    ("--preset", "polarity preset of the Pipeline panel"),
    ("--turn", "the turn of the conversation to replay"),
    (
        "--tables",
        "the JSON file of tables (losses, adduct shifts, contaminants) to match against",
    ),
    ("--sub", "the substructure query"),
    ("--strict", "also compare the SHA-256 of the request"),
    ("--rules", "judge with the fragmentation rules"),
    ("--roles", "the JSON file that names the role of each file"),
    ("--name", "the name the answer carries"),
    ("--match", "also answer what this number is compatible with"),
    ("--full", "every part, with SHA-256"),
    ("--rt", "window in min: A or A,B"),
    ("--ms-level", "MS level, default 1"),
    ("--polarity", "keep one polarity: + or -"),
    ("--mz", "m/z"),
    ("--ppm", "tolerance, ppm"),
    ("--da", "tolerance, Da"),
    ("--n", "points kept per extreme"),
    ("--scan", "scan number"),
    ("--format", "json (default) or csv"),
    ("--formula", "the ion formula the graph starts from"),
    ("--decoy", "the seed of the decoy spectra"),
    ("--conv", "the conversation file"),
    ("--cache", "the folder of the downloaded official sources"),
    ("--adduct", "the adduct, with its signed shift"),
    ("--compact", "the small form: names and ports, no prose"),
    ("--list", "list the names instead of printing one"),
    (
        "--replay",
        "the folder of recordings to score the cases against",
    ),
    ("--against", "the sources the output is compared with"),
    (
        "--params",
        "the numbers of the Pipeline panel as a JSON file",
    ),
];

/// A value placeholder that gives its flag a type the placeholder alone cannot (`N` is a number or a name).
const FLAG_TYPE: &[(&str, &str, Option<&str>)] = &[
    ("--budget", "int", None),
    ("--turn", "int", None),
    ("--decoy", "int", None),
    ("--from", "number", Some("min")),
    ("--to", "number", Some("min")),
    ("--at", "number", Some("min")),
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
const EXIT_MEANING: &[(u8, &str)] = &[
    (exit::OK, "ok"),
    (exit::DATA, "data or domain error, partial outputs written"),
    (exit::USAGE, "usage"),
    (exit::INTEGRITY, "plan, schema or integrity"),
    (exit::INPUT, "input missing or unreadable"),
    (
        exit::SELECTOR,
        "selector not found (sample, channel, node, step)",
    ),
    (exit::OUTPUT, "output write or external source failure"),
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

fn flag_json(f: &Flag, with_required: bool) -> Value {
    let help = FLAG_HELP
        .iter()
        .chain(GLOBAL)
        .find(|(n, _)| *n == f.name)
        .map_or("", |(_, h)| *h);
    // Members that are null or false are left out: `commands --json` stays small.
    let mut o = Obj::new()
        .put("name", text(&f.name))
        .put("type", text(f.kind));
    if let Some(u) = f.unit {
        o = o.put("unit", text(u));
    }
    if let Some(d) = &f.default {
        o = o.put("default", text(d));
    }
    if !f.values.is_empty() {
        o = o.put(
            "enum",
            Value::Arr(f.values.iter().map(|v| text(v)).collect()),
        );
    }
    o = o.put("help", text(help));
    if f.repeat {
        o = o.b("repeat", true);
    }
    if with_required && f.required {
        o = o.b("required", true);
    }
    o.done()
}

/// The stdout of `name`: its format and the schema the entry names.
fn stdout_json(e: &Entry) -> Value {
    let json = crate::cmd::json_by_default(e.name);
    let format = if json {
        "json"
    } else if CSV.contains(&e.name) {
        "csv"
    } else {
        "text"
    };
    let mut o = Obj::new()
        .put("format", text(format))
        .put("schema", text(e.schema));
    if !json && crate::cmd::reads_json(e.name) {
        o = o.b("json_flag", true);
    }
    o.done()
}

/// The exit codes of `e`: the row's own list, else read from what the usage line shows: every command can end 0, 1 or 2; 3 for a plan or a verdict,
/// 4 when it reads a file, 5 when it selects a sample or step, 6 when it writes `--out`.
pub(crate) fn exits_of(e: &Entry, u: &Usage) -> Vec<u8> {
    if !e.exits.is_empty() {
        return e.exits.to_vec();
    }
    let has = |f: &str| u.flags.iter().any(|x| x.name == f);
    let reads = u.args.iter().any(|a| a.kind == "path" || a.kind == "rest")
        || u.flags
            .iter()
            .any(|f| f.kind == "path" && f.name != "--out");
    let mut v = vec![exit::OK, exit::DATA, exit::USAGE];
    let mut add = |on: bool, c: u8| {
        if on {
            v.push(c)
        }
    };
    add(
        e.name.starts_with("flow ") || crate::contract::VERDICT.contains(&e.name),
        exit::INTEGRITY,
    );
    add(reads, exit::INPUT);
    add(
        has("--sample")
            || has("--id")
            || e.name.starts_with("sample ")
            || e.name.starts_with("msn graph"),
        exit::SELECTOR,
    );
    add(has("--out"), exit::OUTPUT);
    v.sort_unstable();
    v
}

/// The usage line without its optional parts: a command line that runs as it stands once the names are replaced.
fn skeleton(u: &Usage) -> String {
    let toks = tokens(&u.synopsis);
    let kept: Vec<&str> = toks
        .iter()
        .map(String::as_str)
        .filter(|t| !t.starts_with('[') && *t != "...")
        .collect();
    format!("mzlab-cli {}", kept.join(" "))
}

fn command_json(e: &Entry) -> Value {
    let u = usage_of(e);
    let exits: Vec<Value> = exits_of(e, &u)
        .iter()
        .map(|c| Value::Num(f64::from(*c)))
        .collect();
    let examples: Vec<String> = if e.examples.is_empty() {
        vec![skeleton(&u)]
    } else {
        e.examples.iter().map(|x| x.to_string()).collect()
    };
    Obj::new()
        .put("path", text(e.name))
        .put("synopsis", text(&u.synopsis))
        .put("summary", text(&u.summary))
        .put(
            "args",
            Value::Arr(
                u.args
                    .iter()
                    .map(|a| {
                        Obj::new()
                            .put("name", text(&a.name))
                            .put("kind", text(a.kind))
                            .b("required", a.required)
                            .b("repeat", a.repeat)
                            .done()
                    })
                    .collect(),
            ),
        )
        .put(
            "flags",
            Value::Arr(u.flags.iter().map(|f| flag_json(f, true)).collect()),
        )
        .b("stdin", u.stdin)
        .put("stdout", stdout_json(e))
        .put("exit_codes", Value::Arr(exits))
        .put(
            "examples",
            Value::Arr(examples.iter().map(|x| text(x)).collect()),
        )
        .done()
}

/// Whether the words `path` are the start of the name of `e`.
fn under(e: &Entry, path: &[String]) -> bool {
    let words: Vec<&str> = e.name.split(' ').collect();
    path.len() <= words.len() && path.iter().zip(&words).all(|(p, w)| p == w)
}

/// The `commands/1` document for the entries under `path` (all of them when `path` is empty).
pub(crate) fn catalog_json(path: &[String]) -> String {
    let meaning = EXIT_MEANING
        .iter()
        .fold(Obj::new(), |o, (c, m)| o.put(&c.to_string(), text(m)))
        .done();
    let global: Vec<Value> = GLOBAL
        .iter()
        .map(|(n, _)| {
            flag_json(
                &Flag {
                    name: n.to_string(),
                    kind: "bool",
                    ..Flag::default()
                },
                false,
            )
        })
        .collect();
    let commands: Vec<Value> = entries()
        .filter(|e| under(e, path))
        .map(command_json)
        .collect();
    let doc = Obj::new()
        .put("schema", text("commands/1"))
        .put("citation", text(&crate::citation::text()))
        .put("exit_code_meaning", meaning)
        .put("global_flags", Value::Arr(global))
        .put("commands", Value::Arr(commands))
        .done();
    write(&doc) + "\n"
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
        return Outcome::fail_error(&err).with_hint("mzlab-cli commands lists every command");
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
