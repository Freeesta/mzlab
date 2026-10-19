#![cfg(feature = "attributio")]
//! Parity of the RG-xx data layer (GHS/CLP categories, snapshot readers) with the golden data of the reference tool (`tools/attributio_riferimento.py`,
//! stages `tox-ghs` and `tox-istantanee`) on the invented snapshots in `tests/data/attributio/istantanee_sintetiche/`.

#[macro_use]
mod attributio_comune;
use attributio_comune::*;
use mzlab_feat::attributio::tox::ghs;
use mzlab_feat::attributio::tox::snapshot::*;
use mzlab_feat::attributio::tox::vocab::canon;
use serde_json::{json, Value};

fn f(v: &Value) -> Option<f64> {
    v.as_f64()
}
fn taxa(v: &Value) -> Vec<(&str, f64)> {
    v.as_object()
        .unwrap()
        .iter()
        .map(|(k, x)| (k.as_str(), x.as_f64().unwrap()))
        .collect()
}
fn opt(s: Option<&str>) -> Value {
    s.map_or(Value::Null, |x| json!(x))
}

#[test]
fn ghs_matches_the_golden_data() {
    let g = golden("tox", "ghs.json");
    for r in g["acute"].as_array().unwrap() {
        assert_eq!(
            opt(ghs::acute_from_value(f(&r[0]))),
            r[1],
            "acute {:?}",
            r[0]
        );
    }
    for r in g["noec"].as_array().unwrap() {
        assert_eq!(
            opt(ghs::chronic_from_noec(f(&r[0]), r[1].as_bool().unwrap())),
            r[2],
            "noec {r:?}"
        );
    }
    for r in g["from_acute"].as_array().unwrap() {
        let got = ghs::chronic_from_acute(f(&r[0]), r[1].as_bool().unwrap(), f(&r[2]), f(&r[3]));
        assert_eq!(opt(got), r[4], "from_acute {r:?}");
    }
    for r in g["classify"].as_array().unwrap() {
        let c = ghs::classify(
            &taxa(&r[0]),
            &taxa(&r[1]),
            r[2].as_bool().unwrap(),
            Some(4.2),
            None,
        );
        let got = json!({"acute": c.acute, "chronic": c.chronic, "chronic_route": c.chronic_route});
        assert_eq!(got, r[3], "classify {r:?}");
    }
    for r in g["h_codes"].as_array().unwrap() {
        let codes: Vec<&str> = r[0]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c.as_str().unwrap())
            .collect();
        let (a, c) = ghs::from_h_codes(&codes);
        assert_eq!(json!({"acute": a, "chronic": c}), r[1], "h_codes {codes:?}");
    }
}

fn read(name: &str) -> String {
    std::fs::read_to_string(
        root()
            .join("tests/data/attributio/istantanee_sintetiche")
            .join(name),
    )
    .expect("snapshot")
}
fn num(v: Option<f64>) -> Value {
    v.map_or(Value::Null, |x| json!(x))
}

#[test]
fn snapshots_match_the_golden_data() {
    let g = golden("tox", "istantanee.json");
    let reg = read_registry(&read("compound_registry.csv")).unwrap();
    let want = g["registry"].as_array().unwrap();
    assert_eq!(reg.len(), want.len());
    for (r, w) in reg.iter().zip(want) {
        let got = json!({
            "inchikey": r.inchikey, "inchikey_first_block": r.first_block, "canonical_name": r.canonical_name, "formula": r.formula,
            "monoisotopic_mass": num(r.monoisotopic_mass), "xlogp": num(r.xlogp), "best_level": num(r.best_level),
            "pcl_annotation_count": num(r.pcl_annotation_count), "cas": r.cas, "dtxsid": r.dtxsid, "ghs_aquatic": r.ghs_aquatic,
            "norman_pred_acute": r.norman_pred_acute, "ik_basis": r.ik_basis, "suspect_lists": r.suspect_lists,
        });
        for (k, v) in got.as_object().unwrap() {
            same(v, &w[k], &format!("registry {} {k}", r.first_block));
        }
    }
    let mem = read_membership(&read("reglist_membership.csv")).unwrap();
    let want = g["membership"].as_array().unwrap();
    assert_eq!(mem.len(), want.len());
    for (r, w) in mem.iter().zip(want) {
        let got = json!({
            "inchikey_skeleton": r.first_block, "list_id": r.list_id, "is_tp": r.is_tp, "lc_esi_amenable": r.lc_esi_amenable,
            "resolved_by": r.resolved_by, "list_version": r.list_version, "xlogp": r.xlogp, "cas": r.cas, "list_group": r.list_group,
        });
        for (k, v) in got.as_object().unwrap() {
            assert_eq!(v, &w[k], "membership {} {k}", r.first_block);
        }
    }
    let vals = read_ecotox_values(&read("ecotox_values.csv")).unwrap();
    let want = g["values"].as_array().unwrap();
    assert_eq!(vals.len(), want.len());
    for (r, w) in vals.iter().zip(want) {
        same(&num(r.value_mg_l), &w["value_mg_L"], "value");
        assert_eq!(
            (json!(r.cas), json!(r.taxon), json!(r.op)),
            (w["cas"].clone(), w["taxon"].clone(), w["op"].clone())
        );
    }
    let eco = read_ecosar(&read("ecosar_endpoints.csv")).unwrap();
    let want = g["ecosar"].as_array().unwrap();
    assert_eq!(eco.len(), want.len());
    for (r, w) in eco.iter().zip(want) {
        same(&num(r.value_mg_l), &w["value_mg_L"], "ecosar value");
        same(&num(r.logkow), &w["logkow"], "logkow");
        assert_eq!(
            (
                json!(&r.key),
                json!(r.in_domain),
                json!(&r.excluded_because)
            ),
            (
                w["key"].clone(),
                w["in_domain"].clone(),
                w["excluded_because"].clone()
            )
        );
    }
}

