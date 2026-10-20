//! The `step/1` descriptors of the identification chain (package `attributio`): `docs/schemi/steps-attributio-1.json` is their export and
//! `docs/schemi/records-1.json` the one of the shared records (`MZLAB_UPDATE_STEPS=1 cargo test -p mzlab-feat --test steps_attributio` rewrites both).
//! The defaults are compared with the engine structs in `steps_attributio_parity.rs` (features `attributio`, `untargeted`).

use mzlab_core::step::json::registry_json;
use mzlab_core::step::{DataType, Kind};
use mzlab_feat::steps::records::{records, records_json};
use mzlab_feat::steps::{attributio, registry};

const IDS: [&str; 9] = [
    "spectra.chimeric",
    "formula.ms1",
    "formula.ms2",
    "spectra.match",
    "spectra.insilico",
    "rt.predict",
    "annot.level",
    "annot.regulatory",
    "annot.hazard",
];

fn committed(name: &str, text: &str) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/schemi")
        .join(name);
    if std::env::var_os("MZLAB_UPDATE_STEPS").is_some() {
        std::fs::write(&path, text).unwrap();
    }
    let on_disk = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("docs/schemi/{name} (MZLAB_UPDATE_STEPS=1 writes it)"));
    assert_eq!(on_disk, text, "docs/schemi/{name} is stale: MZLAB_UPDATE_STEPS=1 cargo test -p mzlab-feat --test steps_attributio");
}

#[test]
fn registry_lists_the_nine_steps_in_the_order_of_the_chain() {
    let ids: Vec<_> = attributio().iter().map(|s| s.id).collect();
    assert_eq!(ids, IDS);
    let core: Vec<_> = registry().iter().map(|s| s.id).collect();
    assert!(
        IDS.iter().all(|i| !core.contains(i)),
        "the core registry stays as it is"
    );
}

#[test]
fn every_port_requires_high_resolution_and_the_package_is_attributio() {
    for s in attributio() {
        assert_eq!(s.package, "attributio", "{}", s.id);
        assert!(
            s.ins.iter().any(|p| p.requires.resolution == ["hr"]),
            "{} has no hr input",
            s.id
        );
        assert!(
            s.algorithms
                .iter()
                .any(|a| Some(a.id) == s.default_algorithm),
            "{} default algorithm",
            s.id
        );
    }
}

#[test]
fn candidates_and_annot_are_passed_between_the_steps() {
    let steps = attributio();
    let out = |ty: DataType| {
        steps
            .iter()
            .filter(|s| s.outs.iter().any(|p| p.ty == ty))
            .count()
    };
    assert!(out(DataType::Candidates) >= 3);
    assert!(out(DataType::Annot) >= 3);
    let rec: Vec<_> = records().iter().map(|r| r.data_type).collect();
    assert_eq!(rec, ["candidates", "annot"]);
}

#[test]
fn every_default_is_inside_its_own_limits() {
    for s in attributio() {
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
    let text = registry_json(&attributio());
    let v: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");
    assert_eq!(v.as_array().unwrap().len(), IDS.len());
    committed("steps-attributio-1.json", &text);
    let rec = records_json(&records());
    serde_json::from_str::<serde_json::Value>(&rec).expect("valid JSON");
    committed("records-1.json", &rec);
}
