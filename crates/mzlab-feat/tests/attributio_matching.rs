//! AT-05: the review matching of Attributio ported on `mzlab-lib` equals the golden stage `abbinamenti.json` (cosines and counts at 1e-9 relative,
//! the same peak indices, the same library entries, adduct keys, source tables and settings).
#![cfg(feature = "attributio")]

use mzlab_feat::attributio::review::adduct::adduct_key;
use mzlab_feat::attributio::review::libraries::{find_library_files, read_records, LibFormat};
use mzlab_feat::attributio::review::matching::{compare, delta_formulas, match_spectra, Kind, Ms2};
use mzlab_feat::attributio::review::settings::{ReviewSettings, Value};
use mzlab_feat::attributio::review::sources::{
    csi_top5, gnps_hits, network_edges, sirius_formulas, sirius_hits, SourceTexts,
};
use serde_json::Value as J;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn golden() -> J {
    serde_json::from_slice(
        &std::fs::read(root().join("tests/golden/attributio/sintetica/abbinamenti.json")).unwrap(),
    )
    .unwrap()
}

fn close(a: f64, b: f64, what: &str) {
    assert!(
        (a - b).abs() <= 1e-9 * b.abs().max(1e-12),
        "{what}: {a} vs {b}"
    );
}

fn f(v: &J) -> f64 {
    v.as_f64().unwrap()
}

fn vec_f(v: &J) -> Vec<f64> {
    v.as_array().unwrap().iter().map(f).collect()
}

fn ms2(v: &J) -> Ms2 {
    Ms2 {
        mz: vec_f(&v["mz"]),
        it: vec_f(&v["it"]),
        prec: f(&v["prec"]),
    }
}

fn pairs_of(v: &J) -> Vec<(usize, usize, usize)> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|p| {
            (
                p[0].as_u64().unwrap() as usize,
                p[1].as_u64().unwrap() as usize,
                p[2].as_u64().unwrap() as usize,
            )
        })
        .collect()
}

fn check_match(m: &mzlab_feat::attributio::review::matching::Match, g: &J, what: &str) {
    close(m.cosine, f(&g["cosine"]), what);
    close(m.reverse, f(&g["reverse"]), what);
    assert_eq!(
        m.matched as u64,
        g["matched"].as_u64().unwrap(),
        "{what} matched"
    );
    assert_eq!(
        m.n_shifted as u64,
        g["n_shifted"].as_u64().unwrap(),
        "{what} shifted"
    );
    let got: Vec<_> = m
        .pairs
        .iter()
        .map(|p| (p.0, p.1, usize::from(p.2 == Kind::Shifted)))
        .collect();
    assert_eq!(got, pairs_of(&g["pairs"]), "{what} pairs");
}

#[test]
fn thirty_pairs_match_attributio() {
    let g = golden();
    let pool: Vec<Ms2> = g["pool"].as_array().unwrap().iter().map(ms2).collect();
    let pairs = g["coppie"].as_array().unwrap();
    assert_eq!(pairs.len(), 30);
    for c in pairs {
        let (a, b) = (
            &pool[c["i"].as_u64().unwrap() as usize],
            &pool[c["j"].as_u64().unwrap() as usize],
        );
        let (shift, low) = (f(&c["shift"]), c["low_limit"].as_f64());
        let tag = format!("pair {}-{}", c["i"], c["j"]);
        check_match(
            &match_spectra(a, b, 0.02, 0.0, None),
            &c["plain"],
            &format!("{tag} plain"),
        );
        check_match(
            &match_spectra(a, b, 0.02, shift, None),
            &c["shifted"],
            &format!("{tag} shifted"),
        );
        check_match(
            &match_spectra(a, b, 0.02, shift, Some(105.0)),
            &c["vis"],
            &format!("{tag} vis"),
        );
        let k = compare(a, b, 0.02, low, shift);
        let e = &c["compare"];
        close(k.cos, f(&e["cos"]), &tag);
        close(k.rev, f(&e["rev"]), &tag);
        close(k.cos_vis, f(&e["cos_vis"]), &tag);
        assert_eq!(k.matched as u64, e["matched"].as_u64().unwrap(), "{tag}");
        assert_eq!(
            k.matched_vis as u64,
            e["matched_vis"].as_u64().unwrap(),
            "{tag}"
        );
        assert_eq!(
            k.n_shifted as u64,
            e["n_shifted"].as_u64().unwrap(),
            "{tag}"
        );
    }
}

