#![cfg(feature = "attributio")]
//! Parity of the MS2 confirmation (B3) with the golden data of the reference tool (`tools/attributio_riferimento.py`, stage `tox-b3`): the verdict of every
//! B2 candidate from synthetic SIRIUS tables, a MetFrag summary and the chemist's decisions, the accepted ones and the formula parser.

#[macro_use]
mod attributio_comune;
use attributio_comune::*;
use mzlab_feat::attributio::reglists::confirm::*;
use mzlab_feat::attributio::reglists::screen::Hit;
use serde_json::{json, Value};
use std::collections::BTreeMap;

fn s(v: &Value) -> String {
    v.as_str().unwrap_or("").to_string()
}

fn opt<T: Into<Value>>(v: Option<T>) -> Value {
    v.map_or(Value::Null, Into::into)
}

/// The id of a SIRIUS row: the export names it `mappingFeatureId`, the campaign's own tables `_id`.
fn id_of(r: &Value) -> i64 {
    r.get("mappingFeatureId")
        .or_else(|| r.get("_id"))
        .and_then(Value::as_i64)
        .unwrap()
}

fn sirius_of(t: &Value) -> SiriusIn {
    let list = |k: &str| t[k].as_array().cloned().unwrap_or_default();
    SiriusIn {
        kind: s(&t["kind"]),
        structures: list("structures")
            .iter()
            .map(|r| StructureIn {
                id: id_of(r),
                rank: r["structurePerIdRank"].as_i64(),
                inchikey2d: s(&r["InChIkey2D"]),
                name: r["name"].as_str().map(str::to_string),
                cosmic: r["ConfidenceScoreApproximate"].as_f64(),
            })
            .collect(),
        formulas: list("formulas")
            .iter()
            .map(|r| FormulaIn {
                id: id_of(r),
                rank: r["formulaRank"].as_i64(),
                formula: s(&r["molecularFormula"]),
            })
            .collect(),
        matches: list("matches")
            .iter()
            .map(|r| MatchIn {
                id: id_of(r),
                inchikey2d: r["InChIkey2D"].as_str().map(str::to_string),
                similarity: r["similarity"].as_f64().unwrap(),
                shared_peaks: r["sharedPeaks"].as_i64().unwrap(),
                analog: s(&r["analogHit"]).to_lowercase() != "false",
            })
            .collect(),
    }
}

fn hit_of(r: &Value) -> Hit {
    Hit {
        class: s(&r["classe"]),
        suspect: s(&r["suspect"]),
        lists: s(&r["lists"]),
        list_labels: s(&r["list_labels"]),
        is_tp: s(&r["is_tp"]),
        parent: s(&r["parent"]),
        formula: s(&r["formula"]),
        polarity: s(&r["polarity"]),
        feature_id: r["feature_id"].as_i64().unwrap(),
        adduct: s(&r["adduct"]),
        mz_obs: r["mz_obs"].as_f64().unwrap(),
        ppm: r["ppm"].as_f64().unwrap(),
        rt: r["rt"].as_f64().unwrap(),
        rt_pred: r["rt_pred"].as_f64(),
        rt_logp: s(&r["rt_logp"]),
        in_blanks: s(&r["in_blanks"]),
        sites: s(&r["sites"]),
        sorbent: s(&r["sorbent"]),
        list_attributes: s(&r["list_attributes"]),
        isolated_adduct: r["isolated_adduct"].as_bool().unwrap(),
        suspect_skeleton: s(&r["suspect_skeleton"]),
        ..Hit::default()
    }
}

fn verdict_json(v: &Verdict) -> Value {
    json!({"verdetto": v.verdict, "verdetto_regole": v.rules, "nota": v.note, "deciso_il": v.decided_on, "classe_B2": v.class_b2, "suspect": v.suspect,
        "lists": v.lists, "list_attributes": v.list_attributes, "is_tp": v.is_tp, "parent": v.parent, "polarity": v.polarity, "feature_id": v.feature_id,
        "adduct": v.adduct, "mz_obs": v.mz_obs, "ppm": v.ppm, "rt": v.rt, "rt_pred": opt(v.rt_pred), "formula_suspect": v.formula_suspect,
        "sirius_formula_top1": opt(v.sirius_formula_top1.clone()), "suspect_formula_rank": opt(v.suspect_formula_rank),
        "sirius_structure_rank": opt(v.sirius_structure_rank), "sirius_top1": opt(v.sirius_top1.clone()), "cosmic_top1": opt(v.cosmic_top1),
        "lib_cosine": opt(v.lib_cosine), "lib_shared_peaks": opt(v.lib_shared_peaks), "metfrag_rank_pcl": opt(v.metfrag_rank),
        "metfrag_expl_peaks": opt(v.metfrag_expl_peaks), "metfrag_n_cand": opt(v.metfrag_n_cand), "a_top5_or_cosmic": v.a_top5_or_cosmic,
        "b_rt_logp": v.b_rt_logp, "c_formula": v.c_formula, "in_blanks": v.in_blanks, "sites": v.sites, "sorbent": v.sorbent,
        "isolated_adduct": v.isolated_adduct, "suspect_skeleton": v.suspect_skeleton, "sirius_input": v.sirius_input})
}

#[test]
fn b3_matches_the_golden_data() {
    let g = golden("tox", "b3.json");
    let hits: Vec<Hit> = g["hits"].as_array().unwrap().iter().map(hit_of).collect();
    let sirius: BTreeMap<String, SiriusIn> = g["sirius"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| (k.clone(), sirius_of(v)))
        .collect();
    let metfrag: Vec<MetFragIn> = g["metfrag"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| MetFragIn {
            pol: s(&r["pol"]),
            fid: r["fid"].as_i64().unwrap(),
            adduct: s(&r["adduct"]),
            suspect: s(&r["suspect"]),
            rank: r["mf_rank"].as_f64(),
            expl: r["expl"].as_f64(),
            n_cand: r["n_cand"].as_f64(),
        })
        .collect();
    let decisions: Vec<Decision> = g["decisions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| Decision {
            suspect: s(&r["suspect"]),
            polarity: s(&r["polarity"]),
            feature_id: s(&r["feature_id"]),
            verdict: s(&r["verdetto"]),
            note: s(&r["nota"]),
            decided_on: s(&r["decided_on"]),
        })
        .collect();
    let rows = evaluate(&hits, &sirius, &metfrag, &decisions).unwrap();
    same(
        &json!(rows.iter().map(verdict_json).collect::<Vec<_>>()),
        &g["rows"],
        "rows",
    );
    let acc: Vec<Value> = accepted(&rows)
        .iter()
        .map(|r| json!([r.suspect, r.polarity, r.feature_id]))
        .collect();
    same(&json!(acc), &g["accepted"], "accepted");
    same(&json!(METHOD), &g["method"], "method");
}

#[test]
fn formulas_parse_like_the_reference() {
    for r in golden("tox", "b3.json")["formulas"].as_array().unwrap() {
        same(
            &json!(parse_formula(r[0].as_str().unwrap())),
            &r[1],
            r[0].as_str().unwrap(),
        );
    }
}
