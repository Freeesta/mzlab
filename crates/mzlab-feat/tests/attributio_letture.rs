#![cfg(feature = "attributio")]
//! Parity of the Attributio readers with the golden data of the reference tool (`tools/attributio_riferimento.py`, Attributio 1.12.0) on the synthetic
//! campaigns, the hand cases, and a read of the real campaigns when `MZLAB_DATI` points at the private data (skipped without).

#[macro_use]
mod attributio_comune;
use attributio_comune::*;
use mzlab_feat::attributio::campaign::{Campaign, Env};
use mzlab_feat::attributio::detection::detection_table;
use mzlab_feat::attributio::io::{Entry, Io};
use mzlab_feat::attributio::readers::gnps::{
    clean_library_hit_name, load_gnps_library, load_gnps_network,
};
use mzlab_feat::attributio::readers::mzmine::{bracket_adduct, load_mzmine_library};
use mzlab_feat::attributio::readers::sirius::load_sirius_tables;
use mzlab_feat::attributio::sorbents::{blank_abundance, remap_ids};
use mzlab_feat::attributio::sources::{
    load_feature_table, load_sources, merged_table_path, GNPS_MASS_TOL_PPM,
};
use mzlab_feat::attributio::toml::{self, Value as T};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

#[path = "../../../tools/dati_percorsi.rs"]
mod dati_percorsi;

// ------------------------------------------------------------------------------------------------------------------ the campaign
#[test]
fn campaign_matches_the_golden_data() {
    let fx = fixture("campagna_sintetica");
    let (c, g) = (&fx.campaign, golden("sintetica", "campagna.json"));
    assert_eq!(json!(c.id), g["id"]);
    assert_eq!(json!(c.name), g["name"]);
    assert_eq!(json!(c.instrument), g["instrument"]);
    assert_eq!(json!(c.sorbents), g["sorbents"]);
    let sites: serde_json::Map<String, Value> = c
        .sites
        .iter()
        .map(|(k, t)| (k.clone(), toml_json(&T::Table(t.clone()))))
        .collect();
    same(&Value::Object(sites), &g["sites"], "sites");
    let prefixes: serde_json::Map<String, Value> = c
        .site_prefix
        .iter()
        .map(|(k, v)| (k.clone(), json!(v)))
        .collect();
    same(&Value::Object(prefixes), &g["site_prefix"], "site_prefix");
    let tags: serde_json::Map<String, Value> = c
        .polarity_tag
        .iter()
        .map(|(k, v)| (k.clone(), json!(v)))
        .collect();
    same(&Value::Object(tags), &g["polarity_tag"], "polarity_tag");
    let matrices: serde_json::Map<String, Value> = c
        .matrices
        .iter()
        .map(|(k, m)| {
            (
                k.clone(),
                json!({"sites": m.sites, "dir": m.dir, "label": m.label}),
            )
        })
        .collect();
    same(&Value::Object(matrices), &g["matrices"], "matrices");
    assert_eq!(json!(c.feature_decisions_csv), g["feature_decisions"]);
    with_io!(fx, io);
    for pol in ["POS", "NEG"] {
        assert_eq!(
            json!(merged_table_path(c, &io, pol).expect("merged table")),
            g["merged"][pol],
            "{pol}"
        );
    }
}

