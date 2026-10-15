#![cfg(feature = "attributio")]
//! Parity of the curation, the names and the identities of Attributio on the synthetic campaign (identity table and counts per level, canonical names, the
//! curation of names, the evidence of the members: no tolerance on text, 1e-9 relative on numbers) and hand cases for the rules that the campaign does not reach.

#[macro_use]
mod attributio_comune;
use attributio_comune::*;
use mzlab_feat::attributio::annotate::{
    gnps_comparer, load_annotations, Annotations, BasicCuration, Curation, Feature,
};
use mzlab_feat::attributio::coherence::coherence_report;
use mzlab_feat::attributio::curation::{classify_noncec, TableCuration};
use mzlab_feat::attributio::identity::{
    feature_level_counts, identity_table, identity_table_by_structure, inchikey_skeleton,
    level_counts, load_confident_inventory, DROP_GROUPS,
};
use mzlab_feat::attributio::io::Io;
use mzlab_feat::attributio::members::{feature_evidence, identity_members, EvidenceExtras};
use mzlab_feat::attributio::names::{canonical_name, split_rt_isomer};
use mzlab_feat::attributio::pattern::Pattern;
use mzlab_feat::attributio::sources::load_sources;
use mzlab_feat::attributio::Sources;
use serde_json::{json, Value};

struct Run {
    fx: Fixture,
    src: Sources,
    ann: Annotations,
    cur: TableCuration,
    sites: Vec<String>,
}

fn run() -> Run {
    let fx = fixture("campagna_sintetica");
    with_io!(fx, io);
    let src = load_sources(&fx.campaign, &io).expect("sources");
    let cur = TableCuration::load(&fx.campaign, &io).expect("curation");
    let ann =
        load_annotations(&fx.campaign, &src, None, &cur, &gnps_comparer).expect("annotations");
    let sites = fx
        .campaign
        .site_keys()
        .iter()
        .map(|s| s.to_string())
        .collect();
    Run {
        fx,
        src,
        ann,
        cur,
        sites,
    }
}

fn text(v: &Option<String>) -> Value {
    json!(v)
}

#[test]
fn the_identity_table_and_the_counts_equal_the_reference_tool() {
    let r = run();
    assert!(
        !r.cur.is_empty(),
        "the synthetic campaign has its curation tables"
    );
    let g = golden("sintetica", "identita.json");
    let got = identity_table(&r.ann.features, &r.sites, &r.cur, &DROP_GROUPS);
    let want = g["identity_table"].as_array().expect("table");
    assert_eq!(got.len(), want.len(), "identities");
    for (row, w) in got.iter().zip(want) {
        let at = &row.identity;
        let mut mine = json!({
            "identity": row.identity, "best_level": row.best_level, "best_level_sub": row.best_level_sub,
            "is_cec": row.is_cec, "identity_category": row.identity_category, "group": row.group,
            "cec_label": text(&row.cec_label), "cec_category": text(&row.cec_category),
        });
        for (s, v) in &row.in_site {
            mine[format!("in_{s}")] = json!(v);
        }
        same(&mine, w, at);
    }
    let c = level_counts(&r.ann.features, &r.sites, &r.cur, &DROP_GROUPS);
    let counts: Value = c
        .pairs()
        .into_iter()
        .map(|(k, v)| (k.to_string(), json!(v)))
        .collect::<serde_json::Map<_, _>>()
        .into();
    same(&counts, &g["level_counts"], "level_counts");
}

#[test]
fn the_cascade_with_the_tables_keeps_the_levels_of_the_golden() {
    let r = run();
    let g = golden("sintetica", "livelli.json");
    for w in g["features"].as_array().expect("features") {
        let (pol, id) = (
            w["_pol"].as_str().expect("pol"),
            w["_id"].as_i64().expect("id"),
        );
        let f = r
            .ann
            .features
            .iter()
            .find(|f| f.pol == pol && f.id == id)
            .expect("feature");
        assert_eq!(
            f.level as i64,
            w["level"].as_i64().expect("level"),
            "{pol} {id}"
        );
        assert_eq!(f.identity(), w["identity"].as_str(), "{pol} {id}: identity");
    }
}

