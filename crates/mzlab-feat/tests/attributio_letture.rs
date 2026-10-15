#![cfg(feature = "attributio")]
//! Parity of the Attributio readers with the golden data of the reference tool (`tools/attributio_riferimento.py`, Attributio 1.12.0) on the synthetic
//! campaigns, the hand cases, and a read of the real campaigns when `MZLAB_DATI` points at the private data (skipped without).

use mzlab_feat::attributio::campaign::{Campaign, Env};
use mzlab_feat::attributio::detection::{detection_table, Detection};
use mzlab_feat::attributio::frame::{Cell, Frame, Record};
use mzlab_feat::attributio::io::{Entry, Io, MemFiles};
use mzlab_feat::attributio::readers::feature_decisions::{
    all_columns, load_feature_decisions, match_decisions, merge_decisions, write_feature_decisions,
    DecisionTable, FeatureKey, OPTIONAL_COLUMNS,
};
use mzlab_feat::attributio::readers::gnps::{
    clean_library_hit_name, load_gnps_library, load_gnps_network,
};
use mzlab_feat::attributio::readers::ms2::Spectrum;
use mzlab_feat::attributio::readers::mzmine::{bracket_adduct, load_mzmine_library};
use mzlab_feat::attributio::readers::sirius::load_sirius_tables;
use mzlab_feat::attributio::readers::standards::{
    mass_rt_candidates, FeatureRef, Similarity, StandardsSettings, Tolerances,
};
use mzlab_feat::attributio::sorbents::{blank_abundance, remap_ids};
use mzlab_feat::attributio::sources::{
    load_feature_table, load_sources, merged_table_path, GNPS_MASS_TOL_PPM,
};
use mzlab_feat::attributio::toml::{self, Value as T};
use mzlab_feat::attributio::Sources;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn golden(dir: &str, name: &str) -> Value {
    let p = root().join("tests/golden/attributio").join(dir).join(name);
    serde_json::from_str(
        &std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display())),
    )
    .expect("golden JSON")
}

fn walk(base: &Path, dir: &Path, files: &mut MemFiles) {
    for e in std::fs::read_dir(dir).expect("fixture folder") {
        let p = e.expect("entry").path();
        if p.is_dir() {
            walk(base, &p, files);
        } else {
            let rel = p
                .strip_prefix(base)
                .expect("inside")
                .to_string_lossy()
                .replace('\\', "/");
            files.add(&rel, &std::fs::read(&p).expect("fixture file"));
        }
    }
}

struct Fixture {
    files: MemFiles,
    campaign: Campaign,
}

fn fixture(name: &str) -> Fixture {
    let base = root().join("tests/data/attributio").join(name);
    let mut files = MemFiles::default();
    walk(&base, &base, &mut files);
    let text =
        String::from_utf8(files.read("campaign.toml").expect("campaign.toml")).expect("utf-8");
    let campaign =
        Campaign::from_toml(&text, "campaign.toml", name, Env::default()).expect("campaign");
    Fixture { files, campaign }
}

macro_rules! with_io {
    ($fx:expr, $io:ident) => {
        let read = |p: &str| $fx.files.read(p);
        let list = |d: &str| $fx.files.list(d);
        let $io = Io::new(&read, &list);
    };
}

// ------------------------------------------------------------------------------------------------------------------ comparison
fn same(a: &Value, b: &Value, at: &str) {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            let (x, y) = (x.as_f64().expect("f64"), y.as_f64().expect("f64"));
            assert!(
                (x - y).abs() <= 1e-9 * x.abs().max(y.abs()).max(1e-300) || x == y,
                "{at}: {x} != {y}"
            );
        }
        (Value::Array(x), Value::Array(y)) => {
            assert_eq!(x.len(), y.len(), "{at}: length");
            x.iter()
                .zip(y)
                .enumerate()
                .for_each(|(i, (p, q))| same(p, q, &format!("{at}[{i}]")));
        }
        (Value::Object(x), Value::Object(y)) => {
            let mut kx: Vec<&String> = x.keys().collect();
            let mut ky: Vec<&String> = y.keys().collect();
            kx.sort();
            ky.sort();
            assert_eq!(kx, ky, "{at}: keys");
            x.iter()
                .for_each(|(k, v)| same(v, &y[k], &format!("{at}.{k}")));
        }
        _ => assert_eq!(a, b, "{at}"),
    }
}

