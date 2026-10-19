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

// ------------------------------------------------------------------------------------------------------------------ amenability (RG-03)
use mzlab_feat::attributio::tox::amenability::lc_esi_amenable;

#[test]
fn lc_esi_amenability_matches_the_golden_data() {
    let g = golden("tox", "amenabilita.json");
    for c in g["cases"].as_array().unwrap() {
        let (verdict, rule) = lc_esi_amenable(
            c[0].as_str().unwrap_or(""),
            c[1].as_f64(),
            c[2].as_str(),
            c[3].as_i64(),
            c[4].as_f64(),
            c[5].as_str().unwrap(),
        );
        assert_eq!(
            json!([verdict, rule]),
            c[6],
            "amenability {:?}",
            &c.as_array().unwrap()[..6]
        );
    }
}

// ------------------------------------------------------------------------------------------------------------------ B1 and P3 (RG-03)
use mzlab_feat::attributio::reglists::b1::{match_campaign, B1Config, IdentityIn, StructRow, B1};
use mzlab_feat::attributio::reglists::catalog;
use mzlab_feat::attributio::reglists::p3::cross_table;

fn s(v: &Value) -> String {
    v.as_str().unwrap_or("").to_string()
}

fn b1_of(g: &Value, lists: Option<&[&str]>) -> B1 {
    let sites: Vec<String> = g["sites"].as_array().unwrap().iter().map(s).collect();
    let idents: Vec<IdentityIn> = g["identities_in"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| IdentityIn {
            nome_comune: s(&r["nome_comune"]),
            group: s(&r["group"]),
            best_level: r["best_level"].as_i64().unwrap(),
            in_site: sites
                .iter()
                .map(|x| r[format!("in_{x}")].as_bool().unwrap())
                .collect(),
        })
        .collect();
    let structs: Vec<StructRow> = g["structures"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| StructRow {
            nome_comune: s(&r["nome_comune"]),
            identity: s(&r["identity"]),
            first_block: r["first_block"].as_str().map(str::to_string),
            inchikey: r["inchikey"].as_str().map(str::to_string),
            key_origin: s(&r["key_origin"]),
            ik_basis: s(&r["ik_basis"]),
        })
        .collect();
    let mem = read_membership(&read("reglist_membership.csv")).unwrap();
    let cfg = B1Config {
        sites,
        submatrices: vec![],
        window: g["window"]
            .as_array()
            .map(|w| (w[0].as_f64().unwrap(), w[1].as_f64().unwrap())),
    };
    match_campaign(&idents, &structs, &mem, &cfg, lists)
}

#[test]
fn b1_matches_the_golden_data() {
    let g = golden("tox", "b1.json");
    assert_eq!(
        json!(catalog::ordered(&[
            "WHO_MIA", "ZZ", "HHP_PAN", "AA", "WHO_MIA"
        ])),
        g["ordered"][0]
    );
    assert_eq!(json!(catalog::ordered(&[])), g["ordered"][1]);
    for case in ["window", "nowindow"] {
        let g = &g[case];
        let b = b1_of(g, None);
        assert_eq!(json!(b.lists), g["lists"], "{case} lists");
        for (c, w) in g["catalog"].as_array().unwrap().iter().zip(&b.lists) {
            assert_eq!(
                json!([
                    w,
                    catalog::label(w),
                    catalog::group_of(w),
                    catalog::question(w)
                ]),
                *c
            );
        }
        let want = g["ident"].as_array().unwrap();
        assert_eq!(b.ident.len(), want.len());
        for (i, w) in b.ident.iter().zip(want) {
            let at = format!("{case} {}", i.nome_comune);
            assert_eq!(json!(i.nome_comune), w["nome_comune"], "{at}");
            assert_eq!(
                json!(i.first_blocks),
                w["first_blocks"],
                "{at} first_blocks"
            );
            assert_eq!(json!(i.inchikeys), w["inchikeys"], "{at} inchikeys");
            assert_eq!(json!(i.key_origin), w["key_origin"], "{at} key_origin");
            assert_eq!(
                json!(i.structure_basis),
                w["structure_basis"],
                "{at} structure_basis"
            );
            assert_eq!(
                (
                    json!(i.split),
                    json!(&i.merge),
                    json!(&i.sites),
                    json!(i.all_sites)
                ),
                (
                    w["SPLIT"].clone(),
                    w["MERGE"].clone(),
                    w["sites"].clone(),
                    w["all_sites"].clone()
                ),
                "{at}"
            );
            assert_eq!(json!(i.n_lists), w["n_lists"], "{at} n_lists");
            assert_eq!(json!(&i.lists), w["lists"], "{at} lists");
            for (lid, label) in b.lists.iter().zip(&i.labels) {
                assert_eq!(json!(label), w[lid], "{at} {lid}");
            }
        }
        let want = g["long"].as_array().unwrap();
        assert_eq!(b.long.len(), want.len(), "{case} long");
        for (r, w) in b.long.iter().zip(want) {
            assert_eq!(
                json!([
                    &r.nome_comune,
                    &r.identity,
                    &r.first_block,
                    &r.list_id,
                    &r.list_name,
                    &r.list_attribute,
                    &r.list_inchikey,
                    r.matched
                ]),
                json!([
                    w["nome_comune"],
                    w["identity"],
                    w["first_block"],
                    w["list_id"],
                    w["list_name"],
                    w["list_attribute"],
                    w["list_inchikey"],
                    w["match"]
                ])
            );
        }
        for (r, w) in b.summary.iter().zip(g["summary"].as_array().unwrap()) {
            let got = json!({"list_id": r.list_id, "group": r.group, "list": r.list, "question": r.question, "entries": r.entries, "Y_vero_in_window": r.y_true_in_window,
                "Y_vero_dubbio_in_window": r.y_true_doubtful_in_window, "X_identities": r.x_identities, "X_anthropogenic": r.x_anthropogenic, "X_natural": r.x_natural,
                "X_by_InChIKey": r.x_by_inchikey, "X_first_block_only": r.x_first_block_only, "all_sites": r.all_sites, "matched": r.matched});
            for (k, v) in got.as_object().unwrap() {
                assert_eq!(v, &w[k], "{case} summary {} {k}", r.list_id);
            }
        }
        assert_eq!(b.summary.len(), g["summary"].as_array().unwrap().len());
        let cov: Vec<Value> = b
            .coverage
            .iter()
            .map(|(what, n)| json!({"what": what, "n": n}))
            .collect();
        assert_eq!(Value::Array(cov), g["coverage"], "{case} coverage");
    }
}

