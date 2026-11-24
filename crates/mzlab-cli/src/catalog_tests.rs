//! The catalog `commands/1` and `schema` agree with the command table, the help, the schema files and the size budgets.

use std::collections::BTreeSet;

use crate::cmd::entries;
use crate::execute;

fn out(args: &[&str]) -> String {
    let o = execute(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>());
    assert_eq!(o.code, 0, "{args:?}: {}", o.stderr);
    String::from_utf8(o.stdout).unwrap()
}

fn json(text: &str) -> serde_json::Value {
    serde_json::from_str(text).unwrap()
}

#[test]
fn every_entry_is_in_the_catalog_with_its_usage_line() {
    let text = out(&["commands", "--json"]);
    assert!(text.ends_with('\n') && text.starts_with("{\"schema\":\"commands/1\""));
    let doc = json(&text);
    let cmds = doc["commands"].as_array().unwrap();
    assert_eq!(cmds.len(), entries().count());
    for (e, c) in entries().zip(cmds) {
        assert_eq!(c["path"], e.name);
        let want = e
            .help
            .split("   ")
            .next()
            .unwrap()
            .trim_start_matches("mzlab-cli ");
        assert_eq!(c["synopsis"], if e.help.is_empty() { e.name } else { want });
        assert_eq!(c["stdout"]["schema"].as_str().unwrap_or(""), e.schema);
    }
}

