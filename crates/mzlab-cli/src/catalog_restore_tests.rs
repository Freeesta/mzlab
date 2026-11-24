//! The rules of `flag_help_note` that read the synopsis: the compact `commands --json` leaves out a flag's `required`, `repeat` and enum values and the
//! `args` of a command whose arguments are all paths. `restored_args` and `restored_flag` put them back from the usage line alone, and the result is the
//! full listing of the command (`commands --json PATH`).

use crate::catalog_implied_tests::flags_of;
use crate::cmd::entries;
use crate::execute;
use serde_json::{json, Value};

/// A word of the usage line: its text without the `[ ] ( )` that open or close a group, whether it is inside one, and which group (a flag takes the next word of its own group as its value).
struct Word {
    text: String,
    optional: bool,
    group: usize,
}

/// The words of `synopsis` after the `skip` words of the command path. A `[` or `(` opens a group only at the start of a word (`bonds=A-B[,...]` keeps its bracket).
fn words(synopsis: &str, skip: usize) -> Vec<Word> {
    let (mut out, mut cur) = (Vec::new(), String::new());
    let (mut stack, mut next, mut inner) = (vec![0usize], 1usize, 0);
    let (mut opt, mut group) = (false, 0usize);
    let flush = |cur: &mut String, opt: &mut bool, group: usize, out: &mut Vec<Word>| {
        if !cur.is_empty() {
            out.push(Word {
                text: std::mem::take(cur),
                optional: *opt,
                group,
            });
        }
        *opt = false;
    };
    for ch in synopsis.chars().chain(std::iter::once(' ')) {
        match ch {
            '[' | '(' if cur.is_empty() => {
                stack.push(next);
                next += 1;
            }
            '[' => {
                inner += 1;
                cur.push(ch);
            }
            ']' if inner > 0 => {
                inner -= 1;
                cur.push(ch);
            }
            ']' | ')' if stack.len() > 1 => {
                // the closing word still belongs to the group it closes
                if cur.is_empty() {
                    stack.pop();
                } else {
                    opt = true;
                    group = *stack.last().unwrap();
                    flush(&mut cur, &mut opt, group, &mut out);
                    stack.pop();
                }
            }
            ' ' => flush(&mut cur, &mut opt, group, &mut out),
            _ => {
                if cur.is_empty() {
                    group = *stack.last().unwrap();
                }
                opt |= stack.len() > 1;
                cur.push(ch);
            }
        }
    }
    out.into_iter().skip(skip).collect()
}

/// The words read as the catalog's rules say: each flag with the word that is its value (if any), each other word an operand.
enum Item<'a> {
    Flag(&'a Word, Option<&'a Word>),
    Operand(&'a Word),
}

fn items(ws: &[Word]) -> Vec<Item<'_>> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < ws.len() {
        let w = &ws[i];
        i += 1;
        if w.text == "|" {
        } else if w.text.starts_with("--") {
            let mut value = None;
            if let Some(n) = ws.get(i).filter(|n| {
                n.group == w.group && !n.text.starts_with('-') && n.text != "|" && n.text != "..."
            }) {
                value = Some(n);
                i += 1;
                // `--set hr=1 ppm=N`: the next words of the same setting
                while ws
                    .get(i)
                    .is_some_and(|n| n.text.contains('=') && !n.text.starts_with('-'))
                {
                    i += 1;
                }
            }
            out.push(Item::Flag(w, value));
        } else {
            out.push(Item::Operand(w));
        }
    }
    out
}

/// Whether the usage line of `c` writes the flag `name` outside every group.
pub(crate) fn is_required(c: &Value, name: &str) -> bool {
    let ws = words(c["synopsis"].as_str().unwrap(), skip_of(c));
    items(&ws)
        .iter()
        .any(|it| matches!(it, Item::Flag(w, _) if w.text == name && !w.optional))
}

fn skip_of(c: &Value) -> usize {
    c["path"].as_str().unwrap().split(' ').count()
}

/// The `args` of a compact command as the listing would show them: its own list, else the operands of the synopsis, each a path (`...` a rest).
pub(crate) fn restored_args(c: &Value) -> Vec<Value> {
    if let Some(a) = c.get("args") {
        return a.as_array().unwrap().clone();
    }
    let ws = words(c["synopsis"].as_str().unwrap(), skip_of(c));
    items(&ws)
        .into_iter()
        .filter_map(|it| match it {
            Item::Operand(w) => Some(w),
            Item::Flag(..) => None,
        })
        .map(|w| {
            let name = w.text.trim_end_matches("...");
            let name = name.strip_suffix("|-").unwrap_or(name);
            let mut a = json!({ "name": if w.text == "..." { "..." } else { name } });
            if w.optional {
                a["optional"] = json!(true);
            }
            if w.text.ends_with("...") && w.text != "..." {
                a["repeat"] = json!(true);
            }
            if w.text == "..." {
                a["repeat"] = json!(true);
                a["optional"] = json!(true);
            }
            a
        })
        .collect()
}