fn names_row(c: &TableCuration, w: &Value) -> Value {
    let raw = w["name"].as_str().unwrap_or("");
    let clean = c.clean_library_name(raw);
    let cec = |m: Option<(String, String)>| m.map_or(json!([null, null]), |(a, b)| json!([a, b]));
    let id = |f: Option<&str>| {
        let (a, b) = c.curate_identity(None, Some(&clean), f);
        json!([a, b])
    };
    json!({
        "name": w["name"], "clean": clean, "classify": classify_noncec(Some(&clean)),
        "match_cec": cec(c.match_cec(&[Some(&clean)])),
        "noncec": c.match_noncec_override(Some(&clean)).map_or(Value::Null, |(a, b)| json!([a, b])),
        "identity": id(None), "identity_formula": id(Some("C1H2O3")),
        "raw_canonical": [canonical_name(raw, None), canonical_name(raw, Some("C6H14O4"))],
        "clean_canonical": [canonical_name(&clean, None), canonical_name(&clean, Some("C6H14O4"))],
    })
}

#[test]
fn the_names_and_the_curation_equal_the_reference_tool() {
    let r = run();
    let g = golden("sintetica", "nomi.json");
    for w in g["names"].as_array().expect("names") {
        let mine = names_row(&r.cur, w);
        let mut want = w.clone();
        // the golden has a canonical name without formula first and with it second; `None` and `""` are the same formula
        want["match_cec"] = w["match_cec"].clone();
        same(&mine, &want, &format!("{}", w["name"]));
    }
    let u = &g["unnamed"];
    let pair = |t: (String, String)| json!([t.0, t.1]);
    assert_eq!(pair(r.cur.curate_identity(None, None, None)), u["none"]);
    assert_eq!(
        pair(r.cur.curate_identity(None, None, Some("C9H9"))),
        u["formula"]
    );
    let given = ("Given".to_string(), "ppcp".to_string());
    assert_eq!(
        pair(r.cur.curate_identity(Some(&given), Some("Caffeine"), None)),
        u["given"]
    );
    let blank = ("  ".to_string(), String::new());
    assert_eq!(
        pair(r.cur.curate_identity(Some(&blank), Some("caffeine"), None)),
        u["blank_given"]
    );
    let a = r.cur.audit_name_map();
    assert!(a.duplicates.is_empty() && a.shadows.is_empty() && a.invalid.is_empty());
    assert!(g["audit"]["duplicates"].as_array().expect("dup").is_empty());
}

fn evidence_json(r: &Run) -> Vec<Value> {
    with_io!(r.fx, io);
    let c = &r.fx.campaign;
    let x = EvidenceExtras::load(c, &io, 10.0).expect("extras");
    let ev = feature_evidence(&c.id, &r.ann.features, &r.src, &x, &r.cur);
    ev.iter()
        .map(|e| {
            json!({
                "_pol": e.pol, "_id": e.id, "identity": e.identity, "group": e.group, "category": e.category, "name_source": e.name_source,
                "source_name": e.source_name, "name_inchikey": e.name_inchikey, "name_block": e.name_block, "name_smiles": e.name_smiles,
                "adduct": e.adduct, "spec_reference": e.spec_reference, "gnps_library": e.gnps_library, "gnps_spectrum_id": e.gnps_spectrum_id,
                "loader_block": e.loader_block, "loader_source": e.loader_source, "sirius_formula": e.sirius_formula,
                "sirius_xlogp": e.sirius_xlogp, "mzlib_structure_ok": e.mzlib_structure_ok, "mzlib_structure_ppm": e.mzlib_structure_ppm,
                "level": e.level, "mz": e.mz, "rt": e.rt,
            })
        })
        .collect()
}

#[test]
fn the_evidence_and_the_members_equal_the_reference_tool() {
    let r = run();
    let g = golden("sintetica", "membri.json");
    let want = g["evidence"].as_array().expect("evidence");
    let got = evidence_json(&r);
    assert_eq!(got.len(), want.len(), "evidence rows");
    for w in want {
        let mut w = w.clone();
        for (to, from) in [("_pol", "_pol"), ("_id", "_id")] {
            w[to] = w[from].clone();
        }
        w.as_object_mut().expect("row").remove("campaign");
        let m = got
            .iter()
            .find(|m| m["_pol"] == w["_pol"] && m["_id"] == w["_id"])
            .expect("row");
        same(m, &w, &format!("{} {}", w["_pol"], w["_id"]));
    }
    with_io!(r.fx, io);
    let x = EvidenceExtras::load(&r.fx.campaign, &io, 10.0).expect("extras");
    let ev = feature_evidence(&r.fx.campaign.id, &r.ann.features, &r.src, &x, &r.cur);
    let members = identity_members(&ev);
    let want = g["members"].as_array().expect("members");
    assert_eq!(members.len(), want.len(), "members");
    for (m, w) in members.iter().zip(want) {
        let mine = json!({
            "name": m.name, "campaign": m.campaign, "group": m.group, "category": m.category, "best_level": m.best_level,
            "n_features": m.n_features, "first_block": m.first_block, "inchikey": m.inchikey, "smiles": m.smiles, "ik_basis": m.ik_basis,
            "name_sources": m.name_sources, "lookup_name": m.lookup_name, "note": m.note,
        });
        let mut w = w.clone();
        for k in ["member_type", "lists", "formula", "mono_mass"] {
            w.as_object_mut().expect("row").remove(k);
        }
        same(&mine, &w, &format!("{} {:?}", m.name, m.first_block));
    }
}

