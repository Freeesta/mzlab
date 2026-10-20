//! The `step/1` descriptors of the Tabui family (package `tabui`): `docs/schemi/steps-tabui-1.json` is their export
//! (`MZLAB_UPDATE_STEPS=1 cargo test -p mzlab-chem --test steps_tabui` rewrites it). The constants of the `interp` feature are compared with the
//! code in `crates/mzlab-feat/tests/steps_tabui_parity.rs`.

use mzlab_chem::step::tabui;
use mzlab_core::step::json::registry_json;
use mzlab_core::step::Kind;

const IDS: [&str; 5] = [
    "tp.generate",
    "tp.filter",
    "tp.link",
    "tp.position",
    "structure.reactivity",
];

#[test]
fn registry_lists_the_five_steps_in_the_order_of_the_chain() {
    let steps = tabui();
    assert_eq!(steps.iter().map(|s| s.id).collect::<Vec<_>>(), IDS);
    for s in &steps {
        assert_eq!(s.package, "tabui", "{}", s.id);
        assert!(
            s.algorithms
                .iter()
                .any(|a| Some(a.id) == s.default_algorithm),
            "{} default algorithm",
            s.id
        );
        assert!(
            s.runs_in.wasm.is_some() && s.runs_in.wasm_ceiling_bytes.is_some(),
            "{}",
            s.id
        );
    }
}

#[test]
fn every_default_is_inside_its_own_limits() {
    for s in tabui() {
        for p in s
            .params
            .iter()
            .chain(s.algorithms.iter().flat_map(|a| a.params.iter()))
        {
            let Some(d) = &p.default else { continue };
            if let (Some(x), Kind::Number | Kind::Integer) = (d.f64(), &p.kind) {
                assert!(
                    p.min.is_none_or(|m| x >= m),
                    "{} {} below its minimum",
                    s.id,
                    p.name
                );
                assert!(
                    p.max.is_none_or(|m| x <= m),
                    "{} {} above its maximum",
                    s.id,
                    p.name
                );
            }
            if let Kind::Choice(values) = &p.kind {
                assert!(
                    values.contains(d),
                    "{} {} default is not a choice",
                    s.id,
                    p.name
                );
            }
        }
    }
}

#[test]
fn exported_json_is_standard_json_and_is_the_committed_file() {
    let text = registry_json(&tabui());
    let v: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");
    assert_eq!(v.as_array().unwrap().len(), IDS.len());
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/schemi/steps-tabui-1.json");
    if std::env::var_os("MZLAB_UPDATE_STEPS").is_some() {
        std::fs::write(&path, &text).unwrap();
    }
    let on_disk = std::fs::read_to_string(&path)
        .expect("docs/schemi/steps-tabui-1.json (MZLAB_UPDATE_STEPS=1 writes it)");
    assert_eq!(on_disk, text, "docs/schemi/steps-tabui-1.json is stale: MZLAB_UPDATE_STEPS=1 cargo test -p mzlab-chem --test steps_tabui");
}

/// The reactivity defaults are the ones the engine applies to an empty option string.
#[test]
fn reactivity_defaults_are_those_of_the_engine() {
    let s = tabui().pop().unwrap();
    let d = mzlab_chem::reatt::Opts::parse("").unwrap();
    let num = |n: &str| s.param(n).unwrap().default_f64().unwrap();
    assert_eq!(num("conformers"), d.conformers as f64);
    assert_eq!(num("conformers"), mzlab_chem::DEFAULT_CONFORMERS as f64);
    assert_eq!(num("seed"), d.seed as f64);
}
