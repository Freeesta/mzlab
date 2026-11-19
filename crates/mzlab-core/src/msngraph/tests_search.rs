//! Tests of the tree search `cerca` on synthetic graphs (no real compound, no real data): the unknown against a small `msn-lib/1`.

use super::{cerca, OpzioniCerca};
use crate::json::parse;

/// A graph with a root at `mz`, its MS2 peaks `(mz, rel)` and an MS3 of the first peak with `deep` peaks; each MS2 peak is a node with a loss from the root.
fn graph(mz: f64, pol: i32, ms2: &[(f64, f64)], deep: &[(f64, f64)]) -> String {
    let mut nodes = format!("{{\"id\":\"n0\",\"mz\":{mz},\"level\":1}}");
    let mut arcs = String::new();
    let mut peaks = String::new();
    for (i, (m, r)) in ms2.iter().enumerate() {
        let id = format!("n{}", i + 1);
        nodes += &format!(",{{\"id\":\"{id}\",\"mz\":{m},\"level\":2}}");
        arcs += &format!(
            "{}{{\"from\":\"n0\",\"a\":\"{id}\",\"loss\":{{\"da_mass\":{:.5},\"formula\":null}}}}",
            if i > 0 { "," } else { "" },
            mz - m
        );
        peaks += &format!(
            "{}{{\"mz\":{m},\"rel\":{r},\"node\":\"{id}\"}}",
            if i > 0 { "," } else { "" }
        );
    }
    let p1 = ms2[0].0;
    let deep_peaks: Vec<String> = deep
        .iter()
        .map(|(m, r)| format!("{{\"mz\":{m},\"rel\":{r},\"node\":null}}"))
        .collect();
    let mut spectra = format!("{{\"tree\":\"{}@cid35\",\"level\":2,\"activation\":\"cid\",\"ce\":35,\"precursor\":\"n0\",\"peaks\":[{peaks}]}}", mz.round());
    if !deep.is_empty() {
        spectra += &format!(",{{\"tree\":\"{}@cid35>{}@cid35\",\"level\":3,\"activation\":\"cid\",\"ce\":35,\"precursor\":\"n1\",\"peaks\":[{}]}}", mz.round(), p1.round(), deep_peaks.join(","));
    }
    format!("{{\"schema\":\"msn-graph/1\",\"polarity\":{pol},\"roots\":[\"n0\"],\"nodes\":[{nodes}],\"edges\":[{arcs}],\"spectra\":[{spectra}]}}")
}

fn library(entries: &[(&str, &str, i32)]) -> String {
    let voci: Vec<String> = entries
        .iter()
        .map(|(id, g, pol)| format!("{{\"id\":\"{id}\",\"ion\":{{\"formula\":\"C1\",\"theoretical_mz\":0,\"polarity\":{pol}}},\"activation\":[\"cid\"],\"graph\":{g}}}"))
        .collect();
    format!(
        "{{\"schema\":\"msn-lib/1\",\"entries\":[{}]}}",
        voci.join(",")
    )
}

const MS2: [(f64, f64); 3] = [(261.1, 100.0), (244.1, 50.0), (217.1, 20.0)];
const DEEP: [(f64, f64); 2] = [(217.1, 100.0), (190.1, 40.0)];

fn candidates(lib: &str, unknown: &str, o: &OpzioniCerca) -> Vec<crate::json::Value> {
    let r = cerca(lib, unknown, o).unwrap();
    let v = parse(r.as_bytes()).unwrap();
    assert_eq!(v.get("schema").unwrap().str(), Some("msn-search/1"));
    v.get("candidates").unwrap().arr().to_vec()
}

fn score(c: &crate::json::Value, k: &str) -> Option<f64> {
    c.get("scores").unwrap().get(k).unwrap().f64()
}

#[test]
fn the_same_tree_scores_one_and_the_nearest_precursor_only_enters() {
    let a = graph(305.1, 1, &MS2, &DEEP);
    let far = graph(320.2, 1, &MS2, &DEEP);
    let lib = library(&[("STD-A", &a, 1), ("STD-FAR", &far, 1)]);
    let found = candidates(&lib, &a, &OpzioniCerca::default());
    assert_eq!(found.len(), 1);
    let c = &found[0];
    assert_eq!(c.get("id").unwrap().str(), Some("STD-A"));
    assert!(score(c, "ms2").unwrap() > 0.999);
    assert!(score(c, "tree").unwrap() > 0.999);
    assert_eq!(score(c, "losses"), Some(1.0));
    let p = c.get("proofs").unwrap();
    assert_eq!(p.get("common_peaks").unwrap().f64(), Some(3.0));
    assert_eq!(p.get("aligned_nodes").unwrap().f64(), Some(1.0));
    assert_eq!(p.get("common_losses").unwrap().f64(), Some(3.0));
}