#[test]
fn p3_matches_the_golden_data() {
    let g = golden("tox", "b1.json");
    let base = &g["window"];
    let all = b1_of(base, None);
    let h = b1_of(base, Some(&["HHP_PAN", "EU_PS_2026_805"]));
    let w = b1_of(base, Some(&["WHO_MIA", "EU_PS_2026_805"]));
    for (tag, res) in [
        ("p3_two", vec![("ALL", &all), ("H", &h)]),
        ("p3_three", vec![("ALL", &all), ("H", &h), ("W", &w)]),
        ("p3_one", vec![("W", &w)]),
    ] {
        let (t, sm) = cross_table(&res);
        let want = &g[tag];
        let wt = want["table"].as_array().unwrap();
        assert_eq!(t.len(), wt.len(), "{tag}");
        for (r, w) in t.iter().zip(wt) {
            assert_eq!(
                json!([
                    &r.list_group,
                    &r.list_id,
                    &r.list,
                    &r.first_block,
                    &r.list_name,
                    &r.found_in,
                    &r.list_inchikey,
                    &r.list_attribute
                ]),
                json!([
                    w["list_group"],
                    w["list_id"],
                    w["list"],
                    w["first_block"],
                    w["list_name"],
                    w["found_in"],
                    w["list_inchikey"],
                    w["list_attribute"]
                ]),
                "{tag}"
            );
            for ((name, _), sd) in res.iter().zip(&r.sides) {
                match sd {
                    Some(sd) => assert_eq!(
                        json!([
                            &sd.identity,
                            sd.level as f64,
                            sd.group,
                            &sd.sites,
                            sd.matched
                        ]),
                        json!([
                            w[format!("{name}_identity")],
                            json!(w[format!("{name}_level")].as_f64().unwrap()),
                            w[format!("{name}_group")],
                            w[format!("{name}_sites")],
                            w[format!("{name}_match")]
                        ]),
                        "{tag}"
                    ),
                    None => assert!(w[format!("{name}_identity")].is_null(), "{tag}"),
                }
            }
        }
        let ws = want["summary"].as_array().unwrap();
        assert_eq!(sm.len(), ws.len());
        for (r, w) in sm.iter().zip(ws) {
            assert_eq!(
                json!([
                    &r.list_group,
                    &r.list_id,
                    &r.list,
                    r.total,
                    &r.compounds_in_both
                ]),
                json!([
                    w["list_group"],
                    w["list_id"],
                    w["list"],
                    w["total"],
                    w["compounds_in_both"]
                ])
            );
            assert_eq!(
                json!(r.both),
                if res.len() == 2 {
                    w["both"].clone()
                } else {
                    w["all"].clone()
                }
            );
            for ((name, _), n) in res.iter().zip(&r.only) {
                assert_eq!(json!(n), w[format!("{name}_only")]);
            }
        }
    }
}
