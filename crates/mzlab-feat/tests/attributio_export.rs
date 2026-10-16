#![cfg(feature = "attributio")]
//! Parity of the exports (quantification, gradient, Schymanski folder, publication table, evidence) with the golden data of the reference tool.

#[macro_use]
mod attributio_comune;
use attributio_comune::*;
use mzlab_feat::attributio::annotate::{gnps_comparer, load_annotations};
use mzlab_feat::attributio::curation::TableCuration;
use mzlab_feat::attributio::gradient::{
    compute_gradient_areas, compute_gradient_areas_by_sorbent, feature_cloud_gradient,
    identity_gradient, same_sorbent_default, FeatureIdentity, GradIdentity, GRADIENT_COLUMNS,
};
use mzlab_feat::attributio::io::Io;
use mzlab_feat::attributio::quantify::{
    load_calibration, quantify_lvl1, quantify_lvl1_environmental, ENV_COLUMNS, QUANT_COLUMNS,
};
use mzlab_feat::attributio::sources::load_sources;
use serde_json::{json, Value};

fn sheet_json(head: &[&str], rows: Vec<Vec<mzlab_feat::attributio::frame::Cell>>) -> Value {
    json!({"head": head, "rows": rows.iter().map(|r| r.iter().map(cell_json).collect::<Vec<_>>()).collect::<Vec<_>>()})
}

fn only(g: &Value) -> Value {
    json!({"head": g["head"], "rows": g["rows"]})
}

#[test]
fn quantification_matches_the_reference() {
    let fx = fixture_overlay(&["campagna_sintetica", "campagna_esportazione"]);
    with_io!(fx, io);
    let src = load_sources(&fx.campaign, &io).expect("sources");
    let cur = TableCuration::load(&fx.campaign, &io).expect("curation");
    let ann = load_annotations(&fx.campaign, &src, None, &cur, &gnps_comparer).expect("ann");
    let mut warnings = Vec::new();
    let cal = load_calibration(&fx.campaign, &io, &mut warnings).expect("calibration");
    let g = golden("esportazione", "quantificazione.json");
    let q = quantify_lvl1(&fx.campaign, &src, &cal, &ann.features, None).expect("lvl1");
    let head: Vec<&str> = QUANT_COLUMNS.to_vec();
    same(
        &sheet_json(&head, q.iter().map(|r| r.cells(false)).collect()),
        &only(&g["lvl1"]),
        "lvl1",
    );
    let (e, warning) =
        quantify_lvl1_environmental(&fx.campaign, &src, &cal, &ann.features, None).expect("env");
    let head: Vec<&str> = QUANT_COLUMNS
        .iter()
        .chain(ENV_COLUMNS.iter())
        .copied()
        .collect();
    same(
        &sheet_json(&head, e.iter().map(|r| r.cells(true)).collect()),
        &only(&g["env"]),
        "env",
    );
    assert_eq!(json!(warning), g["avviso"]);
}

fn present(row: &[Value], head: &[Value], site: &str) -> bool {
    let k = head
        .iter()
        .position(|h| h == &json!(format!("in_{site}")))
        .expect("in_ column");
    row[k] == json!(true)
}

#[test]
fn gradient_matches_the_reference() {
    let fx = fixture("campagna_gradiente");
    with_io!(fx, io);
    let src = load_sources(&fx.campaign, &io).expect("sources");
    let g = golden("esportazione", "gradiente.json");
    let sites: Vec<String> = serde_json::from_value(g["sites"].clone()).expect("sites");
    let st = [sites[0].as_str(), sites[1].as_str()];
    assert_eq!(
        same_sorbent_default(&fx.campaign),
        g["same_sorbent_default"].as_bool().unwrap()
    );
    let ih = g["identities"]["head"].as_array().expect("head");
    let ids: Vec<GradIdentity> = g["identities"]["rows"]
        .as_array()
        .expect("rows")
        .iter()
        .map(|r| {
            let r = r.as_array().expect("row");
            GradIdentity {
                name: r[0].as_str().expect("name").to_string(),
                present: Some((present(r, ih, st[0]), present(r, ih, st[1]))),
            }
        })
        .collect();
    let fi: Vec<FeatureIdentity> = g["feature_identities"]["rows"]
        .as_array()
        .expect("rows")
        .iter()
        .map(|r| FeatureIdentity {
            pol: r[0].as_str().expect("pol").to_string(),
            id: r[1].as_i64().expect("id"),
            rt: r[2].as_f64(),
            name: r[3].as_str().expect("name").to_string(),
        })
        .collect();
    for (key, same_sorbent) in [("stesso_sorbente", None), ("senza_sorbente", Some(false))] {
        let (rows, s) = identity_gradient(&fx.campaign, &src, &ids, &fi, st, 3.0, same_sorbent)
            .expect("gradient");
        let head: Vec<&str> = GRADIENT_COLUMNS.to_vec();
        same(
            &sheet_json(&head, rows.iter().map(|r| r.cells()).collect()),
            &only(&g[key]["table"]),
            key,
        );
        let want = &g[key]["summary"];
        let got = json!({
            "fold_threshold": s.fold_threshold, "n_both": s.n_both, "n_us_enriched": s.n_us_enriched,
            "n_ds_enriched": s.n_ds_enriched, "n_within_fold": s.n_within_fold, "n_us_only": s.n_us_only,
            "n_ds_only": s.n_ds_only, "site_us": s.site_us, "site_ds": s.site_ds,
            "n_not_comparable": s.n_not_comparable, "n_sorbent_discordant": s.n_sorbent_discordant,
        });
        same(&got, want, &format!("{key}.summary"));
    }
    let ann = load_annotations(
        &fx.campaign,
        &src,
        None,
        &mzlab_feat::attributio::annotate::BasicCuration,
        &gnps_comparer,
    )
    .expect("ann");
    let cloud = feature_cloud_gradient(&fx.campaign, &src, &ann.features, st).expect("cloud");
    let head: Vec<&str> = mzlab_feat::attributio::gradient::CLOUD_COLUMNS.to_vec();
    same(&sheet_json(&head, cloud), &only(&g["cloud"]), "cloud");
    let areas = compute_gradient_areas(&fx.campaign, &src, st, &["POS", "NEG"]).expect("areas");
    for (k, want) in g["areas"].as_object().expect("areas") {
        let (p, s) = k.split_once('|').expect("key");
        let m = &areas[&(p.to_string(), s.to_string())];
        for (id, v) in want.as_object().expect("ids") {
            assert_eq!(
                m[&id.parse::<i64>().unwrap()],
                v.as_f64().unwrap(),
                "{k} {id}"
            );
        }
    }
    let by =
        compute_gradient_areas_by_sorbent(&fx.campaign, &src, st, &["POS", "NEG"]).expect("by");
    for (k, want) in g["areas_sorbent"].as_object().expect("areas") {
        let mut it = k.split('|');
        let key = (
            it.next().unwrap().to_string(),
            it.next().unwrap().to_string(),
            it.next().unwrap().to_string(),
        );
        for (id, v) in want.as_object().expect("ids") {
            assert_eq!(
                by[&key][&id.parse::<i64>().unwrap()],
                v.as_f64().unwrap(),
                "{k} {id}"
            );
        }
    }
}

