#![cfg(feature = "attributio")]
//! Parity of the MS1 suspect screening (B2) with the golden data of the reference tool (`tools/attributio_riferimento.py`, stage `tox-b2`): suspects, those
//! already identified, hits with their classes, per-list summary, RT ~ logP model, method lines and the theoretical M+2 ratios.

#[macro_use]
mod attributio_comune;
use attributio_comune::*;
use mzlab_feat::attributio::reglists::b1::StructRow;
use mzlab_feat::attributio::reglists::screen::*;
use mzlab_feat::attributio::tox::snapshot::{read_membership, read_registry};
use serde_json::{json, Value};
use std::collections::HashMap;

fn s(v: &Value) -> String {
    v.as_str().unwrap_or("").to_string()
}

fn read(name: &str) -> String {
    std::fs::read_to_string(
        root()
            .join("tests/data/attributio/istantanee_sintetiche")
            .join(name),
    )
    .expect("snapshot")
}

fn opt(v: Option<f64>) -> Value {
    v.map_or(Value::Null, |x| json!(x))
}

fn suspect_json(x: &Suspect) -> Value {
    json!({"attributes": x.attributes, "formula": x.formula, "inchikey": x.inchikey, "inchikey_skeleton": x.skeleton, "is_tp": x.is_tp, "lc_esi": x.lc_esi,
        "list_labels": x.list_labels, "lists": x.lists, "logp": opt(x.logp), "logp_source": x.logp_source, "mono_mass": x.mono_mass, "name": x.name,
        "parent": x.parent, "smiles": x.smiles, "xlogp": x.xlogp})
}

fn hit_json(h: &Hit) -> Value {
    json!({"adduct": h.adduct, "already_annotated_as": h.already_annotated_as, "classe": h.class, "feature_id": h.feature_id, "feature_level": h.feature_level,
        "formula": h.formula, "formula_match": h.formula_match, "in_blanks": h.in_blanks, "is_tp": h.is_tp, "isolated_adduct": h.isolated_adduct,
        "list_attributes": h.list_attributes, "list_labels": h.list_labels, "lists": h.lists, "logp": opt(h.logp), "logp_source": h.logp_source,
        "m2_theor": h.m2_theor, "ms2": h.ms2, "mz_obs": h.mz_obs, "mz_theor": h.mz_theor, "parent": h.parent, "polarity": h.polarity, "ppm": h.ppm,
        "rt": h.rt, "rt_logp": h.rt_logp, "rt_pred": opt(h.rt_pred), "sirius_formula": h.sirius_formula, "sites": h.sites, "sorbent": h.sorbent,
        "suspect": h.suspect, "suspect_inchikey": h.suspect_inchikey, "suspect_skeleton": h.suspect_skeleton})
}

fn summary_json(r: &SummaryRow) -> Value {
    let mut v = json!({"list": r.list, "list_id": r.list_id, "suspects_screened": r.suspects_screened, "suspects_with_hits": r.suspects_with_hits,
        "hits": r.hits, "suspects_with_MS2_hit": r.suspects_with_ms2_hit});
    for (i, c) in CLASSES.iter().enumerate() {
        v[format!("{c}_hits")] = json!(r.class_hits[i]);
        v[format!("{c}_suspects")] = json!(r.class_suspects[i]);
    }
    v
}

fn features_of(g: &Value, sites: &[String], sorbents: &[String]) -> Vec<FeatureRow> {
    g["features"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| FeatureRow {
            pol: s(&r["_pol"]),
            id: r["_id"].as_i64().unwrap(),
            mz: r["mz"].as_f64().unwrap(),
            rt: r["rt"].as_f64().unwrap(),
            level: r["level"].as_i64().unwrap(),
            xlogp: r["xlogp"].as_f64(),
            formula: s(&r["formula"]),
            in_site: sites
                .iter()
                .map(|x| r[format!("in_{x}")].as_bool().unwrap())
                .collect(),
            any_sorbent: sorbents
                .iter()
                .map(|x| r[format!("any_{x}")].as_bool().unwrap())
                .collect(),
        })
        .collect()
}

