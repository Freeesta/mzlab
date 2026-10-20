//! `disegno/1`: the proposal from `campioni/2`, canonical bytes for any order of the files, the findings of `valida` on five cases and the schema.

#![allow(clippy::disallowed_methods)] // tests compare against the standard sorts

use mzlab_core::campioni::{proponi, Campione};
use mzlab_core::disegno::{self, digest, from_json, to_json, valida, Confronto, Disegno};
use serde_json::Value;

/// Files with a name, a SHA-256 made from the position and the roles `proponi` guesses.
fn series(names: &[&str]) -> Vec<Campione> {
    let v: Vec<(String, String, String)> = names
        .iter()
        .enumerate()
        .map(|(i, n)| (n.to_string(), format!("{i:02x}"), String::new()))
        .collect();
    let mut cs = proponi(&v);
    for c in &mut cs {
        c.time_confirmed = true;
    }
    cs
}

fn codes(d: &Disegno, cs: &[Campione]) -> Vec<(String, String)> {
    valida(d, cs)
        .into_iter()
        .map(|a| (a.code.to_string(), a.series))
        .collect()
}

fn code_list(v: &[(&str, &str)]) -> Vec<(String, String)> {
    v.iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect()
}

const FULL: [&str; 12] = [
    "QC_1",
    "ref_t005_a",
    "ref_t005_b",
    "ref_t005_c",
    "gas_t005_a",
    "gas_t005_b",
    "gas_t005_c",
    "QC_2",
    "ref_t030_a",
    "ref_t030_b",
    "ref_t030_c",
    "QC_3",
];

#[test]
fn proposal_from_the_names() {
    let cs = series(&[
        "ref_t000",
        "ref_t005_a",
        "ref_t005_b",
        "gas_t005_a",
        "gas_t005_b",
        "QC_1",
    ]);
    let d = disegno::proponi(&cs);
    let names: Vec<&str> = d.conditions.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["ref", "gas"]);
    assert_eq!(d.reference.as_deref(), Some("ref"));
    assert_eq!(d.comparisons, [Confronto::VsReference]);
    assert_eq!(d.conditions[0].replicates, 2);
    assert!(d.batches.iter().all(|l| l.batch == "1") && d.batches.len() == cs.len());
    // one series without a name: one condition called ""
    let one = disegno::proponi(&series(&["a_t5", "a_t10"]));
    assert_eq!((one.conditions.len(), one.reference.is_none()), (1, true));
}

#[test]
fn canonical_for_every_permutation_of_the_files() {
    let cs = series(&FULL);
    let want = to_json(&disegno::proponi(&cs));
    let sha = digest(&disegno::proponi(&cs));
    assert_eq!(sha.len(), 64);
    for k in 1..cs.len() {
        let mut p = cs.clone();
        p.rotate_left(k);
        assert_eq!(to_json(&disegno::proponi(&p)), want, "rotation {k}");
        p.reverse();
        assert_eq!(to_json(&disegno::proponi(&p)), want, "reversed {k}");
    }
    let back = from_json(want.as_bytes()).unwrap();
    assert_eq!(to_json(&back), want);
    assert_eq!(disegno::route("canonico", &want).unwrap(), want);
    assert_eq!(disegno::route("digest", &want).unwrap(), sha);
    assert_eq!(
        disegno::route("nope", "{}").unwrap_err().key,
        "error.badDisegno"
    );
}