fn sheets_json(sheets: &[mzlab_feat::attributio::export::cells::Sheet]) -> Value {
    json!(sheets
        .iter()
        .map(|s| json!({
            "name": s.name,
            "head": s.head,
            "rows": s.rows.iter().map(|r| r.iter().map(cell_json).collect::<Vec<_>>()).collect::<Vec<_>>(),
        }))
        .collect::<Vec<_>>())
}

#[test]
fn schymanski_matches_the_reference() {
    let fx = fixture_overlay(&["campagna_sintetica", "campagna_esportazione"]);
    with_io!(fx, io);
    let sheets = mzlab_feat::attributio::export::schymanski::build(&fx.campaign, &io, None)
        .expect("schymanski");
    let g = golden("esportazione", "schymanski.json");
    let mut got = sheets_json(&sheets);
    in_reference_words(&mut got);
    same(&got, &g["sheets"], "schymanski");
}

/// The labels of the levels that the port writes with the sub-level (`L3a`) are the reference tool's `Lvl 3`: the one deliberate difference of the words.
fn in_reference_words(sheets: &mut Value) {
    const PAIRS: [(&str, &str); 5] = [
        (" (L1)", " (Lvl 1)"),
        (" (L2a)", " (Lvl 2a)"),
        (" (L2b)", " (Lvl 2b)"),
        (" (L3a)", " (Lvl 3)"),
        (" (L3b)", " (Lvl 3)"),
    ];
    for sheet in sheets.as_array_mut().expect("sheets") {
        let Some(k) = sheet["head"]
            .as_array()
            .and_then(|h| h.iter().position(|c| c == "level_label"))
        else {
            continue;
        };
        for row in sheet["rows"].as_array_mut().expect("rows") {
            if let Some(t) = row[k].as_str() {
                let mut t = t.to_string();
                for (a, b) in PAIRS {
                    if let Some(p) = t.strip_suffix(a) {
                        t = format!("{p}{b}");
                        break;
                    }
                }
                row[k] = json!(t);
            }
        }
    }
}

#[test]
fn publication_matches_the_reference() {
    use mzlab_feat::attributio::export::publication::build;
    use mzlab_feat::attributio::export::publication_readme::{readme, Provenance, ReadmeInput};
    let fx = fixture_overlay(&["campagna_sintetica", "campagna_esportazione"]);
    with_io!(fx, io);
    let p = build(&fx.campaign, &io, Some("all")).expect("publication");
    let g = golden("esportazione", "pubblicazione.json");
    assert_eq!(json!(p.sites), g["sites"]);
    let sheets = sheets_json(&[p.identities.clone(), p.excluded.clone()]);
    same(&only(&sheets[0]), &only(&g["identities"]), "identities");
    same(&only(&sheets[1]), &only(&g["excluded"]), "excluded");
    let counts: Value = p
        .counts
        .pairs()
        .iter()
        .map(|(k, v)| (k.to_string(), json!(v)))
        .collect::<serde_json::Map<_, _>>()
        .into();
    assert_eq!(counts, g["counts"], "counts");
    assert_eq!(p.identities.csv(), g["csv"].as_str().expect("csv"), "csv");
    let prov = Provenance::default();
    let rd = readme(&ReadmeInput {
        campaign: &fx.campaign,
        counts: &p.counts,
        sites: &p.sites,
        short: &p.short,
        sorbents: &p.sorbents,
        title: "Title",
        description: "Description",
        prov: &prov,
    })
    .expect("readme");
    let mut got = sheets_json(&[rd])[0].clone();
    let mut want = only(&g["readme"]);
    // The reference writes `control sample None` when no control was supplied; the port says so in words.
    for row in want["rows"].as_array_mut().expect("rows") {
        if let Some(t) = row[1].as_str() {
            row[1] = json!(t.replace("control sample None", "control sample (none supplied)"));
        }
    }
    // The provenance rows carry the stamp of the run: compared only by their labels.
    for v in [&mut got, &mut want] {
        let rows = v["rows"].as_array_mut().expect("rows");
        let at = rows.len() - 5;
        for r in &mut rows[at..] {
            r[1] = json!("");
        }
    }
    same(&only(&got), &want, "readme");
}
