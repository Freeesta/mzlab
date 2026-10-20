#![cfg(feature = "attributio")]
//! Parity of the cascade of levels with Attributio on the synthetic campaign (level, sub-level, basis, name, identity, reasons: no tolerance), and one hand
//! case for each step of the cascade on a campaign cut down by hand.

#[macro_use]
mod attributio_comune;
use attributio_comune::*;
use mzlab_feat::attributio::annotate::{
    gnps_comparer, load_annotations, Annotations, BasicCuration, Feature,
};
use mzlab_feat::attributio::campaign::Env;
use mzlab_feat::attributio::io::Io;
use mzlab_feat::attributio::sources::load_sources;
use mzlab_feat::attributio::Campaign;

fn run(fx: &Fixture) -> Annotations {
    with_io!(fx, io);
    let src = load_sources(&fx.campaign, &io).expect("sources");
    load_annotations(&fx.campaign, &src, None, &BasicCuration, &gnps_comparer).expect("annotations")
}

fn find<'a>(a: &'a Annotations, pol: &str, id: i64) -> &'a Feature {
    a.features
        .iter()
        .find(|f| f.pol == pol && f.id == id)
        .unwrap_or_else(|| panic!("feature {pol} {id}"))
}

#[test]
fn the_cascade_equals_the_reference_tool() {
    let fx = fixture("campagna_sintetica");
    let got = run(&fx);
    let g = golden("sintetica", "livelli.json");
    let want = g["features"].as_array().expect("features");
    assert_eq!(got.features.len(), want.len(), "number of features");
    for w in want {
        let f = find(
            &got,
            w["_pol"].as_str().expect("pol"),
            w["_id"].as_i64().expect("id"),
        );
        let at = format!("{} {}", f.pol, f.id);
        assert_eq!(
            f.level as i64,
            w["level"].as_i64().expect("level"),
            "{at}: level"
        );
        assert_eq!(
            f.level_sub,
            w["level_sub"].as_str().expect("sub"),
            "{at}: level_sub"
        );
        assert_eq!(
            f.level_basis,
            w["level_basis"].as_str().expect("basis"),
            "{at}: basis"
        );
        assert_eq!(f.name(), w["name"].as_str(), "{at}: name");
        assert_eq!(f.identity(), w["identity"].as_str(), "{at}: identity");
        assert_eq!(
            f.decision.decision_2b,
            w["decision_2b"].as_bool().expect("2b"),
            "{at}: 2b"
        );
        assert_eq!(
            f.level_decisive,
            w["level_decisive"].as_str().expect("decisive"),
            "{at}: decisive"
        );
        assert_eq!(
            f.level_why,
            w["reasons"].as_str().expect("reasons"),
            "{at}: reasons"
        );
    }
}

#[test]
fn the_expected_table_of_levels_holds() {
    let fx = fixture("campagna_sintetica");
    let got = run(&fx);
    let text =
        String::from_utf8(fx.files.read("livelli_attesi.csv").expect("file")).expect("utf-8");
    let mut n = 0;
    for line in text.lines().skip(1) {
        let c = split_csv(line);
        let f = find(&got, &c[0], c[1].parse().expect("id"));
        let at = format!("{} {}", f.pol, f.id);
        assert_eq!(f.level.to_string(), c[4], "{at}: level");
        assert_eq!(f.level_sub, c[5], "{at}: level_sub");
        assert_eq!(f.level_basis, c[6], "{at}: level_basis");
        assert_eq!(f.name().unwrap_or(""), c[7], "{at}: name");
        n += 1;
    }
    assert_eq!(n, got.features.len());
}

fn split_csv(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let (mut cur, mut quoted) = (String::new(), false);
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                cur.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ';' if !quoted => out.push(std::mem::take(&mut cur)),
            _ => cur.push(c),
        }
    }
    out.push(cur);
    out
}

// ------------------------------------------------------------------------------------------------------------------ one case per step

const DECISIONS: &str = "03 Stats/scripts/shared/feature_decisions.csv";

/// The synthetic campaign with `[annotation]` keys appended to its `campaign.toml` and rows appended to the chemist's decisions.
fn variant(annotation: &str, decisions: &str, edits: &[(&str, &str)]) -> Fixture {
    let mut fx = fixture("campagna_sintetica");
    let mut toml = String::from_utf8(fx.files.read("campaign.toml").expect("toml")).expect("utf-8");
    for (old, new) in edits {
        assert!(toml.contains(old), "{old} in campaign.toml");
        toml = toml.replace(old, new);
    }
    if !annotation.is_empty() {
        toml.push_str(&format!("\n[annotation]\n{annotation}\n"));
    }
    fx.files.add("campaign.toml", toml.as_bytes());
    if !decisions.is_empty() {
        let mut d = String::from_utf8(fx.files.read(DECISIONS).expect("decisions")).expect("utf-8");
        d.push_str(decisions);
        fx.files.add(DECISIONS, d.as_bytes());
    }
    fx.campaign = Campaign::from_toml(&toml, "campaign.toml", "campagna_sintetica", Env::default())
        .expect("campaign");
    fx
}