fn evidence_of(g: &Value) -> Evidence {
    let mut ev = Evidence::default();
    for r in g["info"].as_array().unwrap() {
        ev.ms2_blank.insert(
            (s(&r[0]), r[1].as_i64().unwrap()),
            (r[2].as_bool().unwrap(), s(&r[3])),
        );
    }
    for r in g["annotated"].as_array().unwrap() {
        ev.annotated
            .insert((s(&r[0]), r[1].as_i64().unwrap()), s(&r[2]));
    }
    ev
}

#[test]
fn b2_matches_the_golden_data() {
    let g = golden("tox", "b2.json");
    let strs = |k: &str| -> Vec<String> { g[k].as_array().unwrap().iter().map(s).collect() };
    let (sites, sorbents) = (strs("sites"), strs("sorbents"));
    let feat = features_of(&g, &sites, &sorbents);
    let ev = evidence_of(&g);
    let structs: Vec<StructRow> = g["structures"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| StructRow {
            nome_comune: s(&r["nome_comune"]),
            first_block: r["first_block"].as_str().map(str::to_string),
            ..StructRow::default()
        })
        .collect();
    let mem = read_membership(&read("b2_membership.csv")).unwrap();
    let registry: HashMap<String, Option<f64>> = read_registry(&read("compound_registry.csv"))
        .unwrap()
        .into_iter()
        .map(|r| (r.first_block, r.xlogp))
        .collect();
    for (n, case) in g["cases"].as_array().unwrap().iter().enumerate() {
        let c = &case["cfg"];
        let cfg = ScreenConfig {
            lists: c["b2_lists"].as_array().unwrap().iter().map(s).collect(),
            window: c["mz_window"]
                .as_array()
                .map(|w| (w[0].as_f64().unwrap(), w[1].as_f64().unwrap())),
            sites: sites.clone(),
            sorbents: sorbents.clone(),
            sorbent_labels: c["sorbent_labels"]
                .as_object()
                .map(|o| o.iter().map(|(k, v)| (k.clone(), s(v))).collect())
                .unwrap_or_default(),
        };
        let b2 = screen_campaign(&feat, &ev, &structs, &mem, &registry, &cfg).unwrap();
        let at = format!("case {n}");
        same(
            &json!(b2.suspects.iter().map(suspect_json).collect::<Vec<_>>()),
            &case["suspects"],
            &format!("{at} suspects"),
        );
        let already: Vec<Value> = b2
            .already
            .iter()
            .map(|(x, a)| {
                let mut v = suspect_json(x);
                v["identified_as"] = json!(a);
                v
            })
            .collect();
        same(&json!(already), &case["already"], &format!("{at} already"));
        same(
            &json!(b2.hits.iter().map(hit_json).collect::<Vec<_>>()),
            &case["hits"],
            &format!("{at} hits"),
        );
        same(
            &json!(b2.summary.iter().map(summary_json).collect::<Vec<_>>()),
            &case["summary"],
            &format!("{at} summary"),
        );
        let m = b2.rt_model;
        same(
            &json!([m.slope, m.intercept, m.sd, m.n]),
            &case["rt_model"],
            &format!("{at} rt_model"),
        );
        same(&json!(b2.lists), &case["lists"], &format!("{at} lists"));
        same(
            &json!(method_lines(&b2, cfg.window)),
            &case["method"],
            &format!("{at} method"),
        );
    }
}

#[test]
fn m2_ratio_matches_the_golden_data() {
    for r in golden("tox", "b2.json")["m2"].as_array().unwrap() {
        same(
            &json!(m2_ratio(r[0].as_str().unwrap())),
            &r[1],
            r[0].as_str().unwrap(),
        );
    }
}