#[test]
fn a_different_fragmentation_ranks_below_the_same_one() {
    let a = graph(305.1, 1, &MS2, &DEEP);
    let other = graph(
        305.1,
        1,
        &[(261.1, 100.0), (150.0, 80.0), (120.0, 60.0)],
        &[(100.0, 100.0)],
    );
    let lib = library(&[("STD-B", &other, 1), ("STD-A", &a, 1)]);
    let found = candidates(&lib, &a, &OpzioniCerca::default());
    let ids: Vec<_> = found
        .iter()
        .map(|c| c.get("id").unwrap().str().unwrap())
        .collect();
    assert_eq!(ids, ["STD-A", "STD-B"]);
    assert!(score(&found[1], "ms2").unwrap() < score(&found[0], "ms2").unwrap());
    assert!(score(&found[1], "losses").unwrap() < 1.0);
}

#[test]
fn the_polarity_the_min_proofs_and_the_top_cut_the_list() {
    let a = graph(305.1, 1, &MS2, &DEEP);
    let neg = graph(305.1, -1, &MS2, &DEEP);
    let none = graph(305.1, 1, &[(90.0, 100.0)], &[]);
    let lib = library(&[
        ("STD-A", &a, 1),
        ("STD-NEG", &neg, -1),
        ("STD-NONE", &none, 1),
    ]);
    let found = candidates(&lib, &a, &OpzioniCerca::default());
    let ids: Vec<_> = found
        .iter()
        .map(|c| c.get("id").unwrap().str().unwrap())
        .collect();
    assert_eq!(
        ids,
        ["STD-A"],
        "the negative entry and the one with no shared peak stay out"
    );
    let all = candidates(
        &lib,
        &a,
        &OpzioniCerca {
            min_evidence: 0,
            ..OpzioniCerca::default()
        },
    );
    assert_eq!(all.len(), 2);
    let one = candidates(
        &lib,
        &a,
        &OpzioniCerca {
            min_evidence: 0,
            top: 1,
            ..OpzioniCerca::default()
        },
    );
    assert_eq!(one.len(), 1);
}

#[test]
fn an_unknown_without_msn_has_no_tree_score() {
    let a = graph(305.1, 1, &MS2, &DEEP);
    let flat = graph(305.1, 1, &MS2, &[]);
    let lib = library(&[("STD-A", &a, 1)]);
    let found = candidates(&lib, &flat, &OpzioniCerca::default());
    assert_eq!(score(&found[0], "tree"), None);
    assert!(score(&found[0], "ms2").unwrap() > 0.999);
}

#[test]
fn a_text_that_is_not_a_graph_or_a_library_is_a_keyed_error() {
    let a = graph(305.1, 1, &MS2, &DEEP);
    let lib = library(&[("STD-A", &a, 1)]);
    for (l, g) in [
        ("nope", a.as_str()),
        (lib.as_str(), "nope"),
        (lib.as_str(), "{}"),
    ] {
        let e = cerca(l, g, &OpzioniCerca::default()).unwrap_err();
        assert_eq!(e.key, "err.msnlib.bad");
    }
}