#[test]
fn the_structural_identity_table_equals_the_reference_tool() {
    let r = run();
    let g = golden("sintetica", "membri.json");
    let got = identity_table_by_structure(&r.ann.features, &r.sites, &r.cur, &DROP_GROUPS);
    let want = g["by_structure"].as_array().expect("rows");
    assert_eq!(got.len(), want.len());
    for (row, w) in got.iter().zip(want) {
        assert_eq!(row.dedup_key, w["dedup_key"].as_str().expect("key"));
        assert_eq!(row.identity, w["identity"].as_str().expect("identity"));
        assert_eq!(
            row.best_level as i64,
            w["best_level"].as_i64().expect("level")
        );
        assert_eq!(row.key_type, w["key_type"].as_str().expect("type"));
        assert_eq!(json!(row.names_merged), w["names_merged"]);
        assert_eq!(row.group, w["group"].as_str().expect("group"));
        for (s, v) in &row.in_site {
            assert_eq!(json!(v), w[format!("in_{s}")], "{} in {s}", row.identity);
        }
    }
}

#[test]
fn every_feature_has_the_untid_of_its_polarity() {
    let r = run();
    let ev = evidence_json(&r);
    assert!(!ev.is_empty());
    with_io!(r.fx, io);
    let x = EvidenceExtras::load(&r.fx.campaign, &io, 10.0).expect("extras");
    let rows = feature_evidence("S", &r.ann.features, &r.src, &x, &r.cur);
    let pos = rows
        .iter()
        .find(|e| e.pol == "POS" && e.id == 1)
        .expect("POS 1");
    // 200.102 - 1.00728 = 199.0947, read as [M+H]+ for want of hypotheses
    assert_eq!(pos.untid.as_deref(), Some("199.0947 [M+H]+"));
    assert!(rows.iter().all(|e| e.untid.is_some()));
}

#[test]
fn the_coherence_report_and_the_inventory_hold_on_the_synthetic_campaign() {
    let r = run();
    let rep = coherence_report(&r.ann.features, &r.sites, &r.cur);
    assert!(rep.failed().is_empty(), "violated: {:?}", rep.failed());
    let inv = load_confident_inventory(&r.ann.features, &r.sites, &r.cur, &[1, 2, 3]);
    let all = identity_table(&r.ann.features, &r.sites, &r.cur, &[]);
    assert_eq!(
        inv.len(),
        all.len(),
        "the inventory and the table agree on the identities"
    );
    let by_level = feature_level_counts(&r.ann.features, &r.sites, &[4, 0]);
    assert_eq!(by_level.len(), r.sites.len());
}

// ------------------------------------------------------------------------------------------------------------------ hand cases

fn feature(id: i64, level: i32, name: &str) -> Feature {
    let mut f = Feature {
        pol: "POS".into(),
        id,
        level,
        mz: Some(200.0 + id as f64),
        ..Feature::default()
    };
    f.disp_name = Some(name.to_string());
    f.in_site = vec![("SiteA".into(), true), ("SiteB".into(), false)];
    f
}

fn sites() -> Vec<String> {
    vec!["SiteA".into(), "SiteB".into()]
}

