//! The subagents as data: the prompt files are the exact ```text blocks of `docs/studi/assistente-dati/prompt/0N_*.md`, every role names an existing prompt
//! and a model of the price table, and the files of `docs/schemi/` are valid JSON of the right `$id`.

use std::path::PathBuf;

use mzlab_core::json::{parse, Value};
use mzlab_llm::assist::{builtin, prompt_digest, PROMPTS};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
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
    for (file, text) in PROMPTS {
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
    let roles = builtin().unwrap();
    let names: Vec<&str> = roles.iter().map(|r| r.role.as_str()).collect();
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
    for r in &roles {
        assert!(
            prices.contains(&format!("\"id\":\"{}\"", r.model)),
            "{} has no price",
            r.model
        );
        assert!(!r.prompt.is_empty() && r.max_tokens > 0 && r.max_tokens <= 8192);
        assert!(["router/1", "lettura/1", "param-scelte/1", "grafico/1"]
            .contains(&r.output_schema.as_str()));
        assert!(prompt_digest(&r.prompt_file).is_some_and(|d| d.len() == 64));
    }
    // a repeated role or an unknown prompt is refused
    let twice =
        mzlab_llm::assist::ROLES.replacen("\"role\":\"metadati\"", "\"role\":\"router\"", 1);
    assert!(mzlab_llm::assist::roles(&twice).is_err());
    let lost = mzlab_llm::assist::ROLES.replacen("03_metadati.md", "99_none.md", 1);
    assert!(mzlab_llm::assist::roles(&lost).is_err());
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
    assert_eq!(prompt_digest("02_router.md"), prompt_digest("02_router.md"));
    assert_ne!(
        prompt_digest("02_router.md"),
        prompt_digest("08_grafici.md")
    );
    assert_eq!(prompt_digest("00_none.md"), None);
}