#[test]
fn five_cases_of_findings() {
    // 1: complete, nothing to say
    let cs = series(&FULL);
    let d = disegno::proponi(&cs);
    assert_eq!(codes(&d, &cs), []);
    // 2: two replicates
    let cs2 = series(&[
        "QC_1",
        "ref_t005_a",
        "ref_t005_b",
        "gas_t005_a",
        "gas_t005_b",
        "gas_t005_c",
        "QC_2",
    ]);
    assert_eq!(
        codes(&disegno::proponi(&cs2), &cs2),
        code_list(&[("fewReplicates", "ref")])
    );
    // 3: no QC at the end of the batch
    let cs3 = series(&[
        "QC_1",
        "ref_t005_a",
        "ref_t005_b",
        "ref_t005_c",
        "gas_t005_a",
        "gas_t005_b",
        "gas_t005_c",
    ]);
    assert_eq!(
        codes(&disegno::proponi(&cs3), &cs3),
        code_list(&[("batchNoQc", "1")])
    );
    // 4: a second batch without QC
    let mut d4 = disegno::proponi(&cs);
    for l in d4.batches.iter_mut().filter(|l| l.file.starts_with("gas")) {
        l.batch = "2".into();
    }
    let mut d4 = from_json(to_json(&d4).as_bytes()).unwrap();
    d4.comparisons.clear();
    assert_eq!(codes(&d4, &cs), code_list(&[("batchNoQc", "2")]));
    // 5: comparisons with an empty condition and without reference
    let mut d5 = disegno::proponi(&cs);
    d5.comparisons = vec![Confronto::Pair("ref".into(), "ghost".into())];
    assert_eq!(codes(&d5, &cs), code_list(&[("emptyComparison", "ghost")]));
    d5.comparisons = vec![Confronto::VsReference];
    d5.reference = None;
    assert_eq!(codes(&d5, &cs), code_list(&[("refMissing", "")]));
    d5.reference = Some("ghost".into());
    assert_eq!(codes(&d5, &cs), code_list(&[("emptyComparison", "ghost")]));
}

#[test]
fn route_valida_and_schema() {
    let cs = series(&FULL);
    let d = disegno::proponi(&cs);
    let camp = mzlab_core::campioni::to_json(&cs);
    let j = to_json(&d);
    let out = disegno::route(
        "valida",
        &format!("{{\"campioni\":{camp},\"disegno\":{j}}}"),
    )
    .unwrap();
    assert_eq!(out, "{\"avvisi\":[],\"bloccante\":false}");
    let schema: Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/schemi/disegno-1.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let mut doc: Value = serde_json::from_str(&j).unwrap();
    doc["fattori"] = serde_json::json!([{"nome": "gas", "valori": ["air", "N2"]}]);
    doc["condizioni"][1]["livelli"] = serde_json::json!({"gas": "N2"});
    assert!(fits(&doc, &schema));
    let again = from_json(doc.to_string().as_bytes()).unwrap();
    assert_eq!(again.factors[0].values, ["air", "N2"]);
    assert_eq!(
        again.conditions[1].levels,
        [("gas".to_string(), "N2".to_string())]
    );
    let mut bad = doc.clone();
    bad["confronti"][0]["tipo"] = "all".into();
    assert!(!fits(&bad, &schema));
    assert!(from_json(bad.to_string().as_bytes()).is_err());
    let mut bad = doc.clone();
    bad["schema"] = "disegno/2".into();
    assert!(from_json(bad.to_string().as_bytes()).is_err());
}

/// The few keywords of JSON Schema that `docs/schemi/disegno-1.json` uses.
fn fits(v: &Value, s: &Value) -> bool {
    let ty = |t: &str| match t {
        "object" => v.is_object(),
        "array" => v.is_array(),
        "string" => v.is_string(),
        "number" => v.is_number(),
        "integer" => v.is_i64() || v.is_u64(),
        "boolean" => v.is_boolean(),
        "null" => v.is_null(),
        _ => false,
    };
    let checks = [
        s.get("type").is_none_or(|t| {
            t.as_str().map_or_else(
                || {
                    t.as_array()
                        .unwrap()
                        .iter()
                        .any(|x| ty(x.as_str().unwrap()))
                },
                ty,
            )
        }),
        s.get("const").is_none_or(|c| c == v),
        s.get("enum")
            .is_none_or(|e| e.as_array().unwrap().contains(v)),
        s.get("minimum")
            .is_none_or(|m| v.as_f64().unwrap() >= m.as_f64().unwrap()),
        s.get("required").is_none_or(|r| {
            r.as_array()
                .unwrap()
                .iter()
                .all(|k| v.get(k.as_str().unwrap()).is_some())
        }),
        s.get("properties").is_none_or(|p| {
            p.as_object()
                .unwrap()
                .iter()
                .all(|(k, ps)| v.get(k).is_none_or(|x| fits(x, ps)))
        }),
        s.get("items")
            .is_none_or(|i| v.as_array().unwrap().iter().all(|x| fits(x, i))),
    ];
    checks.iter().all(|c| *c)
}
