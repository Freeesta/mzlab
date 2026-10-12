//! Hypotheses of the targeted pass: the rules file `TP_Mine/data/regole_tp.json` is the table of mzFinder, the hypotheses of the rules have the masses
//! of `tools/feat_ipotesi.py` (golden file `ipotesi_regole.json`), the order and the ids are stable (same SHA-256 twice), and the other sources work.
//! The checks that need python3 are skipped without it (`MZLAB_GOLDEN_STRICT=1` fails instead).

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_feat::hypo::*;
use mzlab_feat::suspects::Suspects;
use serde_json::Value;
use sha2::{Digest, Sha256};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn rules() -> Rules {
    Rules::parse(&std::fs::read(root().join("TP_Mine/data/regole_tp.json")).unwrap()).unwrap()
}

fn none<'a>() -> Inputs<'a> {
    Inputs {
        parent: "C12H17ClN2O3S",
        suspects: None,
        families: &[],
        imported: &[],
        fragments: &[],
    }
}

fn sha(r: &Registry) -> String {
    Sha256::digest(r.to_json().as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[test]
fn rule_hypotheses_match_the_python_list() {
    let g: Value = serde_json::from_slice(
        &std::fs::read(root().join("tests/golden/feat/ipotesi_regole.json")).unwrap(),
    )
    .unwrap();
    let rules = rules();
    let p = HypoParams::default();
    for parent in g["parents"].as_array().unwrap() {
        let formula = parent["formula"].as_str().unwrap();
        let reg = generate(
            &rules,
            &Inputs {
                parent: formula,
                ..none()
            },
            &p,
        )
        .unwrap();
        let want = parent["hypotheses"].as_array().unwrap();
        assert_eq!(reg.len(), want.len(), "{formula}");
        for (h, w) in reg.hyps.iter().zip(want) {
            assert_eq!(h.formula, w["formula"].as_str().unwrap(), "{formula}");
            assert_eq!(
                h.steps as u64,
                w["steps"].as_u64().unwrap(),
                "{formula} {}",
                h.formula
            );
            assert!((h.neutral - w["mass"].as_f64().unwrap()).abs() < 1e-6);
            assert!((h.mz - w["mz"].as_f64().unwrap()).abs() < 1e-6);
            assert_eq!(h.sources, SRC_RULE);
        }
    }
}

#[test]
fn rules_file_is_the_table_of_mzfinder() {
    let ok = Command::new("python3")
        .arg(root().join("tools/genera_regole_tp.py"))
        .arg("--controlla")
        .status()
        .map(|s| s.success());
    match ok {
        Ok(true) => {}
        Ok(false)
            if std::env::var("MZLAB_GOLDEN_STRICT").is_err()
                && Command::new("python3")
                    .arg("-c")
                    .arg("import numpy")
                    .status()
                    .map(|s| !s.success())
                    .unwrap_or(true) => {}
        other => panic!("regole_tp.json: {other:?}"),
    }
}

#[test]
fn two_runs_give_the_same_registry() {
    let rules = rules();
    let fam = [FamilyBase {
        mz: 330.1,
        rt: 5.0,
        polarity: 1,
    }];
    let imp = [Imported {
        name: "lista 1".into(),
        formula: "C6H12O6".into(),
    }];
    let inp = Inputs {
        families: &fam,
        imported: &imp,
        ..none()
    };
    let a = generate(&rules, &inp, &HypoParams::default()).unwrap();
    let b = generate(&rules, &inp, &HypoParams::default()).unwrap();
    assert_eq!(sha(&a), sha(&b));
    assert!(a.hyps.windows(2).all(|w| w[0].mz <= w[1].mz));
    assert!(a.hyps.iter().enumerate().all(|(i, h)| h.id as usize == i));
    assert_eq!(a.by_source()["famiglia"], 5);
    assert_eq!(a.by_source()["importata"], 1);
}

#[test]
fn unknown_version_and_format_are_refused() {
    let e = Rules::parse(br#"{"format":"regole_tp","version":2,"regole":[]}"#)
        .err()
        .unwrap();
    assert_eq!(e.key, "error.hypo.rulesVersion");
    assert_eq!(
        Rules::parse(br#"{"format":"altro","version":1}"#)
            .err()
            .unwrap()
            .key,
        "error.hypo.rulesFormat"
    );
    let bad = br#"{"format":"regole_tp","version":1,"regole":[{"id":"x","delta_formula":"+Qq","passi_max":1,"livello_previsione":4,"fonte":"transformation_table","direzione_rt":"libera"}]}"#;
    assert_eq!(Rules::parse(bad).err().unwrap().key, "error.hypo.rule");
}

#[test]
fn class_rules_need_the_group_and_dealkylation_is_there() {
    let rules = rules();
    let reg = generate(
        &rules,
        &Inputs {
            parent: "C9H13N3O2",
            ..none()
        },
        &HypoParams::default(),
    )
    .unwrap();
    assert!(reg
        .hyps
        .iter()
        .all(|h| !h.label.contains("dechlorination") && !h.label.contains("SO2")));
    let reg = generate(&rules, &none(), &HypoParams::default()).unwrap();
    let has = |l: &str| reg.hyps.iter().any(|h| h.label == l && h.steps == 1);
    for l in [
        "hydroxylation",
        "demethylation",
        "loss_of_c2h4",
        "loss_of_c4h8",
        "reductive_dechlorination",
        "hydrolytic_dechlorination",
        "dihydroxylation",
    ] {
        assert!(has(l), "{l}");
    }
}

#[test]
fn family_members_and_other_polarity() {
    let fam = [
        FamilyBase {
            mz: 100.0,
            rt: 3.0,
            polarity: 1,
        },
        FamilyBase {
            mz: 98.0,
            rt: 4.0,
            polarity: -1,
        },
    ];
    let r = generate(
        &rules(),
        &Inputs {
            parent: "C12H17ClN2O3S",
            families: &fam,
            ..none()
        },
        &HypoParams {
            other_polarity: true,
            ..Default::default()
        },
    )
    .unwrap();
    let f: Vec<_> = r.hyps.iter().filter(|h| h.sources == SRC_FAMILY).collect();
    assert_eq!(f.len(), 6);
    let na = f.iter().find(|h| h.adduct == "[M+Na]+").unwrap();
    assert!((na.mz - (100.0 - 1.007276 + 22.989218)).abs() < 1e-3);
    assert_eq!(na.rt_ref, Some(3.0));
    assert!(f
        .iter()
        .any(|h| h.polarity == 1 && h.adduct == "[M+H]+" && (h.mz - 101.003355).abs() < 1e-9));
}

#[test]
fn fragments_must_come_from_the_parent() {
    let rules = rules();
    let ok = generate(
        &rules,
        &Inputs {
            fragments: &["C6H9N2O".to_string()],
            ..none()
        },
        &HypoParams::default(),
    )
    .unwrap();
    assert_eq!(
        ok.hyps.iter().filter(|h| h.sources == SRC_CLEAVAGE).count(),
        1
    );
    let e = generate(
        &rules,
        &Inputs {
            fragments: &["C20H9".to_string()],
            ..none()
        },
        &HypoParams::default(),
    )
    .err()
    .unwrap();
    assert_eq!(e.key, "error.hypo.fragment");
}

fn pad8(v: &mut Vec<u8>) {
    while !v.len().is_multiple_of(8) {
        v.push(0);
    }
}

/// A `.mzsus` with the given (mass, formula, sources, pubmed) rows, ordered by mass.
fn mzsus(rows: &[(f64, &str, u8, u16)]) -> Vec<u8> {
    let (n, nf) = (rows.len(), rows.len());
    let mut v = b"MZSU".to_vec();
    for x in [1u16, 1] {
        v.extend_from_slice(&x.to_le_bytes());
    }
    for x in [n as u32, nf as u32, 0, 2, 0, 0] {
        v.extend_from_slice(&x.to_le_bytes());
    }
    let sec = |b: Vec<u8>, v: &mut Vec<u8>| {
        v.extend_from_slice(&b);
        pad8(v);
    };
    sec(b"{}".to_vec(), &mut v);
    sec(
        rows.iter().flat_map(|r| r.0.to_le_bytes()).collect(),
        &mut v,
    );
    sec(
        (0..n)
            .flat_map(|i| (0..14).map(move |k| b'A' + ((i + k) % 26) as u8))
            .collect(),
        &mut v,
    );
    sec(
        (0..n as u32).flat_map(|i| i.to_le_bytes()).collect(),
        &mut v,
    );
    sec(rows.iter().map(|r| r.2).collect(), &mut v);
    sec(
        rows.iter().flat_map(|r| r.3.to_le_bytes()).collect(),
        &mut v,
    );
    sec(vec![0u8; 2 * n], &mut v);
    sec(vec![], &mut v);
    let mut off = vec![0u32];
    for r in rows {
        off.push(off.last().unwrap() + r.1.len() as u32);
    }
    sec(off.iter().flat_map(|x| x.to_le_bytes()).collect(), &mut v);
    sec(rows.iter().flat_map(|r| r.1.bytes()).collect(), &mut v);
    v
}

#[test]
fn suspects_are_filtered_cut_and_labelled() {
    let rows = [
        (122.0, "C6H6O3", 4u8, 3u16), // oxygen not in the parent's reach? C6 O3: derivable (O <= 3+3)
        (180.0, "C6H12O6", 4, 50),    // H 12 <= 17, O 6 <= 6
        (200.0, "C10H20Br", 4, 9),    // Br the parent lacks
        (250.0, "C11H14N2O2", 1, 90), // no reaction bit
        (300.0, "C12H17ClN2O5S", 4, 20), // derivable
    ];
    let s = Suspects::open(&mzsus(&rows)).unwrap();
    let p = HypoParams {
        max_suspects: 2,
        ..Default::default()
    };
    let reg = generate(
        &rules(),
        &Inputs {
            suspects: Some(&s),
            ..none()
        },
        &p,
    )
    .unwrap();
    let sus: Vec<_> = reg
        .hyps
        .iter()
        .filter(|h| h.sources & SRC_SUSPECT != 0)
        .collect();
    assert_eq!(sus.len(), 2);
    assert_eq!(reg.suspects_cut, 1);
    assert!(
        sus.iter().any(|h| h.formula == "C6H12O6")
            && sus.iter().any(|h| h.formula == "C12H17ClN2O5S")
    );
    // C12H17ClN2O5S is also +O2 of the parent: one hypothesis, two sources
    let both = reg
        .hyps
        .iter()
        .find(|h| h.formula == "C12H17ClN2O5S")
        .unwrap();
    assert_eq!(both.sources, SRC_RULE | SRC_SUSPECT);
}