#[test]
fn delta_formulas_match_attributio() {
    for d in golden()["delta"].as_array().unwrap() {
        let got = delta_formulas(
            f(&d["delta"]),
            f(&d["tol_mda"]),
            d["n"].as_u64().unwrap() as usize,
        );
        let want = d["out"].as_array().unwrap();
        assert_eq!(got.len(), want.len(), "delta {}", d["delta"]);
        for (g, w) in got.iter().zip(want) {
            assert_eq!(g.label, w["label"].as_str().unwrap());
            assert_eq!(g.common, w["common"].as_bool().unwrap());
            close(g.mass, f(&w["mass"]), &g.label);
            close(g.err_mda, f(&w["err_mda"]), &g.label);
        }
    }
}

#[test]
fn adduct_keys_match_attributio() {
    let g = golden();
    for (k, w) in g["librerie"]["adduct_keys"].as_object().unwrap() {
        let got = adduct_key(k);
        match w.as_array() {
            None => assert!(got.is_none(), "{k:?}"),
            Some(a) => {
                let (n, d, z) = got.unwrap_or_else(|| panic!("{k:?}"));
                assert_eq!(
                    (n, z),
                    (a[0].as_i64().unwrap(), a[2].as_i64().unwrap()),
                    "{k:?}"
                );
                close(d, f(&a[1]), k);
            }
        }
    }
}

#[test]
fn library_records_match_attributio() {
    let g = golden();
    let dir = root().join("tests/data/attributio/librerie_sintetiche");
    let listing: Vec<(String, Vec<u8>)> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .map(|n| (n.clone(), std::fs::read(dir.join(&n)).unwrap()))
        .collect();
    let found: Vec<(String, String)> = find_library_files(&listing)
        .iter()
        .map(|(n, f)| (n.clone(), f.name().to_string()))
        .collect();
    let want: Vec<(String, String)> = g["librerie"]["found"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| (p[0].as_str().unwrap().into(), p[1].as_str().unwrap().into()))
        .collect();
    assert_eq!(found, want);
    for (name, fmt) in &found {
        let format = find_library_files(&listing)
            .into_iter()
            .find(|(n, _)| n == name)
            .map(|x| x.1)
            .unwrap();
        assert_eq!(format.name(), fmt);
        let recs =
            read_records(format, &listing.iter().find(|(n, _)| n == name).unwrap().1).unwrap();
        let w = g["librerie"]["files"][name.as_str()]["records"]
            .as_array()
            .unwrap();
        assert_eq!(recs.len(), w.len(), "{name}");
        for (r, w) in recs.iter().zip(w) {
            assert_eq!(r.name, w["name"].as_str().unwrap(), "{name}");
            assert_eq!(r.adduct, w["adduct"].as_str().unwrap(), "{name}");
            // mzlab-lib also reads the polarity from the charge (`CHARGE=1+`), which Attributio keeps but never uses: only an empty golden differs.
            let wp = w["pol"].as_str().unwrap();
            assert!(r.pol == wp || wp.is_empty(), "{name}: {} vs {wp}", r.pol);
            assert_eq!(r.ik1, w["ik1"].as_str().unwrap(), "{name}");
            assert_eq!(r.entry, w["entry"].as_str().unwrap(), "{name}");
            assert_eq!(r.mz.len() as u64, w["n_peaks"].as_u64().unwrap(), "{name}");
            close(r.prec, f(&w["prec"]), name);
        }
    }
    assert_eq!(LibFormat::SiriusDb.name(), "siriusdb");
}