fn cell_json(c: &Cell) -> Value {
    match c {
        Cell::Null => Value::Null,
        Cell::Bool(b) => json!(b),
        Cell::Int(i) => json!(i),
        Cell::Num(n) => json!(n),
        Cell::Str(s) => json!(s),
    }
}

fn rows_json<R: Record>(rows: &[R]) -> Value {
    Value::Array(
        rows.iter()
            .map(|r| {
                Value::Object(
                    R::COLUMNS
                        .iter()
                        .zip(r.cells())
                        .map(|(c, v)| (c.to_string(), cell_json(&v)))
                        .collect(),
                )
            })
            .collect(),
    )
}

fn table_block<R: Record>(rows: &[R], golden: &Value, at: &str) {
    assert_eq!(
        golden["righe"].as_u64().expect("righe") as usize,
        rows.len(),
        "{at}: rows"
    );
    assert_eq!(json!(R::COLUMNS), golden["colonne"], "{at}: columns");
    let first: Vec<Value> = rows_json(rows)
        .as_array()
        .expect("array")
        .iter()
        .take(5)
        .cloned()
        .collect();
    same(&Value::Array(first), &golden["prime5"], at);
}

/// A column the way pandas types it: numbers when every present cell is one, text otherwise; missing cells as null.
fn frame_rows(f: &Frame, cols: &[&str], limit: usize) -> Value {
    let kinds: HashMap<&str, bool> = cols
        .iter()
        .map(|c| {
            let k = f.col(c).expect("column");
            (
                *c,
                (0..f.len()).all(|r| f.str_na(r, k).is_none() || f.num(r, k).is_some()),
            )
        })
        .collect();
    Value::Array(
        (0..f.len().min(limit))
            .map(|r| {
                let obj = cols.iter().map(|c| {
                    let k = f.col(c).expect("column");
                    let v = match (f.str_na(r, k), kinds[c]) {
                        (None, _) => Value::Null,
                        (Some(_), true) => json!(f.num(r, k)),
                        (Some(t), false) => json!(t),
                    };
                    (c.to_string(), v)
                });
                Value::Object(obj.collect())
            })
            .collect(),
    )
}

fn toml_json(v: &T) -> Value {
    match v {
        T::Str(s) => json!(s),
        T::Int(i) => json!(i),
        T::Float(f) => json!(f),
        T::Bool(b) => json!(b),
        T::Array(a) => Value::Array(a.iter().map(toml_json).collect()),
        T::Table(t) => Value::Object(t.0.iter().map(|(k, v)| (k.clone(), toml_json(v))).collect()),
    }
}

fn detection_json(rows: &[Detection]) -> Value {
    Value::Array(
        rows.iter()
            .map(|d| {
                let mut o = serde_json::Map::new();
                o.insert("_id".into(), json!(d.id));
                o.insert("_pol".into(), json!(d.pol));
                o.insert("mz".into(), json!(d.mz));
                o.insert("rt".into(), json!(d.rt));
                d.in_site
                    .iter()
                    .for_each(|(s, b)| drop(o.insert(format!("in_{s}"), json!(b))));
                d.any_sorbent
                    .iter()
                    .for_each(|(s, b)| drop(o.insert(format!("any_{s}"), json!(b))));
                Value::Object(o)
            })
            .collect(),
    )
}

fn tables(fx: &Fixture) -> Sources {
    with_io!(fx, io);
    load_sources(&fx.campaign, &io).expect("sources")
}

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

// ------------------------------------------------------------------------------------------------------------------ standards
fn clean_spectrum(s: &Spectrum) -> (Vec<f64>, Vec<f64>) {
    let kept: Vec<(f64, f64)> =
        s.mz.iter()
            .copied()
            .zip(s.intensity.iter().copied())
            .filter(|(m, _)| *m < s.precursor - 0.5)
            .collect();
    let top = kept.iter().map(|p| p.1).fold(f64::MIN, f64::max);
    kept.into_iter()
        .filter(|p| p.1 >= 0.01 * top)
        .map(|p| (p.0, p.1))
        .unzip()
}

