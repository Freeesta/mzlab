//! The subagents as data: the prompt files are the exact ```text blocks of `docs/studi/assistente-dati/prompt/0N_*.md`, every role names an existing prompt
//! and a model of the price table, and the files of `docs/schemi/` are valid JSON of the right `$id`.

use std::path::PathBuf;

use mzlab_core::json::{parse, Value};
use mzlab_llm::assist::{roles, with_prompts, Prompts, Role};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn data(file: &str) -> String {
    std::fs::read_to_string(root().join("crates/mzlab-llm/data/assist").join(file)).unwrap()
}

/// The prompt files the way the page hands them over.
fn files() -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> =
        std::fs::read_dir(root().join("crates/mzlab-llm/data/assist"))
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .filter(|n| n.ends_with(".md"))
            .map(|n| (n.clone(), data(&n)))
            .collect();
    v.sort();
    v
}

fn prompts() -> Prompts {
    let f = files();
    let refs: Vec<(&str, &str)> = f.iter().map(|(n, t)| (n.as_str(), t.as_str())).collect();
    with_prompts(&refs)
}

fn builtin() -> Vec<Role> {
    roles(&data("assist-ruoli-1.json")).unwrap()
}

/// The first ```text block of a study prompt, the way `docs/studi` writes it.
fn block(file: &str) -> String {
    let s = std::fs::read_to_string(root().join("docs/studi/assistente-dati/prompt").join(file))
        .unwrap();
    let start = s.find("```text\n").expect("a text block") + 8;
    let end = start + s[start..].find("\n```").expect("a closed block");
    s[start..end].to_string()
}

#[test]
fn the_prompt_files_are_the_blocks_of_the_study_byte_for_byte() {
    let studies = std::fs::read_dir(root().join("docs/studi/assistente-dati/prompt")).unwrap();
    let mut names: Vec<String> = studies
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    assert_eq!(files().len(), 8);
    for (file, text) in files() {
        let n = &file[..2];
        let study = names
            .iter()
            .find(|s| s.starts_with(&format!("{n}_")))
            .expect("a study file");
        assert_eq!(
            text,
            format!("{}\n", block(study)),
            "{file} differs from {study}"
        );
    }
}

#[test]
fn every_role_has_a_prompt_a_priced_model_and_a_known_schema() {
    let all = builtin();
    let book = prompts();
    book.check(&all).unwrap();
    let names: Vec<&str> = all.iter().map(|r| r.role.as_str()).collect();
    assert_eq!(
        names,
        [
            "router",
            "metadati",
            "cromatografia",
            "parametri",
            "interprete",
            "grafici"
        ]
    );
    let prices =
        std::fs::read_to_string(root().join("crates/mzlab-llm/data/fornitori-1.json")).unwrap();
    for r in &all {
        assert!(
            prices.contains(&format!("\"id\":\"{}\"", r.model)),
            "{} has no price",
            r.model
        );
        assert!(
            book.prompt(&r.prompt_file).is_some_and(|p| !p.is_empty())
                && r.max_tokens > 0
                && r.max_tokens <= 8192
        );
        assert!(["router/1", "lettura/1", "param-scelte/1", "grafico/1"]
            .contains(&r.output_schema.as_str()));
        assert!(book.digest(&r.prompt_file).is_some_and(|d| d.len() == 64));
    }
    // a repeated role or an unknown prompt is refused
    let json = data("assist-ruoli-1.json");
    let twice = json.replacen("\"role\":\"metadati\"", "\"role\":\"router\"", 1);
    assert!(roles(&twice).is_err());
    let lost = roles(&json.replacen("03_metadati.md", "99_none.md", 1)).unwrap();
    assert!(book.check(&lost).is_err());
    assert!(roles("{}").is_err());
}

#[test]
fn the_schemas_are_json_with_their_id() {
    for (file, id) in [
        ("lettura-1", "lettura/1"),
        ("param-scelte-1", "param-scelte/1"),
        ("assist-ruoli-1", "assist-ruoli/1"),
    ] {
        let s = std::fs::read_to_string(root().join(format!("docs/schemi/{file}.json"))).unwrap();
        let v = parse(s.as_bytes()).unwrap();
        assert_eq!(v.get("$id").and_then(Value::str), Some(id));
    }
}

#[test]
fn the_digest_of_a_prompt_is_stable() {
    let book = prompts();
    assert_eq!(book.digest("02_router.md"), book.digest("02_router.md"));
    assert_ne!(book.digest("02_router.md"), book.digest("08_grafici.md"));
    assert_eq!(book.digest("00_none.md"), None);
    // trailing newlines do not change what is recorded
    let a = with_prompts(&[("x.md", "abc\n")]);
    let b = with_prompts(&[("x.md", "abc")]);
    assert_eq!(a.digest("x.md"), b.digest("x.md"));
}
