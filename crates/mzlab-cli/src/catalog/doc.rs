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

/// The type of each flag (other than string, the default) that two or more commands carry and all give the same type: said once at the top of the compact listing (`flag_type`).
fn shared_types() -> BTreeMap<String, &'static str> {
    let mut seen: BTreeMap<String, (usize, Option<&'static str>)> = BTreeMap::new();
    for e in entries() {
        for f in usage_of(e).flags {
            let s = seen.entry(f.name).or_insert((0, Some(f.kind)));
            s.0 += 1;
            if s.1 != Some(f.kind) {
                s.1 = None;
            }
        }
    }
    seen.into_iter()
        .filter_map(|(n, (c, k))| k.filter(|k| c > 1 && *k != "string").map(|k| (n, k)))
        .collect()
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

/// `with_help` false: the flag is described once at the top of the compact listing (`flag_help`); `with_type` false: its type is in `flag_type`.
fn flag_json(
    f: &Flag,
    with_required: bool,
    with_help: bool,
    with_type: bool,
    global: bool,
) -> Value {
    // Members that are null or false are left out: `commands --json` stays small.
    let mut o = Obj::new().put("name", text(&f.name));
    if with_type {
        o = o.put("type", text(f.kind));
    }
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
fn stdout_json(e: &Entry, compact: bool) -> Value {
    let json = crate::cmd::json_by_default(e.name);
    let format = if json {
        "json"
    } else if CSV.contains(&e.name) {
        "csv"
    } else {
        "text"
    };
    let json_flag = !json && crate::cmd::reads_json(e.name);
    let mut o = Obj::new();
    // the compact listing says once what a command with a `json_flag` prints without it (`stdout_note`): text
    if !(compact && (json || json_flag && format == "text")) {
        o = o.put("format", text(format));
    }
    // like every other member, a schema that is not there is left out
    if !e.schema.is_empty() {
        o = o.put("schema", text(e.schema));
    }
    if json_flag {
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
pub(super) fn skeleton(u: &Usage) -> String {
    let toks = tokens(&u.synopsis);
    let kept: Vec<&str> = toks
        .iter()
        .map(String::as_str)
        .filter(|t| !t.starts_with('[') && *t != "...")
        .collect();
    format!("mzlab-cli {}", kept.join(" "))
}

/// The arguments of the usage line; the whole listing marks the exception, an argument in brackets (`optional`), the path listing says `required`.
fn args_json(u: &Usage, compact: bool) -> Vec<Value> {
    u.args
        .iter()
        .map(|a| {
            let mut o = Obj::new().put("name", text(&a.name));
            if !(compact && (a.kind == "path" || a.kind == "rest" && a.name == "...")) {
                o = o.put("kind", text(a.kind));
            }
            if compact && !a.required {
                o = o.b("optional", true);
            } else if !compact && a.required {
                o = o.b("required", true);
            }
            if a.repeat {
                o = o.b("repeat", true);
            }
            o.done()
        })
        .collect()
}

/// `compact`: leave out what the top of the listing or the usage line already says (`flag_help`, `flag_type`, an argument of kind `path`,
/// the derived exit codes, a skeleton example, a stdout of JSON without a schema).
fn command_json(
    e: &Entry,
    compact: bool,
    uses: &BTreeMap<String, usize>,
    types: &BTreeMap<String, &'static str>,
) -> Value {
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
        .put("summary", text(&u.summary));
    let args = args_json(&u, compact);
    let flags: Vec<Value> = u
        .flags
        .iter()
        .map(|f| {
            let shared = compact && uses.get(&f.name) > Some(&1);
            // the type is the flag's own, else its `flag_type`, else string
            let typed = compact
                && match types.get(&f.name) {
                    Some(k) => *k == f.kind,
                    None => f.kind == "string",
                };
            flag_json(f, true, !shared, !typed, false)
        })
        .collect();
    // the whole listing leaves out an empty list (`flag_help_note` says so)
    if !(compact && args.is_empty()) {
        o = o.put("args", Value::Arr(args));
    }
    if !(compact && flags.is_empty()) {
        o = o.put("flags", Value::Arr(flags));
    }
    // the most common format, JSON, is said once at the top (`stdout_note`): no format, and no stdout when nothing else is left
    if !(compact && crate::cmd::json_by_default(e.name) && e.schema.is_empty()) {
        o = o.put("stdout", stdout_json(e, compact));
    }
    if u.stdin && !compact {
        o = o.b("stdin", true);
    }
    if !compact || exits != derived_exits(e, &u) {
        let v = exits.iter().map(|c| Value::Num(f64::from(*c))).collect();
        o = o.put("exit_codes", Value::Arr(v));
    }
    if !compact {
        o = related(o, e.name);
    }
    if !compact || examples != [skeleton(&u)] {
        o = o.put(
            "examples",
            Value::Arr(examples.iter().map(|x| text(x)).collect()),
        );
    }
    o.done()
}

/// The full form names the related commands and steps and the one call that replaces a loop of this one (`related.rs`).
fn related(mut o: Obj, name: &str) -> Obj {
    if let Some(r) = super::related::of(name) {
        o = o.put(
            "see_also",
            Value::Arr(r.also.iter().map(|a| text(a)).collect()),
        );
        if !r.bulk.is_empty() {
            o = o.put("bulk", text(r.bulk));
        }
    }
    o
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
            flag_json(&f, false, true, true, true)
        })
        .collect();
    let uses = flag_uses();
    let types = shared_types();
    let commands: Vec<Value> = entries()
        .filter(|e| under(e, path))
        .map(|e| command_json(e, compact, &uses, &types))
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
        let types_json = types
            .iter()
            .fold(Obj::new(), |o, (name, k)| o.put(name, text(k)))
            .done();
        doc = doc
            .b("compact", true)
            .put("detail", text("mzlab-cli commands --json PATH... gives each command in full"))
            .put("flag_help", shared)
            .put("flag_type", types_json)
            .put("flag_help_note", text("no flag help or type: see flag_help, flag_type (else string); no arg kind: path, or `...` (rest); no stdin: true if an argument is `NAME|-`; no args or flags: none; an arg is required unless `optional`"))
            .put("exit_codes_note", text("no exit_codes: 0, 1, 2; +3 a `flow ` path or VERDICT; +4 a path or rest argument or a path flag but --out; +5 --sample, --id or a `sample `, `dad ` or `msn graph` path; +6 --out"))
            .put("examples_note", text("no examples: `mzlab-cli ` + synopsis minus [optional] groups and `...`"))
            .put("stdout_note", text("no stdout format: json, or text with json_flag; no stdout: json, no schema"));
    }
    write(&doc.put("commands", Value::Arr(commands)).done()) + "\n"
}
