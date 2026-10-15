#![cfg(feature = "attributio")]
//! Parity of the proposal tiers with Attributio on the synthetic campaign (tier, proposed level and reasons: exact equality), and hand cases for the
//! pieces the campaign has no data for (references, raw proofs, the JSON of the proposal).

#[macro_use]
mod attributio_comune;
use attributio_comune::*;
use mzlab_feat::attributio::annotate::{gnps_comparer, load_annotations, BasicCuration};
use mzlab_feat::attributio::io::Io;
use mzlab_feat::attributio::review::feature::{review_feature, Env, Inputs};
use mzlab_feat::attributio::review::libraries::LibraryIndex;
use mzlab_feat::attributio::review::raw::NoRaw;
use mzlab_feat::attributio::review::settings::ReviewSettings;
use mzlab_feat::attributio::review::structures::NoStructures;
use mzlab_feat::attributio::sources::load_sources;

#[test]
fn the_tiers_equal_the_reference_tool() {
    let fx = fixture("campagna_sintetica");
    with_io!(fx, io);
    let src = load_sources(&fx.campaign, &io).expect("sources");
    let ann = load_annotations(&fx.campaign, &src, None, &BasicCuration, &gnps_comparer)
        .expect("annotations");
    let (s, lib) = (ReviewSettings::default(), LibraryIndex::build(vec![]));
    let env = Env {
        s: &s,
        lib: &lib,
        st: &NoStructures,
        raw: &NoRaw,
    };
    let g = golden("sintetica", "proposte.json");
    let want = g["features"].as_array().expect("features");
    for w in want {
        let (pol, id) = (
            w["pol"].as_str().expect("pol"),
            w["id"].as_i64().expect("id"),
        );
        let f = ann
            .features
            .iter()
            .find(|f| f.pol == pol && f.id == id)
            .unwrap_or_else(|| panic!("feature {pol} {id}"));
        let p = review_feature(&env, f, &Inputs::default());
        let at = format!("{pol} {id}");
        assert_eq!(p.class, w["tier"].as_str().expect("tier"), "{at}: class");
        assert_eq!(
            p.proposed,
            w["proposed"].as_str().expect("proposed"),
            "{at}: proposed"
        );
        let reasons: Vec<&str> = w["reasons"]
            .as_array()
            .expect("reasons")
            .iter()
            .map(|r| r.as_str().expect("text"))
            .collect();
        assert_eq!(
            p.reasons.iter().map(String::as_str).collect::<Vec<_>>(),
            reasons,
            "{at}: reasons"
        );
    }
    assert_eq!(want.len(), 11, "features of the review scope");
}

#[test]
fn the_proposal_is_valid_json() {
    let fx = fixture("campagna_sintetica");
    with_io!(fx, io);
    let src = load_sources(&fx.campaign, &io).expect("sources");
    let ann = load_annotations(&fx.campaign, &src, None, &BasicCuration, &gnps_comparer)
        .expect("annotations");
    let (s, lib) = (ReviewSettings::default(), LibraryIndex::build(vec![]));
    let env = Env {
        s: &s,
        lib: &lib,
        st: &NoStructures,
        raw: &NoRaw,
    };
    for f in &ann.features {
        let p = review_feature(&env, f, &Inputs::default());
        let v: serde_json::Value = serde_json::from_str(&p.to_json()).expect("JSON");
        assert_eq!(v["class"], p.class);
        assert_eq!(v["id"], p.id);
        for k in ["class", "reasons", "best", "alternatives", "evidence"] {
            assert!(v.get(k).is_some(), "{k}");
        }
    }
}
