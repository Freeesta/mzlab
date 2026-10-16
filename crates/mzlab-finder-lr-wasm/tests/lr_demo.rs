//! False positives of the low-resolution Dig on the five demo full-scan files of the site (`mzlab/web/esempi/FullScan_t*.mzML`, flufenacet on TiO2):
//! a parent that is not in the files and random parents (decoys) give no strong candidate and no level 4, and the right parent keeps its products.

use mzlab_core::idxread::Index;
use mzlab_feat::legacy::lr::{self, summary::level};
use mzlab_feat_wasm::run;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::OnceLock;

mod comune;
use comune::{index_of_file, open};

const TIMES: [u32; 5] = [0, 5, 10, 15, 30];

struct Demo {
    files: String,
    idxs: Vec<Vec<u8>>,
    feats: Vec<Vec<u8>>,
    table: Vec<u8>,
}

/// The demo files indexed, detected and aligned once (the parent does not change them).
fn demo() -> &'static Demo {
    static D: OnceLock<Demo> = OnceLock::new();
    D.get_or_init(|| {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mzlab/web/esempi");
        let names: Vec<String> = TIMES.iter().map(|t| format!("FullScan_t{t}.mzML")).collect();
        let idxs: Vec<Vec<u8>> = names.iter().map(|n| index_of_file(&dir.join(n))).collect();
        let files = serde_json::to_string(
            &names
                .iter()
                .zip(TIMES)
                .map(|(n, t)| serde_json::json!({"name": n, "type": "sample", "time": t as f64, "kind": "full"}))
                .collect::<Vec<_>>(),
        )
        .unwrap();
        let r = run::recipe_for("", &mut open(&idxs[0])).unwrap();
        let feats: Vec<Vec<u8>> = idxs
            .iter()
            .zip(&names)
            .map(|(b, n)| run::detect(&mut open(b), n, &run::recipe_for("", &mut open(b)).unwrap()).unwrap())
            .collect();
        let (f, origin) = run::read_files(&feats).unwrap();
        let table = run::align(&f, &origin, &r, |k| Ok(open(&idxs[k]))).unwrap();
        Demo {
            files,
            idxs,
            feats,
            table,
        }
    })
}

fn dig(neutral: &str) -> lr::LrDig {
    dig_set(neutral, "{}")
}

fn dig_set(neutral: &str, settings: &str) -> lr::LrDig {
    let d = demo();
    let parent = format!(r#"{{"name": "parent", "neutral": "{neutral}", "adduct": "[M+H]+"}}"#);
    let plan: Value =
        serde_json::from_str(&lr::plan::plan_json(&d.files, &parent).unwrap()).unwrap();
    let r = run::recipe_for("", &mut open(&d.idxs[0])).unwrap();
    let (feats, _) = run::read_files(&d.feats).unwrap();
    let rules = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../TP_Mine/data/regole_tp.json"
    ))
    .unwrap();
    let mut ix: Vec<Index<Vec<u8>>> = d.idxs.iter().map(|b| open(b)).collect();
    let refs: Vec<_> = feats.iter().collect();
    let h = run::hypotheses(
        &mut ix,
        &refs,
        &rules,
        "",
        plan["formula"].as_str().unwrap(),
        None,
        &r,
    )
    .unwrap();
    lr::dig(
        &d.files,
        &parent,
        settings,
        &d.table,
        &h.registry.to_json(),
        &mut |k| Ok(open(&d.idxs[k])),
    )
    .unwrap()
}

/// Candidates of the transformations of the parent (unexpected ions aside) with the label `label`.
fn candidates<'a>(d: &'a lr::LrDig, label: &str) -> Vec<&'a lr::Entry> {
    d.entries
        .iter()
        .filter(|e| e.id != 0 && !e.unexpected && e.label == label)
        .collect()
}

fn shown(d: &lr::LrDig) -> usize {
    candidates(d, "forte").len() + candidates(d, "possibile").len()
}

