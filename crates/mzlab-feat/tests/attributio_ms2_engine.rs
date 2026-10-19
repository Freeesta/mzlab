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
use mzlab_feat::attributio::ms2_engine::rearrange::attribute;
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

/// MS-03: the hydrogen-rearrangement attribution equals the reference built on Attributio's sub-formula table (formulas, shifts, ppm, losses, score at 1e-9).
#[test]
fn rearrangement_attribution_equals_the_reference() {
    use mzlab_feat::attributio::review::explain::parse_formula;
    use mzlab_feat::attributio::review::settings::ReviewSettings;
    let p = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/data/attributio/ms2_engine/rearrange.json"
    );
    let g: Value = serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap();
    let s = ReviewSettings::default();
    let mut moved = 0;
    for c in g["cases"].as_array().unwrap() {
        let (mz, it) = (nums(&c["mz_peaks"]), nums(&c["it"]));
        let fd = parse_formula(c["formula"].as_str().unwrap());
        let got = attribute(
            &fd,
            c["mz"].as_f64().unwrap(),
            c["pol"].as_str().unwrap(),
            (&mz, &it),
            &s,
        )
        .unwrap();
        let w = &c["result"];
        assert_eq!(got.adduct, w["adduct"].as_str().unwrap());
        assert_eq!(got.n_explained as i64, w["n_explained"].as_i64().unwrap());
        assert_eq!(got.n_rearranged as i64, w["n_rearranged"].as_i64().unwrap());
        assert!((got.score - w["score"].as_f64().unwrap()).abs() < 1e-9);
        assert!((got.frac_explained - w["frac_explained"].as_f64().unwrap()).abs() < 1e-9);
        for (i, h) in w["hits"].as_array().unwrap().iter().enumerate() {
            match (&got.hits[i], h.is_null()) {
                (None, true) => {}
                (Some(x), false) => {
                    assert_eq!(x.shift as i64, h["k"].as_i64().unwrap());
                    assert_eq!(x.formula, h["formula"].as_str().unwrap());
                    assert_eq!(x.loss, h["loss"].as_str().unwrap());
                    assert!((x.mz_th - h["mz_th"].as_f64().unwrap()).abs() < 1e-9);
                    assert!((x.ppm - h["ppm"].as_f64().unwrap()).abs() < 1e-6);
                    moved += (x.shift != 0) as i32;
                }
                _ => panic!("peak {i} of {}", c["formula"]),
            }
        }
    }
    assert!(moved >= 5, "{moved} rearranged peaks");
}

/// MS-04: the forest follows scikit-learn's on the same invented retention times. Not equality (other bits, other bootstrap draws): the error on the held-out
/// structures stays within 25 % + 0.1 min of scikit-learn's and no worse than 10 % above the mean predictor (with 30 training structures neither forest has a strong signal).
#[test]
fn rt_forest_follows_scikit_learn() {
    use mzlab_feat::attributio::ms2_engine::rtpredict::RtModel;
    let p = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/data/attributio/ms2_engine/rtpredict.json"
    );
    let g: Value = serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap();
    let rows = |k: &str| -> Vec<(String, f64)> {
        g[k].as_array()
            .unwrap()
            .iter()
            .map(|r| (r[0].as_str().unwrap().to_string(), r[1].as_f64().unwrap()))
            .collect()
    };
    let (train, test) = (rows("train"), rows("test"));
    let m = RtModel::train(&train, 42).unwrap();
    let got: Vec<f64> = test.iter().map(|(s, _)| m.predict(s).unwrap()).collect();
    let mae = got
        .iter()
        .zip(&test)
        .map(|(p, (_, t))| (p - t).abs())
        .sum::<f64>()
        / got.len() as f64;
    let sk_mae = g["sklearn_mae"].as_f64().unwrap();
    let base = {
        let mean = train.iter().map(|r| r.1).sum::<f64>() / train.len() as f64;
        test.iter().map(|(_, t)| (t - mean).abs()).sum::<f64>() / test.len() as f64
    };
    eprintln!("rt forest: mae {mae:.3} (scikit-learn {sk_mae:.3}, mean predictor {base:.3})");
    assert!(mae <= sk_mae * 1.25 + 0.1, "mae {mae} vs {sk_mae}");
    assert!(
        mae <= base * 1.1,
        "mae {mae} against the mean predictor {base}"
    );
    assert!(RtModel::train(&train[..5], 42).is_none());
}