fn read(p: &str) -> String {
    std::fs::read_to_string(
        root()
            .join("tests/data/attributio/fonti_sintetiche")
            .join(p),
    )
    .unwrap()
}

fn texts() -> SourceTexts {
    let mut t = SourceTexts::default();
    for (pol, dir) in [("POS", "sirius_POS"), ("NEG", "sirius_NEG")] {
        for table in [
            "spectral_matches_top-5",
            "spectral_matches_analog_top-5",
            "formula_identifications_top-5",
            "structure_identifications_top-5",
        ] {
            if let Ok(s) = std::fs::read_to_string(
                root()
                    .join("tests/data/attributio/fonti_sintetiche")
                    .join(dir)
                    .join(format!("{table}.tsv")),
            ) {
                t.sirius.push((pol.into(), table.into(), s));
            }
        }
    }
    t.gnps_library.push((
        "POS".into(),
        read("gnps_POS/nf_output/library/merged_results_with_gnps.tsv"),
    ));
    t.gnps_pairs.push((
        "POS".into(),
        read("gnps_POS/nf_output/networking/filtered_pairs.tsv"),
    ));
    t
}

#[test]
fn source_tables_match_attributio() {
    let (g, t) = (golden(), texts());
    let fonti = &g["fonti"];
    let hits = sirius_hits(&t);
    let w = fonti["sirius_hits"].as_array().unwrap();
    assert_eq!(hits.len(), w.len());
    for (h, w) in hits.iter().zip(w) {
        assert_eq!(
            (h.pol.as_str(), h.id, h.kind, h.ref_id.as_str()),
            (
                w["_pol"].as_str().unwrap(),
                w["_id"].as_i64().unwrap(),
                w["kind"].as_str().unwrap(),
                w["ref_id"].as_str().unwrap()
            )
        );
        assert_eq!(h.similarity, w["similarity"].as_f64());
        assert_eq!(h.shared, w["shared"].as_f64());
        assert_eq!(h.rank, w["rank"].as_f64());
        assert_eq!(h.adduct.as_deref(), w["adduct"].as_str());
        assert_eq!(h.ik1.as_deref(), w["ik1"].as_str());
    }
    let forms = sirius_formulas(&t);
    let w = fonti["sirius_formulas"].as_array().unwrap();
    assert_eq!(forms.len(), w.len());
    for (((pol, id), items), w) in forms.iter().zip(w) {
        assert_eq!(
            (pol.as_str(), *id),
            (w["pol"].as_str().unwrap(), w["id"].as_i64().unwrap())
        );
        let wi = w["items"].as_array().unwrap();
        assert_eq!(items.len(), wi.len());
        for (c, x) in items.iter().zip(wi) {
            assert_eq!(
                (c.formula.as_str(), c.adduct.as_str(), c.ion.as_str()),
                (
                    x["formula"].as_str().unwrap(),
                    x["adduct"].as_str().unwrap(),
                    x["ion"].as_str().unwrap()
                )
            );
            assert_eq!(c.rank, x["rank"].as_i64());
            assert_eq!(c.score, x["score"].as_f64());
        }
    }
    let csi = csi_top5(&t);
    let w = fonti["csi_top5"].as_array().unwrap();
    assert_eq!(csi.len(), w.len());
    for (((pol, id), items), w) in csi.iter().zip(w) {
        assert_eq!(
            (pol.as_str(), *id),
            (w["pol"].as_str().unwrap(), w["id"].as_i64().unwrap())
        );
        let wi = w["items"].as_array().unwrap();
        assert_eq!(items.len(), wi.len());
        for (c, x) in items.iter().zip(wi) {
            assert_eq!(
                (c.name.as_str(), c.formula.as_str(), c.smiles.as_str()),
                (
                    x["name"].as_str().unwrap(),
                    x["formula"].as_str().unwrap(),
                    x["smiles"].as_str().unwrap()
                )
            );
            assert_eq!(c.xlogp, x["xlogp"].as_f64());
        }
    }
}