#[test]
fn readers_match_the_golden_data() {
    let fx = fixture("campagna_sintetica");
    let (c, g) = (&fx.campaign, golden("sintetica", "letture.json"));
    with_io!(fx, io);
    let mut warnings = Vec::new();
    for pol in ["POS", "NEG"] {
        let (t, twins) = load_feature_table(c, &io, pol, &mut warnings).expect("table");
        assert!(twins.is_empty());
        let want = &g["mzmine_tabella"][pol];
        assert_eq!(
            want["righe"].as_u64().expect("rows") as usize,
            t.len(),
            "{pol} rows"
        );
        assert_eq!(json!(t.columns()), want["colonne"], "{pol} columns");
        same(
            &frame_rows(&t, &["id", "mz", "rt"], 5),
            &want["prime5"],
            &format!("{pol} first rows"),
        );
        table_block(
            &load_mzmine_library(c, &io, &t, pol).expect("mzmine"),
            &g["mzmine_libreria"][pol],
            &format!("mzmine {pol}"),
        );
        table_block(
            &load_gnps_library(c, &io, pol, 15.0).expect("gnps"),
            &g["gnps_libreria"][pol],
            &format!("gnps {pol}"),
        );
        table_block(
            &load_gnps_network(c, &io, pol).expect("network"),
            &g["gnps_rete"][pol],
            &format!("network {pol}"),
        );
    }
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[test]
fn standards_library_and_decision_columns_match_the_golden_data() {
    let fx = fixture("campagna_sintetica");
    let g = golden("sintetica", "letture.json");
    let s = tables(&fx);
    let cols = [
        "name",
        "formula",
        "molecular_class",
        "comment",
        "ESI_pos",
        "ESI_neg",
        "mz_MH_pos",
        "mz_MNa_pos",
        "mz_MNH4_pos",
        "mz_M2H_pos2",
        "mz_MH_neg",
        "mz_MHCOO_neg",
        "rt",
    ];
    let lib = frame_rows(&s.standards_library, &cols, 5);
    let want: Vec<Value> = g["standard"]
        .as_array()
        .expect("standard")
        .iter()
        .map(|r| {
            let o = r.as_object().expect("row");
            Value::Object(
                cols.iter()
                    .map(|c| {
                        (
                            c.to_string(),
                            o[if *c == "formula" { "formula" } else { c }].clone(),
                        )
                    })
                    .collect(),
            )
        })
        .collect();
    same(&lib, &Value::Array(want), "standards library");
    let mut want_cols: Vec<String> = g["decisioni_colonne"]
        .as_array()
        .expect("cols")
        .iter()
        .map(|c| c.as_str().expect("str").to_string())
        .map(|c| match c.as_str() {
            "livello" => "level".into(),
            "nome" => "name".into(),
            "identita" => "identity".into(),
            "motivazione" => "justification".into(),
            _ => c,
        })
        .collect();
    want_cols.push("evidence_checklist".into());
    assert_eq!(s.decisions.columns, want_cols);
}

#[test]
fn detection_table_matches_the_golden_data() {
    let fx = fixture("campagna_sintetica");
    let s = tables(&fx);
    let g = golden("sintetica", "rilevazione.json");
    let sites: Vec<String> = fx
        .campaign
        .site_keys()
        .into_iter()
        .map(String::from)
        .collect();
    for pol in ["POS", "NEG"] {
        let rows = s
            .detection(&fx.campaign, pol, &sites)
            .expect("polarity")
            .expect("detection");
        same(&detection_json(&rows), &g[pol], pol);
    }
}

#[test]
fn sirius_tables_match_the_golden_data() {
    let fx = fixture("campagna_sintetica");
    with_io!(fx, io);
    let (t, g) = (
        load_sirius_tables(&fx.campaign, &io).expect("sirius"),
        golden("sintetica", "sirius.json"),
    );
    let nulls = |v: Value| -> Value {
        Value::Array(
            v.as_array()
                .expect("rows")
                .iter()
                .map(|r| {
                    Value::Object(
                        r.as_object()
                            .expect("row")
                            .iter()
                            .filter(|(_, v)| !v.is_null())
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect(),
                    )
                })
                .collect(),
        )
    };
    same(
        &nulls(rows_json(&t.canopus)),
        &nulls(g["canopus"].clone()),
        "canopus",
    );
    same(
        &nulls(rows_json(&t.structure)),
        &nulls(g["structure"].clone()),
        "structure",
    );
    same(
        &nulls(rows_json(&t.spectral)),
        &nulls(g["spectral"].clone()),
        "spectral",
    );
}

// ------------------------------------------------------------------------------------------------------------------ sorbents
#[test]
fn sorbent_lists_match_the_golden_data() {
    let fx = fixture("campagna_sorbenti");
    let g = golden("sorbenti", "sorbenti.json");
    let s = tables(&fx);
    assert_eq!(s.pols.len(), 1, "only POS has a table");
    assert!(
        s.warnings.iter().any(|w| w.contains("NEG")),
        "{:?}",
        s.warnings
    );
    let p = s.pol("POS").expect("POS");
    assert_eq!(json!(p.table.columns()), g["colonne"]);
    let cols = [
        "id",
        "mz",
        "rt",
        "fragment_scans",
        "blank_abundance:OA",
        "blank_abundance:EC",
        "twin_id",
    ];
    same(&frame_rows(&p.table, &cols, 99), &g["tabella"], "table");
    let twins: serde_json::Map<String, Value> = p
        .twins
        .iter()
        .map(|(a, b)| (a.to_string(), json!(b)))
        .collect();
    same(&Value::Object(twins), &g["gemelli"], "twins");
    let sites: Vec<String> = fx
        .campaign
        .site_keys()
        .into_iter()
        .map(String::from)
        .collect();
    let det = detection_table(&fx.campaign, &p.table, "POS", &sites).expect("detection");
    same(
        &detection_json(&det),
        &g["rilevazione"],
        "detection with the blank fold",
    );
    for pair in g["blank_abundance"].as_array().expect("pairs") {
        let want = pair[1].as_f64().expect("number");
        assert_eq!(
            blank_abundance(pair[0].as_str().expect("text")).expect("abundance"),
            want,
            "{}",
            pair[0]
        );
    }
}

#[test]
fn remap_ids_keeps_the_best_hit_of_a_pair_in_the_original_order() {
    let rows = vec![
        ("POS", 2, Some(0.5)),
        ("POS", 3, Some(0.9)),
        ("POS", 7, None),
        ("NEG", 3, Some(0.1)),
    ];
    let out = remap_ids(
        rows,
        &[(3, 2)],
        "POS",
        |r| (r.0, r.1),
        |r, id| r.1 = id,
        |r| vec![r.2],
        false,
    );
    assert_eq!(
        out,
        vec![
            ("POS", 2, Some(0.9)),
            ("POS", 7, None),
            ("NEG", 3, Some(0.1))
        ]
    );
}

// ------------------------------------------------------------------------------------------------------------------ hand cases
#[test]
fn a_toml_construct_that_is_not_supported_names_its_line() {
    for (text, line) in [
        ("[campaign]\nid = \"A\"\n\n[[sites]]\n", 4),
        ("[campaign]\nid = \"A\"\nsince = 2026-01-01\n", 3),
        ("[campaign]\nid = \"\"\"A\"\"\"\n", 2),
    ] {
        let e =
            Campaign::from_toml(text, "campaign.toml", "root", Env::default()).expect_err("error");
        assert!(
            e.msg.starts_with(&format!("campaign.toml: line {line}:")),
            "{}",
            e.msg
        );
    }
    assert!(toml::parse("[a]\nb = [1, 2.5, \"x\"]\n").is_ok());
}

#[test]
fn campaign_errors_have_the_messages_of_attributio() {
    let e = |text: &str| {
        Campaign::from_toml(text, "campaign.toml", "root", Env::default())
            .expect_err("error")
            .msg
    };
    assert_eq!(
        e("[levels]\nx = 1\n"),
        "campaign.toml [levels]: unknown keys ['x']"
    );
    assert_eq!(
        e("[sites.A]\n[matrices.m]\nsites = [\"A\", \"B\"]\n"),
        "campaign.toml: matrix 'm' lists unknown sites ['B']"
    );
    assert_eq!(
        e("[mzmine]\ncollapse_sorbent_twins = true\n"),
        "campaign.toml [mzmine]: collapse_sorbent_twins needs sorbent_table"
    );
    assert_eq!(
        e("[mzmine]\nmerged_dir = \"{nope}\"\n"),
        "campaign.toml [mzmine]: merged_dir uses the unknown field '{nope}'"
    );
    let c = Campaign::from_toml("", "campaign.toml", "folder", Env::default()).expect("defaults");
    assert_eq!(
        (
            c.id.as_str(),
            c.sorbents.clone(),
            c.twin_mz_ppm,
            c.count_2b_as_2
        ),
        (
            "folder",
            vec!["OA".to_string(), "EC".to_string()],
            5.0,
            true
        )
    );
    assert_eq!(
        c.col("X", "OA", "POS", "area"),
        "datafile:X_OA_ddaPOS.mzML:area"
    );
}

#[test]
fn library_names_are_cleaned_like_attributio() {
    for (raw, want) in [
        ("Massbank:EA255002 Caffeine", "Caffeine"),
        ("MoNA:1083201 Bentazon", "Bentazon"),
        ("Spectral Match to Atrazine from NIST14", "Atrazine"),
        ("\"Caffeine_CE15.5\"", "Caffeine"),
        ("Name|alias", "Name"),
        ("Diuron - 30.0 eV", "Diuron"),
        ("AB1234 not an accession", "AB1234 not an accession"),
        ("ReSpect:PT110640 Quercetin", "Quercetin"),
    ] {
        assert_eq!(clean_library_hit_name(raw), want, "{raw}");
    }
    assert_eq!(
        bracket_adduct(Some("M+H"), "POS").as_deref(),
        Some("[M+H]+")
    );
    assert_eq!(
        bracket_adduct(Some(" M-H "), "NEG").as_deref(),
        Some("[M-H]-")
    );
    assert_eq!(
        bracket_adduct(Some("[M+Na]+"), "POS").as_deref(),
        Some("[M+Na]+")
    );
    assert_eq!(bracket_adduct(Some("  "), "POS"), None);
    assert_eq!(GNPS_MASS_TOL_PPM, 15.0);
}

// ------------------------------------------------------------------------------------------------------------------ real data
fn find_campaigns(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    if dir.join("campaign.toml").is_file() {
        out.push(dir.to_path_buf());
        return;
    }
    if depth == 0 {
        return;
    }
    if let Ok(rd) = std::fs::read_dir(dir) {
        let mut subs: Vec<PathBuf> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.is_dir()
                    && !p
                        .file_name()
                        .is_some_and(|n| n.to_string_lossy().starts_with('.'))
            })
            .collect();
        subs.sort();
        subs.iter().for_each(|s| find_campaigns(s, depth - 1, out));
    }
}

