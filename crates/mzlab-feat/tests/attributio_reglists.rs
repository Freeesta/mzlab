#![cfg(feature = "attributio")]
//! Parity of the regulatory-list matching (B1 by full InChIKey with the first block flagged, and the cross-campaign table P3) with the golden data of the
//! reference tool (`tools/attributio_riferimento.py`, stage `tox-b1`) on the invented snapshots in `tests/data/attributio/istantanee_sintetiche/`.

#[macro_use]
mod attributio_comune;
use attributio_comune::*;
use mzlab_feat::attributio::tox::snapshot::read_membership;
use serde_json::{json, Value};

fn read(name: &str) -> String {
    std::fs::read_to_string(
        root()
            .join("tests/data/attributio/istantanee_sintetiche")
            .join(name),
    )
    .expect("snapshot")
}

use mzlab_feat::attributio::reglists::b1::{match_campaign, B1Config, IdentityIn, StructRow, B1};
use mzlab_feat::attributio::reglists::catalog;
use mzlab_feat::attributio::reglists::p3::{cross_table, CrossSummary};

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

fn check_ident(case: &str, b: &B1, g: &Value) {
    let want = g["ident"].as_array().unwrap();
    assert_eq!(b.ident.len(), want.len());
    for (i, w) in b.ident.iter().zip(want) {
        let at = format!("{case} {}", i.nome_comune);
        let got = json!({"nome_comune": i.nome_comune, "first_blocks": i.first_blocks, "inchikeys": i.inchikeys, "key_origin": i.key_origin,
            "structure_basis": i.structure_basis, "SPLIT": i.split, "MERGE": i.merge, "sites": i.sites, "all_sites": i.all_sites,
            "n_lists": i.n_lists, "lists": i.lists});
        for (k, v) in got.as_object().unwrap() {
            assert_eq!(v, &w[k], "{at} {k}");
        }
        for (lid, label) in b.lists.iter().zip(&i.labels) {
            assert_eq!(json!(label), w[lid], "{at} {lid}");
        }
    }
}

fn check_long(case: &str, b: &B1, g: &Value) {
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
}

fn check_summary(case: &str, b: &B1, g: &Value) {
    let want = g["summary"].as_array().unwrap();
    assert_eq!(b.summary.len(), want.len());
    for (r, w) in b.summary.iter().zip(want) {
        let got = json!({"list_id": r.list_id, "group": r.group, "list": r.list, "question": r.question, "entries": r.entries,
            "Y_vero_in_window": r.y_true_in_window, "Y_vero_dubbio_in_window": r.y_true_doubtful_in_window, "X_identities": r.x_identities,
            "X_anthropogenic": r.x_anthropogenic, "X_natural": r.x_natural, "X_by_InChIKey": r.x_by_inchikey,
            "X_first_block_only": r.x_first_block_only, "all_sites": r.all_sites, "matched": r.matched});
        for (k, v) in got.as_object().unwrap() {
            assert_eq!(v, &w[k], "{case} summary {} {k}", r.list_id);
        }
    }
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
        check_ident(case, &b, g);
        check_long(case, &b, g);
        check_summary(case, &b, g);
        let cov: Vec<Value> = b
            .coverage
            .iter()
            .map(|(what, n)| json!({"what": what, "n": n}))
            .collect();
        assert_eq!(Value::Array(cov), g["coverage"], "{case} coverage");
    }
}

fn check_cross(tag: &str, res: &[(&str, &B1)], want: &Value) {
    let (t, sm) = cross_table(res);
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
            let Some(sd) = sd else {
                assert!(w[format!("{name}_identity")].is_null(), "{tag}");
                continue;
            };
            let col = |c: &str| w[format!("{name}_{c}")].clone();
            assert_eq!(
                json!([
                    &sd.identity,
                    sd.level as f64,
                    sd.group,
                    &sd.sites,
                    sd.matched
                ]),
                json!([
                    col("identity"),
                    json!(col("level").as_f64().unwrap()),
                    col("group"),
                    col("sites"),
                    col("match")
                ]),
                "{tag}"
            );
        }
    }
    check_cross_summary(tag, res, &sm, want);
}

fn check_cross_summary(tag: &str, res: &[(&str, &B1)], sm: &[CrossSummary], want: &Value) {
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

#[test]
fn p3_matches_the_golden_data() {
    let g = golden("tox", "b1.json");
    let base = &g["window"];
    let all = b1_of(base, None);
    let h = b1_of(base, Some(&["HHP_PAN", "EU_PS_2026_805"]));
    let w = b1_of(base, Some(&["WHO_MIA", "EU_PS_2026_805"]));
    check_cross("p3_two", &[("ALL", &all), ("H", &h)], &g["p3_two"]);
    check_cross(
        "p3_three",
        &[("ALL", &all), ("H", &h), ("W", &w)],
        &g["p3_three"],
    );
    check_cross("p3_one", &[("W", &w)], &g["p3_one"]);
}