/// MS-05: the explicit command. The library is made of the campaign's own POS feature spectra (renamed), so every feature with a spectrum finds itself.
#[test]
fn generate_hits_finds_the_campaign_spectra() {
    use mzlab_feat::attributio::ms2_engine::generate::generate_hits;
    use mzlab_feat::attributio::ms2_engine::{engine_hits_csv, read_engine_hits, EngineEntry};
    let fx = fixture("campagna_sintetica");
    with_io!(fx, io);
    let src = load_sources(&fx.campaign, &io).unwrap();
    let p = src.pol("POS").unwrap();
    let entries: Vec<EngineEntry> = p
        .feature_keys()
        .iter()
        .filter_map(|k| {
            let sp = p.feature_spectra.get(&k.id)?;
            Some(EngineEntry {
                id: format!("LIB{}", k.id),
                name: format!("Invented {}", k.id),
                precursor_mz: k.mz?,
                pol: 1,
                adduct: "[M+H]+".into(),
                mz: sp.mz.clone(),
                it: sp.intensity.clone(),
            })
        })
        .collect();
    assert!(!entries.is_empty());
    let n = entries.len();
    let lib = Ms2Library::new(entries);
    let rows = generate_hits(
        &src,
        "POS",
        &lib,
        &SearchOpts {
            min_shared: 1,
            ..SearchOpts::default()
        },
    );
    assert!(!rows.is_empty() && rows.len() <= n);
    assert!(rows.iter().all(|r| r.score > 0.99
        && r.name == format!("Invented {}", r.id)
        && r.library_id == format!("LIB{}", r.id)));
    let back = read_engine_hits(&engine_hits_csv(&rows));
    assert_eq!(back.len(), rows.len());
    assert_eq!(back[0].library_id, rows[0].library_id);
    assert!(generate_hits(&src, "NEG", &lib, &SearchOpts::default()).is_empty());
}

/// The request layer: every op answers JSON on the synthetic campaign (libraries and candidate lists come from the extra files).
#[test]
fn request_layer_answers_every_op() {
    use mzlab_feat::attributio::ms2_engine::run::ms2_json;
    let fx = fixture("campagna_sintetica");
    with_io!(fx, io);
    let src = load_sources(&fx.campaign, &io).unwrap();
    let p = src.pol("POS").unwrap();
    let id = *p.feature_spectra.keys().min().unwrap();
    let k = p.feature_keys().into_iter().find(|k| k.id == id).unwrap();
    let sp = &p.feature_spectra[&id];
    let peaks: Vec<Value> = sp
        .mz
        .iter()
        .zip(&sp.intensity)
        .map(|(m, i)| serde_json::json!([m, i]))
        .collect();
    let lib = serde_json::to_string(&[serde_json::json!({
        "spectrum_id": "LIB1", "Compound_Name": "Invented", "Precursor_MZ": k.mz, "Ion_Mode": "POSITIVE", "peaks_json": Value::Array(peaks).to_string()
    })])
    .unwrap();
    let list = "Identifier,CompoundName,SMILES,MolecularFormula\n1,Caffeine,Cn1cnc2c1c(=O)n(C)c(=O)n2C,C8H10N4O2\n";
    let extra = |p: &str| match p {
        "__ms2/lib.json" => Some(lib.clone().into_bytes()),
        "__ms2/list.csv" => Some(list.as_bytes().to_vec()),
        _ => None,
    };
    let ask = |req: Value| -> Value {
        serde_json::from_str(&ms2_json(&io, "x", &extra, &req.to_string()).unwrap()).unwrap()
    };
    let r =
        ask(serde_json::json!({"op": "engine", "libraries": ["__ms2/lib.json"], "min_shared": 1}));
    assert_eq!(r["library"], 1);
    assert!(r["pos_hits"].as_i64().unwrap() >= 1 && r["pos"].as_str().unwrap().contains("LIB1"));
    let r = ask(
        serde_json::json!({"op": "insilico", "pol": "POS", "id": id, "candidates": "__ms2/list.csv"}),
    );
    assert_eq!(r["candidates"], 1);
    let r =
        ask(serde_json::json!({"op": "rearrange", "pol": "POS", "id": id, "formula": "C8H10N4O2"}));
    assert!(r.get("attribution").is_some());
    let train: Vec<Value> = (0..12)
        .map(|i| serde_json::json!(["C".repeat(i + 1), 1.0 + i as f64 * 0.3]))
        .collect();
    let r = ask(serde_json::json!({"op": "rt", "train": train, "predict": ["CCCCC"]}));
    assert_eq!(r["model"], true);
    assert!(r["predictions"][0]["rt"].as_f64().is_some());
    let r = ask(serde_json::json!({"op": "chimeric", "pol": "POS", "id": id}));
    assert!(r["isolation_window"].as_f64().unwrap() > 0.0 && r["intruders"].is_array());
    assert!(ms2_json(&io, "x", &extra, "{\"op\":\"nope\"}").is_err());
}