#[test]
fn real_campaigns_are_read_without_errors() {
    let Some(base) = dati_percorsi::radice() else {
        eprintln!("SKIP attributio real campaigns: MZLAB_DATI is not set");
        return;
    };
    let mut found = Vec::new();
    find_campaigns(&base, 4, &mut found);
    if found.is_empty() {
        eprintln!(
            "SKIP attributio real campaigns: no campaign.toml under {}",
            base.display()
        );
        return;
    }
    for dir in found {
        let at = |p: &str| -> PathBuf {
            if Path::new(p).is_absolute() {
                PathBuf::from(p)
            } else {
                dir.join(p)
            }
        };
        let read = |p: &str| std::fs::read(at(p)).ok();
        let list = |d: &str| -> Vec<Entry> {
            std::fs::read_dir(at(d))
                .map(|rd| {
                    rd.flatten()
                        .map(|e| Entry {
                            name: e.file_name().to_string_lossy().into_owned(),
                            is_dir: e.path().is_dir(),
                        })
                        .collect()
                })
                .unwrap_or_default()
        };
        let io = Io::new(&read, &list);
        let text = String::from_utf8_lossy(
            &std::fs::read(dir.join("campaign.toml")).expect("campaign.toml"),
        )
        .into_owned();
        let name = dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let c = Campaign::from_toml(&text, "campaign.toml", &name, Env::default())
            .unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
        let s = load_sources(&c, &io).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
        let counts: Vec<String> = s.counts.iter().map(|(k, v)| format!("{k}={v}")).collect();
        eprintln!("REAL {}: {}", c.id, counts.join(", "));
    }
}
