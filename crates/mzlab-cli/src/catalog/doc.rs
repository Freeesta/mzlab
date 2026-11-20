//! The `commands/1` document: one object per command from the usage line. The whole listing is compact (what is shared or derivable is said once
//! at the top); `commands PATH...` gives every command in full, so the 40 KB budget of `commands --json` holds as commands are added.

use std::collections::BTreeMap;

use mzlab_core::json::Value;
use mzlab_core::jsonb::{text, write, Obj};

use super::{tokens, usage_of, Flag, Usage, CSV, EXIT_MEANING, FLAG_HELP, GLOBAL};
use crate::cmd::{entries, Entry};
use crate::exit;

/// How many commands carry each flag: a flag on two or more has its help said once at the top of the compact listing.
fn flag_uses() -> BTreeMap<String, usize> {
    let mut n = BTreeMap::new();
    for e in entries() {
        for f in usage_of(e).flags {
            *n.entry(f.name).or_insert(0) += 1;
        }
    }
    n
}

fn help_of(name: &str) -> &'static str {
    FLAG_HELP
        .iter()
        .chain(GLOBAL)
        .find(|(n, _)| *n == name)
        .map_or("", |(_, h)| *h)
}

/// A global flag's help comes from GLOBAL first: a command's own flag of the same name (`db normative build --cache`) has other help.
fn global_help(name: &str) -> &'static str {
    GLOBAL
        .iter()
        .find(|(n, _)| *n == name)
        .map_or("", |(_, h)| *h)
}

/// `with_help` false: the flag is described once at the top of the compact listing (`flag_help`).
fn flag_json(f: &Flag, with_required: bool, with_help: bool, global: bool) -> Value {
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
    if with_help {
        let h = if global {
            global_help(&f.name)
        } else {
            help_of(&f.name)
        };
        o = o.put("help", text(h));
    }
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
    let mut o = Obj::new().put("format", text(format));
    // like every other member, a schema that is not there is left out
    if !e.schema.is_empty() {
        o = o.put("schema", text(e.schema));
    }
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
    derived_exits(e, u)
}

/// The exit codes the usage line implies (what `exits_of` gives a row with no list of its own).
fn derived_exits(e: &Entry, u: &Usage) -> Vec<u8> {
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
            || e.name.starts_with("dad ")
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

/// `compact`: leave out what the top of the listing or the usage line already says (`flag_help`, the derived exit codes, a skeleton example).
fn command_json(e: &Entry, compact: bool, uses: &BTreeMap<String, usize>) -> Value {
    let u = usage_of(e);
    let exits = exits_of(e, &u);
    let examples: Vec<String> = if e.examples.is_empty() {
        vec![skeleton(&u)]
    } else {
        e.examples.iter().map(|x| x.to_string()).collect()
    };
    let mut o = Obj::new()
        .put("path", text(e.name))
        .put("synopsis", text(&u.synopsis))
        .put("summary", text(&u.summary))
        .put(
            "args",
            Value::Arr(
                u.args
                    .iter()
                    .map(|a| {
                        let mut o = Obj::new()
                            .put("name", text(&a.name))
                            .put("kind", text(a.kind));
                        if a.required {
                            o = o.b("required", true);
                        }
                        if a.repeat {
                            o = o.b("repeat", true);
                        }
                        o.done()
                    })
                    .collect(),
            ),
        )
        .put(
            "flags",
            Value::Arr(
                u.flags
                    .iter()
                    .map(|f| flag_json(f, true, !(compact && uses.get(&f.name) > Some(&1)), false))
                    .collect(),
            ),
        )
        .put("stdout", stdout_json(e));
    if u.stdin {
        o = o.b("stdin", true);
    }
    if !compact || exits != derived_exits(e, &u) {
        let v = exits.iter().map(|c| Value::Num(f64::from(*c))).collect();
        o = o.put("exit_codes", Value::Arr(v));
    }
    if !compact || examples != [skeleton(&u)] {
        o = o.put(
            "examples",
            Value::Arr(examples.iter().map(|x| text(x)).collect()),
        );
    }
    o.done()
}

/// Whether the words `path` are the start of the name of `e`.
pub(super) fn under(e: &Entry, path: &[String]) -> bool {
    let words: Vec<&str> = e.name.split(' ').collect();
    path.len() <= words.len() && path.iter().zip(&words).all(|(p, w)| p == w)
}

/// The `commands/1` document for the entries under `path`: all of them, compact, when `path` is empty; else each one in full.
pub(crate) fn catalog_json(path: &[String]) -> String {
    let compact = path.is_empty();
    let meaning = EXIT_MEANING
        .iter()
        .fold(Obj::new(), |o, (c, m)| o.put(&c.to_string(), text(m)))
        .done();
    let global: Vec<Value> = GLOBAL
        .iter()
        .map(|(n, _)| {
            let f = Flag {
                name: n.to_string(),
                kind: if *n == "--cache" { "path" } else { "bool" },
                ..Flag::default()
            };
            flag_json(&f, false, true, true)
        })
        .collect();
    let uses = flag_uses();
    let commands: Vec<Value> = entries()
        .filter(|e| under(e, path))
        .map(|e| command_json(e, compact, &uses))
        .collect();
    let mut doc = Obj::new()
        .put("schema", text("commands/1"))
        .put("citation", text(&crate::citation::text()))
        .put("exit_code_meaning", meaning)
        .put("global_flags", Value::Arr(global));
    if compact {
        let shared = uses
            .iter()
            .filter(|(_, n)| **n > 1)
            .fold(Obj::new(), |o, (name, _)| o.put(name, text(help_of(name))))
            .done();
        doc = doc
            .b("compact", true)
            .put("detail", text("mzlab-cli commands --json PATH... gives each command in full"))
            .put("flag_help", shared)
            .put("flag_help_note", text("a flag with no help of its own takes it from flag_help"))
            .put("exit_codes_note", text("with no exit_codes: 0, 1, 2, plus 3 for a `flow ` path or a verdict command (VERDICT), 4 with a path or rest argument or a path flag other than --out, 5 with --sample or --id or a `sample `, `dad ` or `msn graph` path, 6 with --out; `commands --json PATH` gives the exact codes"))
            .put("examples_note", text("no examples: the example is `mzlab-cli ` + the synopsis without [optional] groups (a word with a bracket inside is kept) and `...` tokens"));
    }
    write(&doc.put("commands", Value::Arr(commands)).done()) + "\n"
}