/// The greedy cosine of the GNPS convention, as the reference tool computes it (the real one belongs to the review matching).
fn similarity(a: &Spectrum, b: &Spectrum, tol: f64) -> Similarity {
    let (ma, ia) = clean_spectrum(a);
    let (mb, ib) = clean_spectrum(b);
    let zero = Similarity {
        cosine: 0.0,
        reverse: 0.0,
        matched: 0,
    };
    if ma.is_empty() || mb.is_empty() {
        return zero;
    }
    let (wa, wb): (Vec<f64>, Vec<f64>) = (
        ia.iter().map(|v| v.sqrt()).collect(),
        ib.iter().map(|v| v.sqrt()).collect(),
    );
    let mut pairs = Vec::new();
    for (i, m) in ma.iter().enumerate() {
        for (j, n) in mb.iter().enumerate() {
            if (n - m).abs() <= tol {
                pairs.push((wa[i] * wb[j], i, j));
            }
        }
    }
    pairs.sort_by(|x, y| y.partial_cmp(x).expect("order"));
    let (mut ua, mut ub, mut sum) = (Vec::new(), Vec::new(), 0.0);
    for (p, i, j) in pairs {
        if !ua.contains(&i) && !ub.contains(&j) {
            ua.push(i);
            ub.push(j);
            sum += p;
        }
    }
    if ua.is_empty() {
        return zero;
    }
    let norm = |w: &[f64]| w.iter().map(|v| v * v).sum::<f64>().sqrt();
    let matched_a = ua.iter().map(|&i| wa[i] * wa[i]).sum::<f64>().sqrt();
    Similarity {
        cosine: sum / (norm(&wa) * norm(&wb)),
        reverse: sum / (matched_a * norm(&wb)),
        matched: ua.len(),
    }
}

fn compare(
    sample: &Spectrum,
    lib: &mzlab_feat::attributio::readers::ms2::RefLibrary,
    name: &str,
    ppm: f64,
    tol: f64,
) -> Option<Similarity> {
    let refs = lib.get(name.trim())?.iter().filter(|r| {
        r.precursor.is_finite()
            && sample.precursor.is_finite()
            && (sample.precursor - r.precursor).abs() / r.precursor * 1e6 <= ppm
    });
    refs.map(|r| similarity(sample, r, tol)).max_by(|x, y| {
        (x.cosine, x.reverse, x.matched)
            .partial_cmp(&(y.cosine, y.reverse, y.matched))
            .expect("order")
    })
}

