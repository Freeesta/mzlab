//! MS-00: the MS2 engine (compiler + entropy search) against `tools/ms2_engine_riferimento.py` (`ms_entropy.FlashEntropySearch`) on invented spectra.
//! Library read from GNPS-JSON text built from the golden; scores equal within 1e-6 (float32 peaks), hit sets equal.
#![cfg(feature = "attributio")]

use mzlab_feat::attributio::ms2_engine::matcher::{Query, SearchOpts};
use mzlab_feat::attributio::ms2_engine::{compile_library, Ms2Library};
use serde_json::Value;

fn golden() -> Value {
    let p = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/data/attributio/ms2_engine/golden.json"
    );
    serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
}

fn nums(v: &Value) -> Vec<f64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect()
}

#[test]
fn engine_equals_ms_entropy() {
    let g = golden();
    let lib_json: Vec<Value> = g["library"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| {
            serde_json::json!({
                "spectrum_id": s["id"], "Compound_Name": s["name"], "Precursor_MZ": s["precursor_mz"],
                "Ion_Mode": s["ion_mode"], "Adduct": s["adduct"], "peaks_json": s["peaks"].to_string(),
            })
        })
        .collect();
    let (entries, stats) = compile_library(
        "lib.json",
        serde_json::to_string(&lib_json).unwrap().as_bytes(),
    )
    .unwrap();
    assert_eq!(entries.len(), 60, "{stats:?}");
    let lib = Ms2Library::new(entries);
    let o = SearchOpts {
        ppm: g["ppm"].as_f64().unwrap(),
        min_score: g["min_score"].as_f64().unwrap(),
        min_shared: 0,
        top: 1000,
    };
    let mut n_hits = 0;
    let mut worst: f64 = 0.0;
    for q in g["queries"].as_array().unwrap() {
        let qid = q["query_id"].as_i64().unwrap();
        let query = Query {
            id: qid.to_string(),
            precursor_mz: q["precursor_mz"].as_f64().unwrap(),
            pol: if q["ion_mode"] == "POSITIVE" { 1 } else { -1 },
            mz: nums(&q["peaks_mz"]),
            it: nums(&q["peaks_int"]),
        };
        let got = lib.search(&query, &o);
        let want = g["hits"][qid.to_string()].as_array().unwrap();
        let got_ids: Vec<&str> = got.iter().map(|h| h.id.as_str()).collect();
        let mut want_ids: Vec<&str> = want.iter().map(|h| h["id"].as_str().unwrap()).collect();
        let mut gs = got_ids.clone();
        gs.sort();
        want_ids.sort();
        assert_eq!(gs, want_ids, "query {qid}");
        for h in &got {
            let w = want.iter().find(|w| w["id"] == h.id.as_str()).unwrap();
            worst = worst.max((h.score - w["score"].as_f64().unwrap()).abs());
            n_hits += 1;
        }
    }
    assert!(n_hits > 10);
    assert!(worst < 1e-6, "worst score difference {worst}");
}