/// The flags of the help line, found without the parser of the catalog (every word that starts with `--`), are the flags of the catalog, and no others.
#[test]
fn the_flags_of_help_and_catalog_are_the_same() {
    let doc = json(&out(&["commands", "--json"]));
    for (e, c) in entries().zip(doc["commands"].as_array().unwrap()) {
        let usage = e.help.split("   ").next().unwrap();
        let in_help: BTreeSet<String> = usage
            .split(|ch: char| ch.is_whitespace() || "[]|()".contains(ch))
            .filter(|w| w.starts_with("--"))
            .map(String::from)
            .collect();
        let in_catalog: BTreeSet<String> = members(c, "flags")
            .iter()
            .map(|f| f["name"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(in_help, in_catalog, "{}", e.name);
    }
}

#[test]
fn flags_have_type_unit_and_default() {
    let doc = json(&out(&["commands", "--json", "peak", "auto"]));
    let c = &doc["commands"][0];
    let from = members(c, "flags")
        .iter()
        .find(|f| f["name"] == "--from")
        .unwrap();
    assert_eq!(
        (
            from["type"].as_str(),
            from["unit"].as_str(),
            from["required"].as_bool()
        ),
        (Some("number"), Some("min"), None)
    );
    let doc = json(&out(&["commands", "--json", "features"]));
    let preset = doc["commands"][0]["flags"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["name"] == "--preset")
        .unwrap();
    assert_eq!(
        (preset["type"].as_str(), preset["default"].as_str()),
        (Some("enum"), Some("auto"))
    );
    assert_eq!(preset["enum"], json("[\"auto\",\"pos\",\"neg\"]"));
    let doc = json(&out(&["commands", "--json", "mol", "cip"]));
    assert_eq!(doc["commands"][0]["stdin"], true);
}

#[test]
fn every_named_schema_is_listed_and_the_folder_is_embedded() {
    let list = json(&out(&["schema", "--list"]));
    let names: BTreeSet<&str> = list["schemas"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    for e in entries().filter(|e| !e.schema.is_empty()) {
        assert!(
            names.contains(e.schema),
            "{}: schema {} is not in schema --list",
            e.name,
            e.schema
        );
    }
    for n in [
        "mzflow/2",
        "samples/1",
        "design/1",
        "error/1",
        "prov/1",
        "commands/1",
        "schemas/1",
    ] {
        assert!(names.contains(n), "{n}");
        assert_eq!(json(&out(&["schema", n]))["$id"], n);
    }
    // every JSON Schema of docs/schemi is embedded (a new one is added to the list of schema_cmd.rs)
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/schemi");
    let mut files = 0;
    for f in std::fs::read_dir(dir).unwrap() {
        let p = f.unwrap().path();
        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&p).unwrap()).unwrap();
        if let Some(id) = v.get("$schema").and_then(|_| v["$id"].as_str()) {
            files += 1;
            assert!(names.contains(id), "{} is not embedded", p.display());
        }
    }
    assert_eq!(files, names.len());
}

#[test]
fn flag_objects_never_carry_null_or_false_members() {
    let doc = json(&out(&["commands", "--json"]));
    let mut flags: Vec<&serde_json::Value> =
        doc["global_flags"].as_array().unwrap().iter().collect();
    for c in doc["commands"].as_array().unwrap() {
        flags.extend(members(c, "flags"));
    }
    assert!(flags.len() > 20);
    for f in flags {
        for (k, v) in f.as_object().unwrap() {
            assert!(
                !v.is_null() && v != &serde_json::Value::Bool(false),
                "{f}: {k}"
            );
        }
    }
}

#[test]
fn unknown_names_are_selector_errors_and_sizes_stay_small() {
    for a in [&["schema", "nope/1"][..], &["commands", "nope"]] {
        let o = execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        assert_eq!(o.code, 5, "{a:?}");
    }
    let n = out(&["commands", "--json"]).len();
    assert!(n < 40_000, "commands --json is {n} bytes");
    let n = out(&["flow", "steps", "--compact"]).len();
    assert!(n < 20_000, "flow steps --compact is {n} bytes");
    let steps = json(&out(&["flow", "steps", "--compact"]));
    assert_eq!(steps["schema"], "steps-compact/1");
    assert_eq!(
        steps["steps"].as_array().unwrap().len(),
        crate::flow::steps().len()
    );
    let tools = json(&out(&["llm", "tools", "--compact"]));
    assert_eq!(tools["schema"], "tools-compact/1");
    assert!(!tools["tools"].as_array().unwrap().is_empty());
    assert_eq!(
        json(&out(&["llm", "tools"]))["cli_catalog"],
        "mzlab-cli commands --json"
    );
}

#[test]
fn the_text_form_lists_a_command_per_line() {
    let t = out(&["commands", "flow"]);
    assert!(t.lines().any(|l| l.starts_with("flow run  ")));
    assert!(t.lines().all(|l| l.starts_with("flow ")));
}

/// The command `e` in full (the listing of `commands --json NAME`, which the compact whole listing leaves some members out of).
fn full(e: &crate::cmd::Entry) -> serde_json::Value {
    let mut args = vec!["commands", "--json"];
    args.extend(e.name.split(' '));
    let doc = json(&out(&args));
    doc["commands"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["path"] == e.name)
        .unwrap()
        .clone()
}

/// Every flag says what it does, its type agrees with the value the usage line shows, and every command has exit codes and an example.
#[test]
fn flags_are_described_and_typed_like_the_usage_line() {
    for e in entries() {
        let c = &full(e);
        let usage = e.help.split("   ").next().unwrap();
        for f in members(c, "flags") {
            let name = f["name"].as_str().unwrap();
            assert!(
                !f["help"].as_str().unwrap().is_empty(),
                "{}: {name} has no help (FLAG_HELP)",
                e.name
            );
            // the word after the flag, if it is a value and not the next flag or a closing bracket
            let after = usage.split(name).nth(1).unwrap_or("");
            let value = after
                .split_whitespace()
                .next()
                .filter(|w| !w.starts_with(['-', '[', '|', ']']) && !w.starts_with("..."));
            match value {
                None => assert_eq!(f["type"], "bool", "{}: {name}", e.name),
                Some(v) => {
                    assert_ne!(f["type"], "bool", "{}: {name} takes {v}", e.name);
                    if f["type"] == "enum" {
                        let alts: Vec<String> = v
                            .trim_end_matches(']')
                            .split('|')
                            .map(String::from)
                            .collect();
                        assert_eq!(f["enum"], serde_json::json!(alts), "{}: {name}", e.name);
                    }
                }
            }
        }
        assert!(
            !c["exit_codes"].as_array().unwrap().is_empty(),
            "{}",
            e.name
        );
        assert!(!c["examples"].as_array().unwrap().is_empty(), "{}", e.name);
        assert!(c["exit_codes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|x| x.as_u64().unwrap() <= 8));
    }
}

/// Every JSON document the catalog commands print has `schema` first and ends with a newline; a raw schema is the file itself.
#[test]
fn the_documents_have_schema_first_and_a_newline() {
    for (a, schema) in [
        (&["commands", "--json"][..], "commands/1"),
        (&["schema", "--list"], "schemas/1"),
        (&["flow", "steps", "--compact"], "steps-compact/1"),
        (&["llm", "tools", "--compact"], "tools-compact/1"),
        (&["llm", "tools"], "tools/1"),
    ] {
        let t = out(a);
        assert!(t.ends_with('\n'), "{a:?}");
        let compact: String = t.chars().filter(|c| !c.is_whitespace()).take(40).collect();
        assert!(
            compact.starts_with(&format!("{{\"schema\":\"{schema}\"")),
            "{a:?}: {compact}"
        );
    }
    assert!(out(&["schema", "error/1"]).ends_with('\n'));
}

/// The `args` or `flags` of a command of the listing: the whole listing leaves out an empty list.
fn members<'a>(c: &'a serde_json::Value, key: &str) -> &'a Vec<serde_json::Value> {
    static NONE: Vec<serde_json::Value> = Vec::new();
    c[key].as_array().unwrap_or(&NONE)
}

/// The exit codes `exit_codes_note` states, from the compact listing of command `c` alone (written out again here, not taken from `exits_of`).
fn derived_exits(name: &str, c: &serde_json::Value, types: &serde_json::Value) -> Vec<u8> {
    let flags = members(c, "flags");
    let has = |f: &str| flags.iter().any(|x| x["name"] == f);
    // an arg with no kind is a path (or `...`, a rest): both read a file; a flag with no type takes its `flag_type`
    let reads = members(c, "args")
        .iter()
        .any(|a| a.get("kind").is_none_or(|k| k == "path" || k == "rest"))
        || flags.iter().any(|f| {
            let t = f
                .get("type")
                .or_else(|| types.get(f["name"].as_str().unwrap()));
            t == Some(&serde_json::json!("path")) && f["name"] != "--out"
        });
    let mut v = vec![0, 1, 2];
    if name.starts_with("flow ") || crate::contract::VERDICT.contains(&name) {
        v.push(3);
    }
    if reads {
        v.push(4);
    }
    if has("--sample")
        || has("--id")
        || name.starts_with("sample ")
        || name.starts_with("dad ")
        || name.starts_with("msn graph")
    {
        v.push(5);
    }
    if has("--out") {
        v.push(6);
    }
    v
}

/// The words of a usage line outside its `[...]` groups, without the `...` tokens: what `examples_note` says an example is made of.
fn top_level_words(synopsis: &str) -> String {
    let (mut depth, mut cur, mut words) = (0, String::new(), Vec::new());
    for ch in synopsis.chars().chain(std::iter::once(' ')) {
        match ch {
            // a `[` that opens a word starts an optional group; one inside a word (`bonds=A-B[,...]`) stays in it
            '[' if depth > 0 || cur.is_empty() => depth += 1,
            ']' if depth > 0 => depth -= 1,
            ' ' if depth == 0 => {
                if !cur.is_empty() && cur != "..." {
                    words.push(std::mem::take(&mut cur));
                }
                cur.clear();
            }
            _ if depth == 0 => cur.push(ch),
            _ => {}
        }
    }
    words.join(" ")
}

/// An arg with no kind is a path (`...` a rest), a flag with no type has its `flag_type` or is a string: the types of the full listing.
fn defaults_give_the_full_types(
    doc: &serde_json::Value,
    c: &serde_json::Value,
    whole: &serde_json::Value,
    name: &str,
) {
    for (a, g) in members(c, "args")
        .iter()
        .zip(whole["args"].as_array().unwrap())
    {
        let kind = match a.get("kind") {
            Some(k) => k.clone(),
            None if a["name"] == "..." => serde_json::json!("rest"),
            None => serde_json::json!("path"),
        };
        assert_eq!(kind, g["kind"], "{name}: {}", a["name"]);
    }
    for (f, g) in members(c, "flags")
        .iter()
        .zip(whole["flags"].as_array().unwrap())
    {
        let flag = f["name"].as_str().unwrap();
        let kind = f
            .get("type")
            .or_else(|| doc["flag_type"].get(flag))
            .cloned()
            .unwrap_or_else(|| serde_json::json!("string"));
        assert_eq!(kind, g["type"], "{name}: {flag}");
    }
}

/// Every argument is `optional` in the whole listing or `required` in the path listing, never both and never neither.
#[test]
fn compact_optional_is_the_full_listings_missing_required_for_every_argument() {
    let doc = json(&out(&["commands", "--json"]));
    let mut n = 0;
    for (e, c) in entries().zip(doc["commands"].as_array().unwrap()) {
        let whole = full(e);
        let (short, long) = (members(c, "args"), members(&whole, "args"));
        assert_eq!(short.len(), long.len(), "{}", e.name);
        for (a, b) in short.iter().zip(long) {
            let (opt, req) = (a.get("optional").is_some(), b.get("required").is_some());
            assert!(opt != req, "{}: {}", e.name, a["name"]);
            assert!(a.get("required").is_none() && b.get("optional").is_none());
            n += 1;
        }
    }
    assert!(n > 50, "only {n} arguments compared");
}

/// The whole listing says what is shared or derivable once, and gives each command in full under its path: nothing is lost, only moved.
#[test]
fn the_whole_listing_is_compact_and_the_path_listing_is_full() {
    let doc = json(&out(&["commands", "--json"]));
    assert_eq!(doc["compact"], true);
    let shared = doc["flag_help"].as_object().unwrap();
    assert!(shared.len() > 5 && shared.contains_key("--out"));
    assert_eq!(doc["flag_type"]["--out"], "path");
    for (e, c) in entries().zip(doc["commands"].as_array().unwrap()) {
        let whole = full(e);
        assert!(whole.get("compact").is_none());
        defaults_give_the_full_types(&doc, c, &whole, e.name);
        for (f, g) in members(c, "flags")
            .iter()
            .zip(whole["flags"].as_array().unwrap())
        {
            let name = f["name"].as_str().unwrap();
            // a flag with no help here takes the shared one, and it is the help of the full listing
            let help = f.get("help").or_else(|| shared.get(name)).unwrap();
            assert_eq!(help, &g["help"], "{}: {name}", e.name);
        }
        // a member left out is the derived one: the rule of `exit_codes_note`, applied here to what the compact listing shows
        let derived = derived_exits(e.name, c, &doc["flag_type"]);
        if c.get("exit_codes").is_none() {
            assert_eq!(
                whole["exit_codes"],
                serde_json::json!(derived),
                "{}",
                e.name
            );
        } else {
            // a command that lists its codes lists others than the derived ones (the rest leave the member out)
            assert_ne!(c["exit_codes"], serde_json::json!(derived), "{}", e.name);
            assert_eq!(c["exit_codes"], whole["exit_codes"]);
        }
        if c.get("examples").is_none() {
            let want = format!(
                "mzlab-cli {}",
                top_level_words(c["synopsis"].as_str().unwrap())
            );
            assert_eq!(whole["examples"], serde_json::json!([want]), "{}", e.name);
        }
    }
    assert!(
        doc["commands"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c.get("exit_codes").is_some()),
        "some command lists its own codes"
    );
    let n = out(&["commands", "--json"]).len();
    assert!(n < 33_000, "the compact listing is {n} bytes");
}

/// The global `--cache` is the index cache; `db normative build` keeps its own `--cache` (sources folder), and no other command names the flag.
#[test]
fn the_global_cache_flag_is_not_the_normative_one() {
    let doc = crate::execute(&["commands".to_string(), "--json".to_string()]);
    let v: serde_json::Value = serde_json::from_slice(&doc.stdout).unwrap();
    let g = v["global_flags"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["name"] == "--cache")
        .expect("global --cache");
    assert!(g["help"].as_str().unwrap().contains("index cache"), "{g}");
    let n = full(entries().find(|e| e.name == "db normative build").unwrap());
    let own = n["flags"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["name"] == "--cache")
        .expect("normative --cache");
    assert_eq!(own["help"], "sources folder");
    for e in entries().filter(|e| e.name != "db normative build") {
        assert!(
            !e.help.contains("--cache"),
            "{}: synopsis names --cache",
            e.name
        );
    }
}

/// The compact listing leaves out `stdout.format` only when it is json, or text with a `json_flag` (and `stdout` when nothing else is left); the full entry always has it.
#[test]
fn a_json_stdout_is_said_once_in_the_compact_listing() {
    let doc = json(&out(&["commands", "--json"]));
    assert!(doc["stdout_note"]
        .as_str()
        .is_some_and(|n| n.contains("json")));
    for (e, c) in entries().zip(doc["commands"].as_array().unwrap()) {
        let words: Vec<&str> = e.name.split(' ').collect();
        let mut args = vec!["commands", "--json"];
        args.extend(&words);
        let full = json(&out(&args));
        let whole = &full["commands"][0]["stdout"];
        assert!(whole["format"].is_string(), "{}", e.name);
        let json_out = whole["format"] == "json";
        let text_json_flag = whole["format"] == "text" && whole["json_flag"] == true;
        assert_eq!(
            c["stdout"].get("format").is_none(),
            json_out || text_json_flag,
            "{}",
            e.name
        );
        assert_eq!(
            c.get("stdout").is_none(),
            json_out && whole.get("schema").is_none(),
            "{}",
            e.name
        );
    }
}