#[test]
fn standards_match_the_golden_data() {
    let fx = fixture("campagna_sintetica");
    let mut s = tables(&fx);
    let g = golden("sintetica", "standard.json");
    let set = StandardsSettings::from_campaign(&fx.campaign).expect("standards");
    for pol in ["POS", "NEG"] {
        let p = s.pol(pol).expect("polarity");
        let features: Vec<FeatureRef> = p
            .feature_keys()
            .iter()
            .map(|k| FeatureRef {
                id: k.id,
                mz: k.mz,
                rt: k.rt,
            })
            .collect();
        let i = StandardsSettings::index(pol);
        let tol = Tolerances {
            mass_tol_ppm: set.mass_tol_ppm,
            rt_tol_min: set.rt_tol_min,
            rt_column: &set.rt_column[i],
            adducts: Some(&set.adducts[i]),
        };
        let cand =
            mass_rt_candidates(&features, pol, &s.standards_library, &tol).expect("candidates");
        let got: Vec<Value> = cand
            .iter()
            .map(|c| json!({"_id": c.id, "_pol": c.pol, "std_adduct": c.adduct, "std_category": c.category, "std_drt": c.drt, "std_formula": c.formula, "std_name": c.name, "std_ppm": c.ppm}))
            .collect();
        same(
            &Value::Array(got),
            &g["candidati"][pol],
            &format!("candidates {pol}"),
        );
        let mut fs: Vec<Value> = p
            .feature_spectra
            .iter()
            .map(|(k, v)| json!([k, v.mz.len(), v.precursor]))
            .collect();
        fs.sort_by_key(|v| v[0].as_i64());
        same(
            &Value::Array(fs),
            &g["spettri"][pol]["feature"],
            &format!("feature spectra {pol}"),
        );
        let mut ls: Vec<Value> = p
            .library_spectra
            .iter()
            .map(|(k, v)| json!([k, v.len()]))
            .collect();
        ls.sort_by_key(|v| v[0].as_str().map(String::from));
        same(
            &Value::Array(ls),
            &g["spettri"][pol]["libreria"],
            &format!("library spectra {pol}"),
        );
        let m = s.match_standards(pol, &compare).expect("matches");
        same(
            &rows_json(&m),
            &g["abbinamenti"][pol],
            &format!("matches {pol}"),
        );
    }
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

// ------------------------------------------------------------------------------------------------------------------ decisions
#[test]
fn decisions_match_the_golden_data() {
    let fx = fixture("campagna_sintetica");
    let s = tables(&fx);
    let g = golden("sintetica", "decisioni.json");
    let want = g["decisioni"].as_array().expect("rows");
    assert_eq!(s.decisions.len(), want.len());
    for (r, w) in want.iter().enumerate() {
        for (k, v) in w.as_object().expect("row") {
            assert_eq!(
                s.decisions.get(r, k),
                v.as_str().expect("text"),
                "row {r} {k}"
            );
        }
        assert_eq!(s.decisions.get(r, "evidence_checklist"), "");
    }
    let order = golden("sintetica", "livelli.json");
    let keys: Vec<FeatureKey> = order["features"]
        .as_array()
        .expect("features")
        .iter()
        .map(|f| {
            let (pol, id) = (
                f["_pol"].as_str().expect("pol"),
                f["_id"].as_i64().expect("id"),
            );
            s.pol(pol)
                .expect("pol")
                .feature_keys()
                .into_iter()
                .find(|k| k.id == id)
                .expect("feature")
        })
        .collect();
    let mut warnings = Vec::new();
    let matched = match_decisions(&keys, &s.decisions, None, &mut warnings);
    assert_eq!(
        matched.len(),
        g["abbinate"].as_array().expect("matched").len()
    );
    for (m, w) in matched
        .iter()
        .zip(g["abbinate"].as_array().expect("matched"))
    {
        assert_eq!(m.row as i64, w["_row"].as_i64().expect("row"));
        for (k, got) in [
            ("livello", &m.livello),
            ("nome", &m.nome),
            ("motivazione", &m.motivazione),
            ("group", &m.group),
        ] {
            assert_eq!(got, w[k].as_str().expect("text"), "{k}");
        }
    }
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].contains("POS 99 Nothing"), "{}", warnings[0]);
}

fn table(rows: &[&[(&str, &str)]]) -> DecisionTable {
    let mut t = DecisionTable::empty();
    for r in rows {
        t.rows.push(
            t.columns
                .iter()
                .map(|c| {
                    r.iter()
                        .find(|(k, _)| k == c)
                        .map_or("", |(_, v)| *v)
                        .to_string()
                })
                .collect(),
        );
    }
    t
}

