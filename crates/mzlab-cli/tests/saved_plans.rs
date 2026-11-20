//! Every `mzflow/2` plan kept in the repository (fixtures, schemas, docs examples, saved-format samples, the LLM data) still goes through the strict reader
//! (`mzlab_core::flow::parse`, which refuses unknown keys and values of the wrong type): a plan an exporter ever wrote must not become unreadable.
//! The plans written inline in the Rust tests are read by those tests themselves; the page's own writers (`F.newPlan`, the canonical export) are covered by
//! `tests/test_flow_grafico.py`.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn tracked() -> Vec<String> {
    let out = std::process::Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(root())
        .output()
        .expect("git ls-files");
    String::from_utf8(out.stdout)
        .unwrap()
        .split('\0')
        .filter(|f| !f.is_empty() && !f.starts_with("mzlab/web/vendor/"))
        .map(String::from)
        .collect()
}

/// Every object of `v` that is a plan (`"format": "mzflow/2"`), and every string that holds one as JSON text.
fn plans(v: &serde_json::Value, out: &mut Vec<String>) {
    match v {
        serde_json::Value::Object(m) => {
            if m.get("format").and_then(|f| f.as_str()) == Some("mzflow/2") {
                out.push(v.to_string());
            } else {
                m.values().for_each(|x| plans(x, out));
            }
        }
        serde_json::Value::Array(a) => a.iter().for_each(|x| plans(x, out)),
        serde_json::Value::String(s)
            if s.contains("mzflow/2") && s.trim_start().starts_with('{') =>
        {
            if let Ok(x) = serde_json::from_str(s) {
                plans(&x, out);
            }
        }
        _ => {}
    }
}

/// The `{...}` objects of free text (a document) that name `mzflow/2`, found by balanced braces outside strings.
fn objects(text: &str) -> Vec<&str> {
    let b = text.as_bytes();
    let mut found = Vec::new();
    let mut at = 0;
    while let Some(i) = text[at..].find("\"mzflow/2\"") {
        let pos = at + i;
        at = pos + 10;
        let mut depth = 0i32;
        let mut start = None;
        for k in (0..pos).rev() {
            match b[k] {
                b'}' => depth += 1,
                b'{' if depth == 0 => {
                    start = Some(k);
                    break;
                }
                b'{' => depth -= 1,
                _ => {}
            }
        }
        let Some(s) = start else { continue };
        let (mut d, mut in_str, mut esc) = (0i32, false, false);
        for (k, c) in b[s..].iter().enumerate() {
            match (in_str, *c) {
                (true, _) if esc => esc = false,
                (true, b'\\') => esc = true,
                (true, b'"') => in_str = false,
                (true, _) => {}
                (false, b'"') => in_str = true,
                (false, b'{') => d += 1,
                (false, b'}') => {
                    d -= 1;
                    if d == 0 {
                        found.push(&text[s..s + k + 1]);
                        at = at.max(s + k + 1);
                        break;
                    }
                }
                _ => {}
            }
        }
    }
    found
}

#[test]
fn every_saved_plan_of_the_repository_still_reads() {
    let (mut read, mut files, mut bad) = (0, 0, Vec::new());
    for f in tracked() {
        let ext = f.rsplit('.').next().unwrap_or("");
        if !["json", "jsonl", "md"].contains(&ext) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(root().join(&f)) else {
            continue;
        };
        // the cases of the checker hold plans that are refused on purpose, with the answer expected; `flow_cases.rs` reads them
        if !text.contains("mzflow/2") || f.ends_with("flow_check_cases.json") {
            continue;
        }
        let mut found: Vec<String> = Vec::new();
        if ext == "json" {
            if let Ok(v) = serde_json::from_str(&text) {
                plans(&v, &mut found);
            }
        } else if ext == "jsonl" {
            for line in text.lines().filter_map(|l| serde_json::from_str(l).ok()) {
                plans(&line, &mut found);
            }
        }
        if found.is_empty() {
            // a document (markdown) or a JSON file that holds plans only as text: the objects that parse as JSON
            for o in objects(&text) {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(o) {
                    plans(&v, &mut found);
                }
            }
        }
        if found.is_empty() {
            continue;
        }
        files += 1;
        for p in found {
            read += 1;
            if let Err(e) = mzlab_core::flow::parse(p.as_bytes()) {
                bad.push(format!("{f}: {} {:?}", e.key, e.params));
            }
        }
    }
    println!("{read} plans in {files} files read");
    assert!(
        bad.is_empty(),
        "plans that no longer read:\n{}",
        bad.join("\n")
    );
    assert!(read >= 20, "only {read} plans found: the walk lost its way");
}