#[test]
fn network_and_gnps_tables_match_attributio() {
    let (g, t) = (golden(), texts());
    let fonti = &g["fonti"];
    let edges = network_edges(&t);
    let w = fonti["network_edges"].as_array().unwrap();
    assert_eq!(edges.len(), w.len());
    for (e, w) in edges.iter().zip(w) {
        assert_eq!(
            (e.a, e.b),
            (w["a"].as_i64().unwrap(), w["b"].as_i64().unwrap())
        );
        assert_eq!(e.cosine, w["cosine"].as_f64());
        assert_eq!(e.component, w["component"].as_f64());
    }
    let gh = gnps_hits(&t);
    let w = fonti["gnps_hits"].as_array().unwrap();
    assert_eq!(gh.len(), w.len());
    for (h, w) in gh.iter().zip(w) {
        assert_eq!(
            (h.pol.as_str(), h.id, h.name.as_str()),
            (
                w["_pol"].as_str().unwrap(),
                w["_id"].as_i64().unwrap(),
                w["name"].as_str().unwrap()
            )
        );
        assert_eq!(h.score, w["score"].as_f64());
        assert_eq!(h.mass_diff, w["mass_diff"].as_f64());
        assert_eq!(h.ref_prec, w["ref_prec"].as_f64());
    }
}

fn to_json(v: &Value) -> J {
    match v {
        Value::Num(n) => serde_json::json!(n),
        Value::Str(s) => J::String(s.clone()),
        Value::List(l) => J::Array(l.iter().map(to_json).collect()),
    }
}

fn same(a: &J, b: &J, k: &str) {
    match (a, b) {
        (J::Number(x), J::Number(y)) => close(x.as_f64().unwrap(), y.as_f64().unwrap(), k),
        (J::Array(x), J::Array(y)) => {
            assert_eq!(x.len(), y.len(), "{k}");
            x.iter().zip(y).for_each(|(p, q)| same(p, q, k));
        }
        _ => assert_eq!(a, b, "{k}"),
    }
}

#[test]
fn settings_match_attributio() {
    let g = golden();
    let s = &g["impostazioni"];
    let defaults = ReviewSettings::default();
    for (k, v) in defaults.entries() {
        same(&to_json(&v), &s["defaults"][k], k);
    }
    let parsed =
        ReviewSettings::from_campaign_toml("campaign.toml", s["toml"].as_str().unwrap()).unwrap();
    for (k, v) in parsed.entries() {
        same(&to_json(&v), &s["parsed"][k], k);
    }
    let err = ReviewSettings::from_campaign_toml("campaign.toml", "[review]\nppm_ionn = 3\n")
        .unwrap_err();
    assert_eq!(err, s["unknown_key_error"].as_str().unwrap());
}

#[test]
fn library_index_finds_by_mass_and_entry() {
    use mzlab_feat::attributio::review::libraries::{LibInput, LibraryIndex};
    let dir = root().join("tests/data/attributio/librerie_sintetiche");
    let inputs = vec![LibInput {
        label: "lib.mgf".into(),
        format: LibFormat::Mgf,
        bytes: std::fs::read(dir.join("lib.mgf")).unwrap(),
    }];
    let idx = LibraryIndex::build(inputs);
    assert_eq!(idx.len(), 3);
    let hits = idx.exact(222.1, "POS", 10.0, None);
    assert_eq!(hits.len(), 1);
    assert_eq!(idx.record(hits[0]).name, "Fictazine");
    assert_eq!(
        idx.find_entry("SYN0002")
            .map(|r| idx.record(r).name.clone())
            .as_deref(),
        Some("Fictol")
    );
    let siriusdb = LibraryIndex::build(vec![LibInput {
        label: "x.siriusdb".into(),
        format: LibFormat::SiriusDb,
        bytes: vec![1],
    }]);
    assert!(siriusdb.is_empty() && siriusdb.warnings[0].contains("desktop"));
}