#[test]
fn italian_vocabulary_reads_as_english() {
    assert_eq!(canon("vero"), "true");
    assert_eq!(canon("dubbio | falso"), "doubtful | false");
    assert_eq!(canon("anything else"), "anything else");
}

// ------------------------------------------------------------------------------------------------------------------ registry rules (RG-01)
use mzlab_feat::attributio::tox::registry::*;

fn parse(v: &Value) -> Option<f64> {
    v.as_str()
        .and_then(|s| s.trim().parse::<f64>().ok())
        .filter(|x| !x.is_nan())
}

#[test]
fn registry_rules_match_the_golden_data() {
    let g = golden("tox", "registro.json");
    let props: Vec<(i64, String)> = g["props"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| (k.parse().unwrap(), v.as_str().unwrap().to_string()))
        .collect();
    for r in g["representative"].as_array().unwrap() {
        let got = representative_cid(r[0].as_str().unwrap(), &props, r[1].as_str());
        assert_eq!(
            got.map_or(Value::Null, |c| json!(c)),
            r[2],
            "representative {r:?}"
        );
    }
    for r in g["synonyms"].as_array().unwrap() {
        let syn: Vec<&str> = r[0]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().unwrap())
            .collect();
        assert_eq!(json!(dtxsid_from_synonyms(&syn)), r[1], "dtxsid {syn:?}");
        assert_eq!(json!(cas_from_synonyms(&syn)), r[2], "cas {syn:?}");
    }
    for r in g["h_codes"].as_array().unwrap() {
        let t: Vec<&str> = r[0]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().unwrap())
            .collect();
        assert_eq!(json!(h_aquatic_codes(&t)), r[1], "h codes {t:?}");
    }
    for r in g["categories"].as_array().unwrap() {
        let mut counts = [None; 10];
        for (i, v) in r[0].as_array().unwrap().iter().enumerate() {
            counts[i] = v.as_f64().or_else(|| parse(v));
        }
        assert_eq!(json!(pcl_categories(&counts)), r[1], "categories {r:?}");
    }
    for r in g["predicted"].as_array().unwrap() {
        let c = r[0].as_array().unwrap();
        let (v, w) = predicted_lowest_lc50_mg_l(&[parse(&c[0]), parse(&c[1]), parse(&c[2])]);
        same(&v.map_or(Value::Null, |x| json!(x)), &r[1], "lc50");
        assert_eq!(json!(w), r[2]);
        assert_eq!(json!(ghs_acute_category(v)), r[3]);
    }
}

// ------------------------------------------------------------------------------------------------------------------ hazard records (RG-02)
use mzlab_feat::attributio::tox::hazard::{coverage, record, HazardData};

fn hazard_data() -> HazardData {
    HazardData {
        registry: read_registry(&read("compound_registry.csv")).unwrap(),
        members: read_identity_members(&read("registry_members.csv")).unwrap(),
        curated: read_curated(&read("ecotox_curated.csv")).unwrap(),
        overrides: read_overrides(&read("ecotox_structures.csv")).unwrap(),
        values: read_ecotox_values(&read("ecotox_values.csv")).unwrap(),
        ecosar: read_ecosar(&read("ecosar_endpoints.csv")).unwrap(),
    }
}

#[test]
fn hazard_records_match_the_golden_data() {
    let (d, g) = (hazard_data(), golden("tox", "pericolo.json"));
    let (cases, want) = (
        g["cases"].as_array().unwrap(),
        g["records"].as_array().unwrap(),
    );
    let mut table = Vec::new();
    for (c, w) in cases.iter().zip(want) {
        let r = record(&d, c[0].as_str().unwrap(), c[2].as_str(), c[1].as_i64());
        let got = json!({
            "identity": r.identity, "blocks": r.blocks, "structure_basis": r.structure_basis, "acute": r.acute, "acute_tier": r.acute_tier,
            "acute_basis": r.acute_basis, "chronic": r.chronic, "chronic_tier": r.chronic_tier, "chronic_basis": r.chronic_basis,
            "chronic_route": r.chronic_route, "degradability": r.degradability, "ecosar_classes": r.ecosar_classes,
            "log_kow": r.log_kow, "caveat": r.caveat, "cross_check": r.cross_check,
        });
        same(&got, w, &format!("record {}", r.identity));
        table.push(r);
    }
    let anthropogenic: Vec<bool> = (0..table.len()).map(|i| i % 2 == 0).collect();
    let got: serde_json::Map<String, Value> = coverage(&table, Some(&anthropogenic))
        .into_iter()
        .map(|(k, v)| (k, json!(v)))
        .collect();
    same(&Value::Object(got), &g["coverage"], "coverage");
}