#[test]
fn two_features_with_one_skeleton_and_two_names_are_one_structural_identity() {
    let mut a = feature(1, 2, "Fictol");
    let mut b = feature(2, 3, "Fictol, alternative name");
    a.inchikey_skeleton = Some("ABCDEFGHIJKLMN".into());
    b.inchikey_skeleton = Some("ABCDEFGHIJKLMN".into());
    let by_name = identity_table(
        &[a.clone(), b.clone()],
        &sites(),
        &BasicCuration,
        &DROP_GROUPS,
    );
    assert_eq!(by_name.len(), 2, "the name method keeps two names apart");
    let by_structure = identity_table_by_structure(&[a, b], &sites(), &BasicCuration, &DROP_GROUPS);
    assert_eq!(by_structure.len(), 1);
    let row = &by_structure[0];
    assert_eq!(
        (
            row.identity.as_str(),
            row.best_level,
            row.names_merged.len()
        ),
        ("Fictol", 2, 2)
    );
    assert_eq!(row.key_type, "inchikey_skeleton");
}

#[test]
fn the_identity_override_of_the_rt_rule_and_the_group_override_of_the_chemist_apply() {
    let mut a = feature(1, 3, "Fictol");
    a.rt_info.identity_override = Some("Fictol isomer, RT 8.0 min".into());
    let mut b = feature(2, 3, "Fictmix");
    b.decision.group_override = Some("lab_artifact".into());
    let t = identity_table(
        &[a.clone(), b.clone()],
        &sites(),
        &BasicCuration,
        &DROP_GROUPS,
    );
    assert_eq!(
        t.iter().map(|r| r.identity.as_str()).collect::<Vec<_>>(),
        ["Fictol isomer, RT 8.0 min"]
    );
    let all = identity_table(&[a, b], &sites(), &BasicCuration, &[]);
    assert_eq!(all.len(), 2);
    let mix = all
        .iter()
        .find(|r| r.identity == "Fictmix")
        .expect("Fictmix");
    assert_eq!(
        (mix.group.as_str(), mix.identity_category.as_str()),
        ("lab_artifact", "lab_artifact")
    );
}

#[test]
fn a_campaign_without_curation_tables_curates_nothing_and_raises_nothing() {
    let none = TableCuration::from_texts(None, None, None).expect("empty tables");
    assert!(none.is_empty());
    let f = feature(1, 2, "Caffeine [M+H]+");
    let t = identity_table(&[f], &sites(), &none, &DROP_GROUPS);
    assert_eq!(
        (t[0].identity.as_str(), t[0].group.as_str(), t[0].is_cec),
        ("Caffeine", "natural", false)
    );
    assert!(t[0].cec_label.is_none() && t[0].cec_category.is_none());
    let fx = fixture("campagna_sintetica");
    let nothing = |_: &str| None;
    let list = |_: &str| Vec::new();
    let io = Io::new(&nothing, &list);
    assert!(TableCuration::load(&fx.campaign, &io)
        .expect("missing files")
        .is_empty());
}

#[test]
fn a_table_without_its_columns_is_an_error() {
    assert!(TableCuration::from_texts(Some("a;b\n1;2\n"), None, None).is_err());
}

#[test]
fn the_2b_sub_level_follows_the_chemists_decisions() {
    let mut a = feature(1, 2, "Fictol");
    let mut b = feature(2, 2, "Fictol");
    a.level_sub = "b".into();
    b.level_sub = "b".into();
    let t = identity_table(
        &[a.clone(), b.clone()],
        &sites(),
        &BasicCuration,
        &DROP_GROUPS,
    );
    assert_eq!(t[0].best_level_sub, "b");
    b.level_sub = "a".into();
    assert_eq!(
        identity_table(&[a, b], &sites(), &BasicCuration, &DROP_GROUPS)[0].best_level_sub,
        "a"
    );
}

#[test]
fn skeleton_and_patterns() {
    assert_eq!(inchikey_skeleton(" bsyddsxx-yyy "), None);
    assert_eq!(
        inchikey_skeleton(" rygnmdkuyyrjfh-uhfffaoysa-n ").as_deref(),
        Some("RYGNMDKUYYRJFH")
    );
    assert!(
        Pattern::new(r"(?<!nor)ketamine").search("ketamine")
            && !Pattern::new(r"(?<!nor)ketamine").search("norketamine")
    );
    assert!(Pattern::new(r"a{2,3}b").search("xaab") && !Pattern::new(r"a{2,3}b").search("xab"));
    assert!(
        Pattern::new(r"\bcat\b").search("a cat sat")
            && !Pattern::new(r"\bcat\b").search("concatenate")
    );
    assert!(!Pattern::new("(unclosed").is_valid() && !Pattern::new("(unclosed").search("unclosed"));
    assert_eq!(split_rt_isomer("X isomer, RT 8.0 min"), Some(("X", "8.0")));
    assert_eq!(split_rt_isomer("X isomer, RT 8.0.1 min"), None);
}