fn row(a: &Annotations, pol: &str, id: i64) -> (u8, String, String) {
    let f = find(a, pol, id);
    (f.level as u8, f.level_sub.clone(), f.level_basis.clone())
}

fn is(a: &Annotations, pol: &str, id: i64, want: (u8, &str, &str)) {
    assert_eq!(
        row(a, pol, id),
        (want.0, want.1.to_string(), want.2.to_string()),
        "{pol} {id}"
    );
}

#[test]
fn step_1_a_reference_standard_with_its_ms2_is_level_1_and_without_it_is_not() {
    is(
        &run(&fixture("campagna_sintetica")),
        "POS",
        1,
        (1, "", "reference standard"),
    );
    let a = run(&variant(
        "",
        "",
        &[("ms2_min_cosine = 0.70", "ms2_min_cosine = 1.01")],
    ));
    is(&a, "POS", 1, (0, "", "none"));
}

#[test]
fn step_2_a_valid_library_source_is_level_2a() {
    let a = run(&fixture("campagna_sintetica"));
    is(&a, "POS", 3, (2, "a", "spectral library"));
    is(&a, "POS", 9, (2, "a", "spectral library"));
}

#[test]
fn step_2_libraries_that_name_different_structures_are_settled_by_the_margin() {
    let a = run(&fixture("campagna_sintetica"));
    is(&a, "POS", 8, (3, "a", "name conflict"));
    assert_eq!(find(&a, "POS", 8).name(), Some("Fictamine isomer"));
    let a = run(&variant("library_conflict_margin = 0.0", "", &[]));
    is(&a, "POS", 8, (2, "a", "spectral library"));
    let a = run(&variant("library_conflict = \"level3\"", "", &[]));
    is(&a, "POS", 8, (3, "a", "name conflict"));
}

#[test]
fn the_chemists_2b_decision_beats_the_library_and_the_structure() {
    let rows = "POS;9;299.1754;9.50;2b;;;2026-10-01;test;fictional;\nPOS;4;300.15;4.10;2b;;;2026-10-01;test;fictional;\n";
    let a = run(&variant("", rows, &[]));
    is(&a, "POS", 9, (2, "b", "diagnostic evidence"));
    is(&a, "POS", 4, (2, "b", "diagnostic evidence"));
    assert!(find(&a, "POS", 9).decision.decision_2b);
    // the decision to Level 3 of the fixture
    is(
        &run(&fixture("campagna_sintetica")),
        "POS",
        2,
        (3, "a", "expert review"),
    );
}

#[test]
fn step_4_csi_fingerid_gives_level_3_above_cosmic_and_a_canopus_class_gives_no_level() {
    let a = run(&fixture("campagna_sintetica"));
    is(&a, "POS", 4, (3, "a", "CSI:FingerID"));
    // POS 5 has a CANOPUS class above the threshold and nothing else: the class is shown, the level stays 0
    is(&a, "POS", 5, (0, "", "none"));
    assert!(find(&a, "POS", 5).classes[0].is_some() || find(&a, "POS", 5).classes[3].is_some());
    let a = run(&variant("cosmic_min = 0.99", "", &[]));
    assert_ne!(find(&a, "POS", 4).level_basis, "CSI:FingerID");
}

#[test]
fn the_same_compound_at_other_retention_times_are_isomers_until_the_rt_tolerance_joins_them() {
    let a = run(&fixture("campagna_sintetica"));
    is(&a, "POS", 10, (3, "a", "isomers not discriminated"));
    is(&a, "POS", 11, (3, "a", "isomers not discriminated"));
    is(&a, "POS", 14, (3, "a", "RT isomer"));
    let a = run(&variant("rt_group_tol_min = 5.0", "", &[]));
    is(&a, "POS", 10, (2, "a", "spectral library"));
    is(&a, "POS", 11, (2, "a", "spectral library"));
}

#[test]
fn every_basis_has_its_text_in_the_attributio_profile_of_the_levels() {
    let profile: serde_json::Value =
        serde_json::from_str(include_str!("../data/livelli-1.json")).expect("livelli-1.json");
    let text = profile.to_string();
    let a = run(&fixture("campagna_sintetica"));
    for f in &a.features {
        assert!(
            text.contains(&format!("\"{}\"", f.level_basis)) || f.level_basis == "none",
            "basis {}",
            f.level_basis
        );
    }
    for basis in [
        "putative library",
        "reference standard",
        "spectral library",
        "CSI:FingerID",
    ] {
        assert!(text.contains(basis), "{basis}");
    }
}
