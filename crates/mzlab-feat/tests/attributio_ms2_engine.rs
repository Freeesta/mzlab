//! MS-00: the MS2 engine (compiler + entropy search) against `tools/ms2_engine_riferimento.py` (`ms_entropy.FlashEntropySearch`) on invented spectra.
//! Library read from GNPS-JSON text built from the golden; scores equal within 1e-6 (float32 peaks), hit sets equal.
#![cfg(feature = "attributio")]

#[macro_use]
mod attributio_comune;
use attributio_comune::*;
use mzlab_feat::attributio::annotate::{gnps_comparer, load_annotations, BasicCuration};
use mzlab_feat::attributio::campaign::Env;
use mzlab_feat::attributio::io::Io;
use mzlab_feat::attributio::sources::load_sources;
use mzlab_feat::attributio::Campaign;

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

/// MS-01: the engine hits ride along as evidence; they never move a level, and without `use_ms2_engine` they are ignored.
#[test]
fn engine_hits_are_evidence_not_levels() {
    let levels = |on: bool| {
        let mut fx = fixture("campagna_sintetica");
        let mut toml = String::from_utf8(fx.files.read("campaign.toml").unwrap()).unwrap();
        if on {
            toml.push_str("\n[annotation]\nuse_ms2_engine = true\n");
        }
        fx.files.add("campaign.toml", toml.as_bytes());
        fx.campaign =
            Campaign::from_toml(&toml, "campaign.toml", "campagna_sintetica", Env::default())
                .unwrap();
        fx.files.add(
            "02 Data/Data Synth/engine/engine_pos.csv",
            b"_pol,_id,engine_name,engine_score,engine_precursor_mz,engine_id\nPOS,1,Invented compound,0.91,200.1,LIB1\nPOS,2,Another,0.7,210.2,LIB2\n",
        );
        with_io!(fx, io);
        let src = load_sources(&fx.campaign, &io).unwrap();
        load_annotations(&fx.campaign, &src, None, &BasicCuration, &gnps_comparer).unwrap()
    };
    let (off, on) = (levels(false), levels(true));
    assert_eq!(off.features.len(), on.features.len());
    let sig = |a: &mzlab_feat::attributio::annotate::Annotations| -> Vec<(i32, String, String)> {
        a.features
            .iter()
            .map(|f| (f.level, f.level_sub.clone(), f.level_basis.clone()))
            .collect()
    };
    assert_eq!(sig(&off), sig(&on), "levels must not change");
    assert!(off.features.iter().all(|f| f.engine.name.is_none()));
    let f1 = on
        .features
        .iter()
        .find(|f| f.pol == "POS" && f.id == 1)
        .unwrap();
    assert_eq!(f1.engine.name.as_deref(), Some("Invented compound"));
    assert_eq!(f1.engine.score, Some(0.91));
    assert!(
        on.features
            .iter()
            .filter(|f| f.engine.name.is_some())
            .count()
            == 2
    );
}

/// MS-02: the fragmenter score equals the pure-Python reference on the explained sets of the Rust fragmenter; the explained sets themselves equal the RDKit
/// ones for at least 90 % of the structures (the same bar as the fragmenter, `attributio_spiegazioni`: there is no RDKit in the engine).
#[test]
fn insilico_score_follows_the_reference() {
    use mzlab_feat::attributio::ms2_engine::insilico::{
        fragmenter_score, rank, read_candidates, InSilicoOpts,
    };
    use mzlab_feat::attributio::review::fragmenter::fragment_molecule;
    let p = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/data/attributio/ms2_engine/insilico.json"
    );
    let g: Value = serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap();
    let (mut same, mut total) = (0, 0);
    for m in g["molecole"].as_array().unwrap() {
        let (peaks, ints) = (nums(&m["peaks"]), nums(&m["intensities"]));
        let pairs: Vec<(f64, f64)> = peaks.iter().cloned().zip(ints).collect();
        let pol = m["pol"].as_str().unwrap();
        let fr = fragment_molecule(m["smiles"].as_str().unwrap(), pol, &peaks, 10.0, 200).unwrap();
        let mut got: Vec<f64> = fr.explained.iter().map(|e| e.peak).collect();
        got.sort_by(f64::total_cmp);
        got.dedup();
        let mut want = nums(&m["explained"]);
        want.sort_by(f64::total_cmp);
        total += 1;
        if got == want {
            same += 1;
            let d = (fragmenter_score(&pairs, &got) - m["score"].as_f64().unwrap()).abs();
            assert!(d < 1e-12, "{} score {d}", m["smiles"]);
        }
    }
    eprintln!("explained sets equal: {same}/{total}");
    assert!(same as f64 / total as f64 >= 0.90, "{same}/{total}");

    // hand case: caffeine (194.0804) is found among three candidates, the others are out of the mass window
    let list = "Identifier,CompoundName,SMILES,MolecularFormula\n1,Caffeine,Cn1cnc2c1c(=O)n(C)c(=O)n2C,C8H10N4O2\n2,Theophylline,Cn1cnc2c1c(=O)[nH]c(=O)n2C,C7H8N4O2\n3,Ethanol,CCO,C2H6O\n";
    let cands = read_candidates(list);
    assert_eq!(cands.len(), 3);
    let r = rank(
        195.0877,
        "POS",
        &[(138.0662, 100.0), (110.0713, 40.0), (42.0, 5.0)],
        &cands,
        &InSilicoOpts::default(),
    );
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].candidate.name, "Caffeine");
    assert!(r[0].mass_ppm.abs() < 1.0);
}
