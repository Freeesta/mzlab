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
        assert_eq!(c["stdout"]["schema"], e.schema);
    }
}

/// The flags of the help line, found without the parser of the catalog (every word that starts with `--`), are the flags of the catalog, and no others.
#[test]
fn the_flags_of_help_and_catalog_are_the_same() {
    let doc = json(&out(&["commands", "--json"]));
    for (e, c) in entries().zip(doc["commands"].as_array().unwrap()) {
        let usage = e.help.split("   ").next().unwrap();
        let in_help: BTreeSet<String> = usage
            .split(|ch: char| ch.is_whitespace() || "[]|".contains(ch))
            .filter(|w| w.starts_with("--"))
            .map(String::from)
            .collect();
        let in_catalog: BTreeSet<String> = c["flags"]
            .as_array()
            .unwrap()
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
    let from = c["flags"]
        .as_array()
        .unwrap()
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
        flags.extend(c["flags"].as_array().unwrap());
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

/// Every flag says what it does, its type agrees with the value the usage line shows, and every command has exit codes and an example.
#[test]
fn flags_are_described_and_typed_like_the_usage_line() {
    let doc = json(&out(&["commands", "--json"]));
    for (e, c) in entries().zip(doc["commands"].as_array().unwrap()) {
        let usage = e.help.split("   ").next().unwrap();
        for f in c["flags"].as_array().unwrap() {
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
            .all(|x| x.as_u64().unwrap() <= 6));
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
