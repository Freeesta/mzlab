//! The JSON of the transformation-product steps is English: the registry of `tp.generate` (`hypotheses/3`: outcomes `not_searched` / `not_found`,
//! alternatives always `FORMULA (PATH)`), the meta of the block `tp/2` and the network of mass differences. No key and no value holds an Italian word
//! of `tools/italian_words.tsv`.

use std::path::{Path, PathBuf};

use mzlab_feat::hypo::*;
use mzlab_feat::tp::{RoleCounts, TpBlock};
use serde_json::Value;

const WORDS: &str = include_str!("../../../tools/italian_words.tsv");
/// Italian words that are also English words, as in `tools/check_english.py`.
const AMBIGUOUS: &[&str] = &[
    "come", "dove", "male", "fine", "solo", "coda", "piano", "lotto", "prima", "zone", "vie",
    "unite", "verso", "porta", "legge", "pari", "specie", "genere", "marca", "pila", "seme",
    "parte", "posa", "velo", "cosa", "nota", "conti", "vista", "tempo", "area", "linea", "idea",
    "vario", "banda", "fila", "tasso", "ramo", "corso", "esca", "ancora", "gira", "largo", "lente",
    "modo", "stato", "tema", "tocca", "canale",
];

/// Every key and string value of `v` with an Italian word (`RT_attesa_min` gives `attesa`).
fn italian_in(v: &Value, path: &str, out: &mut Vec<String>) {
    let bad = |s: &str| {
        s.split(|c: char| !c.is_alphabetic()).any(|w| {
            let w = w.to_lowercase();
            w.len() >= 4
                && !AMBIGUOUS.contains(&w.as_str())
                && WORDS
                    .lines()
                    .any(|l| !l.starts_with('#') && l.split('\t').next() == Some(w.as_str()))
        })
    };
    match v {
        Value::String(s) if bad(s) => out.push(format!("{path} = {s:?}")),
        Value::Array(a) => a
            .iter()
            .for_each(|x| italian_in(x, &format!("{path}[]"), out)),
        Value::Object(m) => {
            for (k, x) in m {
                if bad(k) {
                    out.push(format!("{path}.{k} (key)"));
                }
                italian_in(x, &format!("{path}.{k}"), out);
            }
        }
        _ => {}
    }
}

fn check(json: &str, what: &str) -> Value {
    let v: Value = serde_json::from_str(json).unwrap();
    let mut out = Vec::new();
    italian_in(&v, what, &mut out);
    out.sort();
    out.dedup();
    assert!(out.is_empty(), "Italian in {what}:\n{}", out.join("\n"));
    v
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn registry(nominal: bool) -> Registry {
    let rules = Rules::parse(&std::fs::read(root().join("TP_Mine/data/regole_tp_3.json")).unwrap())
        .unwrap();
    let frag = ["C6H9N2O".to_string()];
    let inputs = Inputs {
        parent: "C12H17ClN2O3S",
        suspects: None,
        families: &[],
        imported: &[],
        fragments: &frag,
        groups: None,
    };
    let mut r = generate(
        &rules,
        &inputs,
        &HypoParams {
            nominal,
            ..HypoParams::default()
        },
    )
    .unwrap();
    let n = r.outcomes.len();
    for (i, o) in r.outcomes.iter_mut().enumerate() {
        *o = match i % 3 {
            0 => Outcome::NotSearched,
            1 => Outcome::NotFound,
            _ => Outcome::Found(n as u32),
        };
    }
    r
}

#[test]
fn the_registry_is_english() {
    for nominal in [false, true] {
        let v = check(&registry(nominal).to_json(), "registry");
        assert_eq!(v["format"], "hypotheses/3");
        let hyps = v["hypotheses"].as_array().unwrap();
        let outcomes: std::collections::BTreeSet<String> = hyps
            .iter()
            .map(|h| {
                h["outcome"]
                    .as_str()
                    .map_or("rows".to_string(), String::from)
            })
            .collect();
        assert_eq!(
            outcomes,
            ["not_found", "not_searched", "rows"]
                .map(String::from)
                .into()
        );
    }
}

/// An alternative is always `FORMULA (PATH)`, the same formula as the leader's included.
#[test]
fn every_alternative_names_its_formula() {
    let mut seen = 0;
    for nominal in [false, true] {
        for h in registry(nominal).hyps {
            for a in &h.alternatives {
                let (f, rest) = a.split_once(" (").unwrap_or_else(|| panic!("{a:?}"));
                assert!(
                    f.chars().all(|c| c.is_ascii_alphanumeric()) && rest.ends_with(')'),
                    "{a:?}"
                );
                seen += 1;
            }
        }
    }
    assert!(seen > 0, "no alternative to check");
}

#[test]
fn the_tp_meta_is_english() {
    let mut b = TpBlock::default();
    let v = check(&b.meta_json(), "tp");
    assert_eq!(v["schema"], "tp/2");
    b.roles = Some(RoleCounts {
        by_blank: 1,
        by_t0: 2,
        by_dark: 3,
        no_t0: false,
        dark_over_t0: Some(0.8),
        blank_kinds: vec!["solvent".into()],
    });
    let v = check(&b.meta_json(), "tp");
    assert_eq!(v["counts"]["dropped_by_t0"], 2);
    assert_eq!(v["roles"]["blank_kinds"][0], "solvent");
}

/// The network of the Dig has the parent node `parent` (it was an Italian id).
#[test]
fn the_saved_network_is_english() {
    let text = std::fs::read_to_string(
        root().join("crates/mzlab-feat/tests/data/mass_network_timolol.json"),
    )
    .unwrap();
    let v = check(&text, "network");
    assert_eq!(v["nodes"][0]["id"], "parent");
}