#[test]
fn merge_replaces_the_row_of_the_same_day_and_reviewer_and_adds_the_others() {
    let day = |level: &'static str,
               on: &'static str,
               by: &'static str|
     -> Vec<(&'static str, &'static str)> {
        vec![
            ("pol", "POS"),
            ("feature_id", "5"),
            ("mz", "250.1234"),
            ("rt", "6.00"),
            ("livello", level),
            ("decided_on", on),
            ("decided_by", by),
            ("motivazione", "x"),
        ]
    };
    let existing = table(&[&day("2b", "2026-09-24", "ann")]);
    let same_day = table(&[&day("3", "2026-09-24", "ann")]);
    let (merged, stats) = merge_decisions(&existing, &same_day);
    assert_eq!((stats.replaced, stats.added, stats.unchanged), (1, 0, 0));
    assert_eq!((merged.len(), merged.get(0, "livello")), (1, "3"));
    let (merged, stats) = merge_decisions(&existing, &table(&[&day("3", "2026-09-30", "ann")]));
    assert_eq!((stats.replaced, stats.added, stats.unchanged), (0, 1, 0));
    assert_eq!(
        (
            merged.len(),
            merged.get(0, "livello"),
            merged.get(1, "livello")
        ),
        (2, "2b", "3")
    );
    let (merged, stats) = merge_decisions(&existing, &table(&[&day("3", "2026-09-24", "bob")]));
    assert_eq!(
        (stats.added, merged.len()),
        (1, 2),
        "another reviewer adds a row"
    );
    let (merged, stats) = merge_decisions(&existing, &table(&[&day("2b", "2026-10-01", "bob")]));
    assert_eq!(
        (stats.unchanged, merged.len()),
        (1, 1),
        "the same decision writes nothing"
    );
    let moved = [
        ("pol", "POS"),
        ("feature_id", "5"),
        ("mz", "250.1230"),
        ("rt", "6.02"),
        ("livello", "3"),
        ("decided_on", "2026-10-02"),
        ("decided_by", "ann"),
        ("motivazione", "x"),
    ];
    let (merged, stats) = merge_decisions(&existing, &table(&[&moved]));
    assert_eq!(
        (stats.added, merged.len()),
        (1, 2),
        "a feature within the tolerances is the same feature"
    );
    let other = [
        ("pol", "POS"),
        ("feature_id", "9"),
        ("mz", "400.0"),
        ("rt", "7.00"),
        ("livello", "3"),
        ("decided_on", "2026-10-02"),
        ("decided_by", "ann"),
    ];
    let (merged, _) = merge_decisions(&existing, &table(&[&other]));
    assert_eq!(merged.len(), 2);
}

#[test]
fn a_file_without_the_checklist_is_read_and_written_back_with_the_empty_column_last() {
    let text = "pol;feature_id;mz;rt;livello;nome;identita;decided_on;decided_by;motivazione\nPOS;2;250.1234;6.00;3;;A;2026-09-24;t;why\n";
    let t = load_feature_decisions(Some(text), "f.csv").expect("load");
    assert_eq!(
        t.columns.last().map(String::as_str),
        Some("evidence_checklist")
    );
    assert_eq!(t.get(0, "evidence_checklist"), "");
    let out = write_feature_decisions(&t);
    let header: Vec<&str> = out.lines().next().expect("header").split(';').collect();
    assert_eq!(header, all_columns());
    assert_eq!(header.last(), Some(&"evidence_checklist"));
    assert_eq!(
        out.lines().nth(1).expect("row").split(';').count(),
        header.len()
    );
    let again = load_feature_decisions(Some(&out), "f.csv").expect("reload");
    assert_eq!(again, t);
    let with = text
        .replace("motivazione\n", "motivazione;evidence_checklist\n")
        .replace("why\n", "why;fragment|isotopes\n");
    let t = load_feature_decisions(Some(&with), "f.csv").expect("load");
    assert_eq!(t.get(0, "evidence_checklist"), "fragment|isotopes");
    assert!(OPTIONAL_COLUMNS.contains(&"evidence_checklist"));
}

#[test]
fn decision_errors_have_the_messages_of_attributio() {
    let head = "pol;feature_id;mz;rt;livello;nome;identita;decided_on;decided_by;motivazione";
    let err = |rows: &str| {
        load_feature_decisions(Some(&format!("{head}\n{rows}\n")), "f.csv")
            .expect_err("error")
            .msg
    };
    assert_eq!(
        err("POS;1;;;4;;;;;"),
        "f.csv: livello must be '2b', '3', '2' or empty, found ['4']"
    );
    assert_eq!(
        err("POS;1;;;2b;;;;;"),
        "f.csv: a Level 2b decision needs 'motivazione' (POS 1)"
    );
    let e = load_feature_decisions(Some("pol;feature_id\nPOS;1\n"), "f.csv")
        .expect_err("error")
        .msg;
    assert!(
        e.starts_with("f.csv: missing columns ['decided_by', 'decided_on'"),
        "{e}"
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
    let Some(base) = std::env::var_os("MZLAB_DATI").map(PathBuf::from) else {
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