/// The flag objects of a compact command with `required`, `repeat` and an enum's values put back from the synopsis (and the flags of `flags_of`).
pub(crate) fn restored_flags(c: &Value, doc: &Value) -> Vec<Value> {
    let ws = words(c["synopsis"].as_str().unwrap(), skip_of(c));
    let its = items(&ws);
    flags_of(c)
        .into_iter()
        .map(|mut f| {
            let name = f["name"].as_str().unwrap().to_string();
            // the first time the usage line writes the flag decides, like the catalog
            let (w, v) = its
                .iter()
                .find_map(|it| match it {
                    Item::Flag(w, v) if w.text == name => Some((*w, *v)),
                    _ => None,
                })
                .unwrap();
            if !w.optional {
                f["required"] = json!(true);
            }
            let value = v.map(|v| v.text.as_str());
            if value.is_some_and(|v| v.ends_with("...")) {
                f["repeat"] = json!(true);
            }
            let kind = f
                .get("type")
                .or_else(|| doc["flag_type"].get(&name))
                .map(|k| k.as_str().unwrap());
            if kind == Some("enum") && f.get("enum").is_none() {
                let alts = value.unwrap().trim_end_matches("...").split('|');
                f["enum"] = json!(alts.collect::<Vec<_>>());
            }
            f
        })
        .collect()
}

fn run(args: &[&str]) -> Value {
    let o = execute(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>());
    assert_eq!(o.code, 0, "{args:?}: {}", o.stderr);
    serde_json::from_slice(&o.stdout).unwrap()
}

/// What an argument says in the full listing: its kind (a path unless it says `value`, `...` a rest) and `required` unless it says `optional`.
fn full_arg(a: &Value) -> Value {
    let mut o = json!({ "name": a["name"] });
    o["kind"] = match a.get("kind") {
        Some(k) => k.clone(),
        None if a["name"] == "..." => json!("rest"),
        None => json!("path"),
    };
    if a.get("optional").is_none() {
        o["required"] = json!(true);
    }
    if a.get("repeat").is_some() {
        o["repeat"] = json!(true);
    }
    o
}

#[test]
fn the_args_and_flags_put_back_from_the_synopsis_are_the_full_ones() {
    let doc = run(&["commands", "--json"]);
    let (mut left_out, mut flags_back, mut enums_back) = (0, 0, 0);
    for (e, c) in entries().zip(doc["commands"].as_array().unwrap()) {
        let mut path = vec!["commands", "--json"];
        path.extend(e.name.split(' '));
        let whole = run(&path);
        let whole = whole["commands"]
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["path"] == e.name)
            .unwrap();
        let want = whole["args"].as_array().cloned().unwrap_or_default();
        let got: Vec<Value> = restored_args(c).iter().map(full_arg).collect();
        assert_eq!(got, want, "{}: args", e.name);
        if c.get("args").is_none() && !want.is_empty() {
            left_out += 1;
        }
        let want = whole["flags"].as_array().cloned().unwrap_or_default();
        let mut got = restored_flags(c, &doc);
        for (f, g) in got.iter_mut().zip(&want) {
            let name = f["name"].as_str().unwrap().to_string();
            let name = name.as_str();
            // what the other rules put back: the shared help and the type (`flag_help`, `flag_type`)
            if f.get("help").is_none() {
                f["help"] = doc["flag_help"][name].clone();
            }
            if f.get("type").is_none() {
                f["type"] = doc["flag_type"]
                    .get(name)
                    .cloned()
                    .unwrap_or(json!("string"));
            }
            assert_eq!(f, g, "{}: {name}", e.name);
            flags_back += usize::from(f.get("required").is_some() || f.get("repeat").is_some());
            enums_back += usize::from(f.get("enum").is_some());
        }
        assert_eq!(got.len(), want.len(), "{}: flags", e.name);
    }
    assert!(
        left_out > 50,
        "only {left_out} commands leave their args out"
    );
    assert!(
        flags_back > 30 && enums_back > 10,
        "{flags_back} {enums_back}"
    );
    // the compact listing really leaves the members out
    for c in doc["commands"].as_array().unwrap() {
        for f in c["flags"].as_array().into_iter().flatten() {
            assert!(
                f.get("required").is_none() && f.get("repeat").is_none(),
                "{c}"
            );
            assert!(f.get("enum").is_none(), "{c}");
        }
    }
}