#[test]
fn an_analogue_splits_the_fragments_in_shifted_and_not_shifted_and_stays_after_the_exact_ones() {
    let std = graph(305.1, 1, &MS2, &[]);
    let modified = graph(
        321.1,
        1,
        &[(277.1, 100.0), (244.1, 50.0), (217.1, 20.0)],
        &[],
    );
    let lib = library(&[("STD", &std, 1)]);
    let none = candidates(&lib, &modified, &OpzioniCerca::default());
    assert!(none.is_empty(), "analogues are not searched by default");
    let o = OpzioniCerca {
        delta_max: 20.0,
        ..OpzioniCerca::default()
    };
    let found = candidates(&lib, &modified, &o);
    assert_eq!(found.len(), 1);
    let a = found[0].get("analog").unwrap();
    assert_eq!(a.get("delta_da").unwrap().f64(), Some(16.0));
    assert_eq!(a.get("shifted").unwrap().f64(), Some(1.0));
    assert_eq!(a.get("unshifted").unwrap().f64(), Some(2.0));
    assert_eq!(a.get("coverage").unwrap().f64(), Some(1.0));
    // an exact entry of the same unknown comes first and has no analogue field
    let exact = library(&[("STD", &std, 1), ("MOD", &modified, 1)]);
    let found = candidates(&exact, &modified, &o);
    assert_eq!(found[0].get("id").unwrap().str(), Some("MOD"));
    assert!(found[0].get("analog").unwrap().is_null());
    assert_eq!(found[1].get("id").unwrap().str(), Some("STD"));
    // beyond delta_max nothing enters
    let narrow = OpzioniCerca {
        delta_max: 10.0,
        ..OpzioniCerca::default()
    };
    assert!(candidates(&lib, &modified, &narrow).is_empty());
}

#[test]
fn the_options_of_a_request_have_defaults_and_a_bad_number_is_a_keyed_error() {
    use super::opzioni_cerca;
    let q = |pairs: &'static [(&str, &str)]| {
        move |k: &str| pairs.iter().find(|p| p.0 == k).map(|p| p.1.to_string())
    };
    assert_eq!(opzioni_cerca(&q(&[])).unwrap(), OpzioniCerca::default());
    let o = opzioni_cerca(&q(&[
        ("precursor_ppm", "5"),
        ("top", "3"),
        ("delta_max", "16"),
    ]))
    .unwrap();
    assert_eq!((o.ppm_prec, o.top, o.delta_max), (5.0, 3, 16.0));
    let e = opzioni_cerca(&q(&[("top", "abc")])).err().unwrap();
    assert_eq!(e.key, "err.request.badNumber");
    assert!(opzioni_cerca(&q(&[("tol_mz", "-1")])).is_err());
}

/// The same ion fragmented at the same last energy after two different upstream energies: two spectra with the same shape and the same energy.
fn two_routes() -> String {
    let base = graph(305.1, 1, &MS2, &DEEP);
    let extra = "{\"tree\":\"305@cid20\",\"level\":2,\"activation\":\"cid\",\"ce\":20,\"precursor\":\"n0\",\"peaks\":[{\"mz\":261.1,\"rel\":100,\"node\":\"n1\"},{\"mz\":244.1,\"rel\":50,\"node\":\"n2\"}]},\
        {\"tree\":\"305@cid20>261@cid35\",\"level\":3,\"activation\":\"cid\",\"ce\":35,\"precursor\":\"n1\",\"peaks\":[{\"mz\":150.0,\"rel\":100,\"node\":null},{\"mz\":120.0,\"rel\":50,\"node\":null}]}";
    let cut = base.rfind("]}").unwrap();
    format!("{},{extra}{}", &base[..cut], &base[cut..])
}

#[test]
fn a_tree_scores_one_against_itself_when_two_spectra_share_shape_and_energy() {
    let a = two_routes();
    let lib = library(&[("STD-A", &a, 1)]);
    let found = candidates(&lib, &a, &OpzioniCerca::default());
    let c = &found[0];
    assert!(
        (score(c, "tree").unwrap() - 1.0).abs() < 1e-9,
        "{:?}",
        score(c, "tree")
    );
    assert!((score(c, "ms2").unwrap() - 1.0).abs() < 1e-9);
}

#[test]
fn an_ms2_of_a_feature_finds_the_entry_at_its_precursor() {
    let lib = library(&[
        ("A", &graph(305.1, 1, &MS2, &[]), 1),
        ("B", &graph(350.2, 1, &MS2, &[]), 1),
        ("N", &graph(305.1, -1, &MS2, &[]), -1),
    ]);
    let hits = super::search_ms2(&lib, 305.1, true, &MS2, 10.0, 0.01, 3).unwrap();
    // B is at another precursor, N has the other polarity
    assert_eq!(hits.len(), 1);
    assert_eq!((hits[0].id.as_str(), hits[0].shared), ("A", 3));
    assert!(hits[0].entropy > 0.99);
    // too few shared peaks: nothing
    assert!(
        super::search_ms2(&lib, 305.1, true, &[(100.0, 1.0)], 10.0, 0.01, 3)
            .unwrap()
            .is_empty()
    );
}
