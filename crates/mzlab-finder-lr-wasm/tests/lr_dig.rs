//! The low-resolution Dig on the synthetic series of `TP_Mine/tests/lr_synth.py` (same ions, kinetics and +0.30 Da shift, written here in
//! Rust): the pipeline runs natively as the pool of `feat-worker.js` runs it (detection, alignment, hypotheses), then the adapter. Checked
//! against `tests/golden/finder_lr_contratto.json` (`tools/finder_lr_riferimento.py`): the keys and JSON types of the Python answer, and its
//! rows that pass the criteria found again within ±0.35 Da and ±0.25 min.

use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter};
use mzlab_core::upload::prepare;
use mzlab_feat::legacy::lr::{self, detail::detail, summary::summary};
use serde_json::Value;

mod comune;
use comune::*;

#[path = "lr_dig/ms2_real.rs"]
mod ms2_real;
#[path = "lr_dig/ms2_window.rs"]
mod ms2_window;

fn golden() -> Value {
    let p = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/golden/finder_lr_contratto.json"
    );
    serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap()
}

fn kind(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

/// Every key of the contract is there with one of its types; nothing else.
fn same_shape(got: &Value, want: &Value, path: &str) {
    let (Value::Object(g), Value::Object(w)) = (got, want) else {
        panic!("{path}: not an object")
    };
    let keys = |m: &serde_json::Map<String, Value>| {
        m.keys().cloned().collect::<std::collections::BTreeSet<_>>()
    };
    assert_eq!(keys(g), keys(w), "{path}: keys");
    for (k, types) in w {
        let t = kind(&g[k]);
        let ok = types.as_array().unwrap().iter().any(|x| x == t);
        assert!(ok, "{path}.{k}: {t} not in {types}");
    }
}

/// `v` without the keys `keys`.
fn without(v: &Value, keys: &[&str]) -> Value {
    let mut out = v.clone();
    for k in keys {
        out.as_object_mut().unwrap().remove(*k);
    }
    out
}

#[test]
fn contract_and_rows_of_python() {
    let (files, idxs) = series();
    let d = dig(&files, &idxs);
    let s: Value = serde_json::from_str(&summary(&d, r#"{"total": 1}"#)).unwrap();
    let g = golden();
    // keys of the Rust Dig only: the parent check and the matches by chance (`lr/chance.rs`)
    assert_eq!(s["parent_ok"], true);
    assert!(s["chance"].as_array().is_some_and(|c| !c.is_empty()));
    let rust_only = [
        "ms2_links",
        "ms2_rep",
        "ms2_text",
        "parent_ok",
        "chance",
        "unexpected_left",
    ];
    same_shape(&without(&s, &rust_only), &g["summary"], "summary");
    for row in s["rows"].as_array().unwrap() {
        let keys = ["ms2", "tp_name", "status", "net_change", "modification"];
        same_shape(&without(row, &keys), &g["row"], "row");
        for k in row["kinetics"].as_array().unwrap() {
            same_shape(k, &g["kinetics"], "kinetics");
        }
    }
    let rows = s["rows"].as_array().unwrap();
    assert_eq!(rows[0]["label"], "progenitore");
    assert_eq!(rows[0]["id"], 0);
    let found = |m: f64, rt: f64| {
        rows.iter().any(|r| {
            matches!(
                r["label"].as_str(),
                Some("progenitore" | "forte" | "possibile")
            ) && (r["mz"].as_f64().unwrap() - m).abs() <= 0.35
                && r["ref_rt"].as_f64().is_some_and(|x| (x - rt).abs() <= 0.25)
        })
    };
    let want = g["python_rows"].as_array().unwrap();
    let hit = want
        .iter()
        .filter(|w| found(w["mz"].as_f64().unwrap(), w["ref_rt"].as_f64().unwrap()))
        .count();
    assert!(
        hit as f64 >= 0.95 * want.len() as f64,
        "{hit} of {} rows of the Python found: {rows:#?}",
        want.len()
    );
    for r in rows
        .iter()
        .filter(|r| matches!(r["label"].as_str(), Some("forte" | "possibile")))
    {
        let id = r["id"].as_i64().unwrap();
        let det: Value =
            serde_json::from_str(&detail(&d, id, |k| Ok(open(&idxs[k]))).unwrap()).unwrap();
        let keys = ["specificity", "ms2_links", "ms2_rep", "ms2_text", "candidates"];
        same_shape(&without(&det, &keys), &g["detail"], "detail");
        own_candidate_first(&det);
        assert!(
            det["criteria"]
                .as_array()
                .unwrap()
                .iter()
                .any(|c| c["status"] == "pass"),
            "{id} without proofs"
        );
        for t in det["traces"].as_array().unwrap() {
            same_shape(t, &g["trace"], "trace");
        }
        let sp = &det["specificity"];
        assert_eq!(sp["neighbours"].as_array().map(Vec::len), Some(4), "{id}");
        assert_eq!(sp["decoys"].as_array().map(Vec::len), Some(2), "{id}");
        assert!(det["criteria"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["name"] == "persistent"));
    }
}

#[test]
fn isotope_and_offset() {
    let (files, idxs) = series();
    let d = dig(&files, &idxs);
    let o = d
        .offset
        .as_ref()
        .expect("the offset measured on the parent");
    assert!(
        o.applied && (o.offset - SHIFT).abs() < 0.1,
        "offset {}",
        o.offset
    );
    let strong = |e: &&lr::Entry| matches!(e.label, "forte" | "possibile");
    assert!(
        !d.entries
            .iter()
            .filter(strong)
            .any(|e| (e.mz_x - (334.20 + SHIFT)).abs() <= 0.35),
        "the 13C peak of 333.20 is not an ion of its own"
    );
    assert!(
        !d.entries
            .iter()
            .filter(strong)
            .any(|e| (e.mz_x - (311.10 + SHIFT)).abs() <= 0.35),
        "an ion of the blank"
    );
}

#[test]
fn deterministic() {
    let (files, idxs) = series();
    let (a, b) = (dig(&files, &idxs), dig(&files, &idxs));
    assert_eq!(summary(&a, "{}"), summary(&b, "{}"));
}

/// An EPI file: product-ion scans of `precursor` every 0.02 min around `rt`, fragments 17, 44 and 61 Da under it (the first two shared with a
/// product at the same precursor), as the 3200 writes them (`precursor` = the selected ion).
fn ms2_index(precursor: f64, rt: f64) -> Vec<u8> {
    let mut w = MzmlWriter::new(
        Vec::new(),
        &Header {
            run: "lr_epi".into(),
            ..Default::default()
        },
    )
    .unwrap();
    let n = 21;
    w.open_spectra(n).unwrap();
    for i in 0..n {
        let frag = [
            (precursor - 17.0, 900.0f32),
            (precursor - 44.0, 500.0),
            (precursor - 61.0, 120.0),
        ];
        w.spectrum(&Spectrum {
            index: i,
            id: format!("sample=1 period=1 cycle={} experiment=2", i + 1),
            level: 2,
            rt: rt - 0.2 + 0.02 * i as f64,
            polarity: 1,
            profile: false,
            filter: "+ EPI".into(),
            precursor: Some(precursor),
            mz: frag.iter().map(|f| f.0).collect(),
            intensity: frag.iter().map(|f| f.1).collect(),
            wide: None,
        })
        .unwrap();
    }
    w.close_spectra().unwrap();
    let mzml = w.finish().unwrap();
    let size = mzml.len() as u64;
    let mut src = mzml.clone();
    let p = prepare(&mut src, size, "lr_epi.mzML").unwrap();
    build_mzml_to_vec(&p.table, &mzml, 64 << 20).unwrap().0
}

#[test]
fn ms2_files_give_the_fragments_and_the_transitions() {
    let (files, idxs) = series();
    let plain = dig(&files, &idxs);
    let parent = &plain.entries[0];
    let epi = ms2_index(parent.mz_x, parent.ref_rt.expect("RT of the parent"));
    let with = files.replace(
        ']',
        r#",{"name": "lr_EPI.mzML", "time": null, "type": "sample", "kind": "ms2"}]"#,
    );
    let plan: Value = serde_json::from_str(&lr::plan::plan_json(&with, PARENT).unwrap()).unwrap();
    assert_eq!(plan["rust"], true);
    assert_eq!(plan["ms2"][0], "lr_EPI.mzML");
    assert_eq!(plan["lc"].as_array().unwrap().len(), idxs.len());
    let d = linked(dig(&with, &idxs), &idxs, &[&epi]);
    let n = idxs.len();
    let get = |id: i64| -> Value {
        let s = detail(&d, id, |k| {
            Ok(if k < n { open(&idxs[k]) } else { open(&epi) })
        })
        .unwrap();
        serde_json::from_str(&s).unwrap()
    };
    let p = get(parent.id);
    let m = &p["ms2"];
    assert!(m["scans"].as_i64().unwrap() >= 15, "{m}");
    assert!((m["precursor"].as_f64().unwrap() - parent.mz_x).abs() < 0.01);
    let frags = m["fragments"].as_array().unwrap();
    assert_eq!(frags.len(), 3);
    assert_eq!(frags[0]["rel"], 100.0);
    assert!((frags[0]["mz"].as_f64().unwrap() - (parent.mz_x - 17.0)).abs() < 0.01);
    let t = p["transitions"].as_array().unwrap();
    assert_eq!(t.len(), 3);
    assert!((t[0]["Q3_consigliato"].as_f64().unwrap() - (parent.mz_x - 17.0 - SHIFT)).abs() < 0.1);
    let other = d.entries.iter().find(|e| e.steps > 0).expect("a product");
    assert_eq!(get(other.id)["ms2"]["scans"], 0);
}

#[test]
fn the_tree_comes_from_the_ms2_of_the_parent() {
    let (files, idxs) = series();
    let plain = dig(&files, &idxs);
    let parent = &plain.entries[0];
    let epi = ms2_index(parent.mz_x, parent.ref_rt.expect("RT of the parent"));
    let with = files.replace(
        ']',
        r#",{"name": "lr_EPI.mzML", "time": null, "type": "sample", "kind": "ms2"}]"#,
    );
    let d = linked(dig(&with, &idxs), &idxs, &[&epi]);
    let n = idxs.len();
    let mut op = |k: usize| Ok(if k < n { open(&idxs[k]) } else { open(&epi) });
    let none: Value =
        serde_json::from_str(&lr::tree::frag_tree(&plain, &mut |k| Ok(open(&idxs[k]))).unwrap())
            .unwrap();
    assert_eq!(none["ok"], false);
    let t: Value = serde_json::from_str(&lr::tree::frag_tree(&d, &mut op).unwrap()).unwrap();
    assert_eq!(t["ok"], true, "{t}");
    assert_eq!(t["mode"], "lr");
    let other = d.entries.iter().find(|e| e.steps > 0).expect("a product");
    let c: Value =
        serde_json::from_str(&lr::tree::frag_tree_compare(&d, other.id, &mut op).unwrap()).unwrap();
    assert!(c.get("ok").is_some(), "{c}");
}

#[test]
fn mrm_files_are_listed_by_the_plan_and_keep_it_in_rust() {
    let (files, idxs) = series();
    let with = files.replace(
        ']',
        r#",{"name": "lr_MRM.mzML", "time": null, "type": "standard", "kind": "mrm"}]"#,
    );
    let plan: Value = serde_json::from_str(&lr::plan::plan_json(&with, PARENT).unwrap()).unwrap();
    assert_eq!(plan["rust"], true);
    assert_eq!(plan["mrm"][0], "lr_MRM.mzML");
    assert_eq!(plan["lc"].as_array().unwrap().len(), idxs.len());
}

/// The candidates of the trace: the row's own first, with its neutral and [M+H]+ formula and the calculated m/z of the row.
fn own_candidate_first(det: &Value) {
    let c = det["candidates"].as_array().unwrap();
    assert!(!c.is_empty() && c[0]["own"] == true, "{c:?}");
    assert_eq!(c[0]["formula"], det["formula"]);
    assert_eq!(c[0]["mz"], det["mz"]);
    assert_eq!(c.len(), 1 + det["alternatives"].as_array().unwrap().len());
    if let (Some(f), Some(i)) = (c[0]["formula"].as_str(), c[0]["ion_formula"].as_str()) {
        let h = |t: &str| {
            let k = t.find('H').unwrap() + 1;
            t[k..]
                .chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>()
                .parse::<u32>()
                .unwrap_or(1)
        };
        assert_eq!(h(i), h(f) + 1, "{f} -> {i}");
    }
}
