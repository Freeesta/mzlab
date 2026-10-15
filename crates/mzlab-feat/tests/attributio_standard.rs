#![cfg(feature = "attributio")]
//! Parity of the Level 1 standards and of the feature decisions with the golden data of the reference tool, and the hand cases of the decision file.

#[macro_use]
mod attributio_comune;
use attributio_comune::*;
use mzlab_feat::attributio::readers::feature_decisions::{
    all_columns, load_feature_decisions, match_decisions, merge_decisions, write_feature_decisions,
    DecisionTable, FeatureKey, OPTIONAL_COLUMNS,
};
use mzlab_feat::attributio::readers::ms2::Spectrum;
use mzlab_feat::attributio::readers::standards::{
    mass_rt_candidates, FeatureRef, Similarity, StandardsSettings, Tolerances,
};
use serde_json::{json, Value};

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
        "f.csv: livello must be '2b', '3a', '3b', '3', '2' or empty, found ['4']"
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
