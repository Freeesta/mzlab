//! `report.methods@1` on a fixed `corsa/1` record: the Markdown is the golden file byte for byte, in both styles.

use super::{methods_text, placeholders, template, templated};
use mzlab_core::json::Value;
use std::path::Path;

fn golden(name: &str) -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/golden/feat")
            .join(name),
    )
    .unwrap()
}

fn style(s: &str) -> Vec<(String, Value)> {
    vec![("style".to_string(), Value::Str(s.to_string()))]
}

fn check(style_name: &str, file: &str) {
    let text = methods_text(&style(style_name), &golden("methods-corsa.json")).unwrap();
    if std::env::var("MZLAB_UPDATE_GOLDEN").is_ok() {
        std::fs::write(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/golden/feat")
                .join(file),
            &text,
        )
        .unwrap();
    }
    assert_eq!(
        text.as_bytes(),
        golden(file).as_slice(),
        "{style_name}\n{text}"
    );
}

#[test]
fn the_paragraph_is_the_golden_file() {
    check("paragraph", "methods-paragraph.md");
}

#[test]
fn the_list_is_the_golden_file() {
    check("list", "methods-list.md");
}

#[test]
fn the_default_style_is_the_paragraph() {
    let run = golden("methods-corsa.json");
    assert_eq!(
        methods_text(&[], &run).unwrap(),
        methods_text(&style("paragraph"), &run).unwrap()
    );
}

#[test]
fn a_text_that_is_not_a_run_record_is_refused() {
    assert!(methods_text(&[], b"{\"schema\":\"prov/1\"}").is_err());
    assert!(methods_text(&[], b"nonsense").is_err());
}

#[test]
fn the_algorithm_selects_the_sentence_and_a_step_without_template_gets_the_generic_one() {
    assert!(template("features.detect", "centwave@1")
        .unwrap()
        .contains("centWave"));
    assert!(template("features.detect", "masstrace")
        .unwrap()
        .contains("mass traces"));
    assert!(template("features.detect", "other").is_some());
    assert!(template("vendor.custom", "").is_none());
}

#[test]
fn every_placeholder_of_a_template_is_a_word_of_the_table() {
    for (step, alg) in templated() {
        let t = template(step, alg).unwrap();
        assert!(t.ends_with('.'), "{step}: {t}");
        for p in placeholders(t) {
            assert!(
                !p.is_empty()
                    && p.chars()
                        .all(|c| c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit()),
                "{step}: {p}"
            );
        }
    }
}