fn level4(d: &lr::LrDig) -> Vec<(f64, Option<f64>, String)> {
    d.entries
        .iter()
        .filter(|e| level(e).0 == Some(4))
        .map(|e| (e.mz, e.ref_rt, e.name.clone()))
        .collect()
}

#[test]
fn a_parent_that_is_not_in_the_files_gives_no_product() {
    let d = dig("C15H12N2O"); // carbamazepine: its [M+H]+ grows in the series instead of falling
    assert!(!d.parent_ok);
    assert_eq!(d.entries[0].id, 0);
    assert_eq!(shown(&d), 0, "{:?}", candidates(&d, "forte"));
    // the candidates with a peak stay in the table, in the weak band
    assert!(candidates(&d, "debole").iter().all(|e| e
        .criteria
        .iter()
        .any(|c| c.name == lr::PARENT_OK && c.status == "fail")));
}

#[test]
fn random_parents_give_no_strong_candidate() {
    // formulas whose nominal [M+H]+ falls in the series (the parent check passes): only chance can match their transformations
    for f in [
        "C8H5N4", "C14H9N", "C9H12O4", "C11H6N4O", "C21H21O4", "C9H7N4",
    ] {
        let d = dig(f);
        assert!(level4(&d).is_empty(), "{f}: {:?}", level4(&d));
        assert!(
            candidates(&d, "forte").is_empty(),
            "{f}: {:?}",
            candidates(&d, "forte")
        );
    }
}

#[test]
fn the_right_parent_keeps_its_products() {
    let d = dig("C14H13F4N3O2S"); // flufenacet
    assert!(d.parent_ok);
    let one = d.chance.iter().find(|c| c.steps == 1).unwrap();
    assert!(one.above(), "{one:?}");
    let l4 = level4(&d);
    let has = |mz: f64, rt: f64| {
        l4.iter()
            .any(|x| (x.0 - mz).abs() < 0.5 && x.1.is_some_and(|t| (t - rt).abs() < 0.2))
    };
    assert!(
        has(362.06, 13.02) && has(378.05, 13.57) && has(396.06, 13.41),
        "{l4:?}"
    );
    assert!(l4.len() <= 6, "{l4:?}");
    for e in candidates(&d, "forte") {
        let peak = e.criteria.iter().find(|c| c.name == lr::peakchk::PEAK);
        assert!(
            peak.is_some_and(|c| c.status == "pass"),
            "{} {:?}",
            e.mz,
            e.criteria
        );
    }
}

/// `cargo test --release -p mzlab-finder-lr-wasm --test lr_demo -- --ignored --nocapture`: candidates shown (strong + possible) and level 4 by threshold.
#[test]
#[ignore]
fn table_threshold_noise() {
    let sets = [
        (
            "no filter",
            r#"{"min_snr": 0, "max_fwhm_s": 1e9, "min_points": 0, "dead_factor": 0, "parent_decay": false}"#,
        ),
        ("S/N 3", r#"{"min_snr": 3}"#),
        ("S/N 5", r#"{"min_snr": 5}"#),
        ("S/N 10", r#"{"min_snr": 10}"#),
    ];
    let decoys = [
        "C8H5N4", "C14H9N", "C9H12O4", "C11H6N4O", "C21H21O4", "C9H7N4",
    ];
    println!("setting | carbamazepine shown/strong | decoys shown/strong/L4 | flufenacet shown/strong/L4");
    for (n, s) in sets {
        let c = dig_set("C15H12N2O", s);
        let (mut ds, mut df, mut d4) = (0, 0, 0);
        for f in decoys {
            let d = dig_set(f, s);
            ds += shown(&d);
            df += candidates(&d, "forte").len();
            d4 += level4(&d).len();
        }
        let f = dig_set("C14H13F4N3O2S", s);
        println!(
            "{n} | {}/{} | {ds}/{df}/{d4} | {}/{}/{}",
            shown(&c),
            candidates(&c, "forte").len(),
            shown(&f),
            candidates(&f, "forte").len(),
            level4(&f).len()
        );
    }
}
