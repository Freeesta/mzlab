#![cfg(feature = "attributio")]
//! The coherence checks of the registry (`attributio::checks`) on hand-made registries, the isotonic fit of RT against XLogP, and the cascade on the real
//! campaigns when `MZLAB_DATI` points at the private data (skipped without). To compare the real levels with the reference tool, run
//! `ATTRIBUTIO_SRC=<attributio/src> python3 tools/attributio_riferimento.py --campagna <campaign folder> --uscita <folder>` and set `MZLAB_ORO_LIVELLI=<folder>`
//! (the test then compares `<folder>/livelli.json` with no tolerance).

#[macro_use]
mod attributio_comune;
use mzlab_feat::attributio::annotate::{gnps_comparer, load_annotations, BasicCuration, Feature};
use mzlab_feat::attributio::campaign::{Campaign, Env};
use mzlab_feat::attributio::checks::*;
use mzlab_feat::attributio::io::{Entry, Io};
use mzlab_feat::attributio::rt_logp::{
    build_batch_fit, isotonic_fit, rt_vs_batch, FitInput, Z_FLAG,
};
use mzlab_feat::attributio::sources::load_sources;
use std::path::{Path, PathBuf};

#[path = "../../../tools/dati_percorsi.rs"]
mod dati_percorsi;

fn caffeine() -> Compound {
    Compound {
        first_block: "RYYVLZVUVIJVGH".into(),
        formula: "C8H10N4O2".into(),
        monoisotopic_mass: Some(194.080376),
        xlogp: Some(-0.1),
        xlogp_source: "PubChem".into(),
        pubchem_cid: "2519".into(),
        pubchem_title: "Caffeine".into(),
    }
}

fn member(camp: &str, name: &str, block: Option<&str>) -> Member {
    Member {
        campaign: camp.into(),
        name: name.into(),
        first_block: block.map(str::to_string),
        ik_basis: "name source".into(),
        best_level: 2,
        n_features: 1,
        group: "contaminant".into(),
        ..Member::default()
    }
}

fn evidence(camp: &str, id: i64, mz: f64, adduct: Option<&str>) -> Evidence {
    Evidence {
        campaign: camp.into(),
        identity: "Caffeine".into(),
        pol: "POS".into(),
        id,
        mz,
        rt: Some(5.0),
        adduct: adduct.map(str::to_string),
        level: 2,
        name_block: Some("RYYVLZVUVIJVGH".into()),
        ..Evidence::default()
    }
}

fn ctx() -> Context {
    let mut c = Context::default();
    c.compounds.insert("RYYVLZVUVIJVGH".into(), caffeine());
    c.campaigns = vec!["A".into()];
    c
}

#[test]
fn coverage_flags_identities_without_structure_and_structures_outside_the_registry() {
    let mut c = ctx();
    c.members = vec![
        member("A", "Caffeine", Some("RYYVLZVUVIJVGH")),
        member("A", "Unknown", None),
        member("A", "Other", Some("AAAAAAAAAAAAAA")),
    ];
    let f = check_coverage(&c);
    assert_eq!(f.len(), 2);
    assert_eq!((f[0].severity, f[0].identity.as_str()), (INFO, "Unknown"));
    assert_eq!(
        (f[1].severity, f[1].inchikey_first_block.as_str()),
        (WARNING, "AAAAAAAAAAAAAA")
    );
}

#[test]
fn mass_accepts_the_reported_adduct_and_names_the_closest_one_when_it_is_wrong() {
    let mut c = ctx();
    c.evidence = vec![
        evidence("A", 1, 195.087652, Some("[M+H]+")),
        evidence("A", 2, 217.069596, Some("[M+H]+")),
        evidence("A", 3, 217.069596, None),
        evidence("A", 4, 400.0, None),
    ];
    let f = check_mass(&c);
    assert_eq!(f.len(), 2, "{f:#?}");
    assert!(
        f[0].detail
            .starts_with("POS 2 m/z 217.0696: [M+H]+ of C8H10N4O2 expected 195.0877"),
        "{}",
        f[0].detail
    );
    assert!(
        f[0].detail.contains("; explained instead as [M+Na]+ ("),
        "{}",
        f[0].detail
    );
    assert!(matches!(f[0].value, Value::Num(v) if v > 100000.0));
    assert!(
        f[1].detail
            .starts_with("POS 4 m/z 400.0000: adduct not reported; the closest for C8H10N4O2 is"),
        "{}",
        f[1].detail
    );
}

#[test]
fn names_find_merge_split_and_the_name_that_pubchem_does_not_know() {
    let mut c = ctx();
    c.members = vec![
        member("A", "Caffeine", Some("RYYVLZVUVIJVGH")),
        member("A", "Trimethylxanthine", Some("RYYVLZVUVIJVGH")),
        member("A", "Mixture", Some("B1")),
        member("A", "Mixture", Some("B2")),
    ];
    c.pubchem = vec![Pubchem {
        cid: "2519".into(),
        title: "Caffeine".into(),
        synonyms: "1,3,7-trimethylxanthine|guaranine".into(),
    }];
    c.evidence = vec![Evidence {
        source_name: Some("Caffeine".into()),
        ..evidence("A", 1, 195.0877, Some("[M+H]+"))
    }];
    let f = check_names(&c);
    let kinds: Vec<&str> = f
        .iter()
        .map(|x| x.detail.split(':').next().unwrap_or(""))
        .collect();
    assert!(kinds.contains(&"MERGE"), "{kinds:?}");
    assert!(kinds.contains(&"SPLIT"), "{kinds:?}");
    let split = f
        .iter()
        .find(|x| x.detail.starts_with("SPLIT"))
        .expect("split");
    assert_eq!(split.inchikey_first_block, "B1|B2");
    assert!(
        f.iter().any(|x| x
            .detail
            .starts_with("name not among the PubChem synonyms of CID 2519 ('Caffeine')")
            && x.identity == "Trimethylxanthine"),
        "{f:#?}"
    );
}

#[test]
fn name_and_loader_structures_that_disagree_are_reported_once_per_identity() {
    let mut c = ctx();
    let e = Evidence {
        loader_block: Some("ZZZZZZZZZZZZZZ".into()),
        name_source: "GNPS2".into(),
        loader_source: "SIRIUS".into(),
        ..evidence("A", 1, 195.0877, None)
    };
    c.evidence = vec![e.clone(), Evidence { id: 2, ..e }];
    let f: Vec<_> = check_names(&c)
        .into_iter()
        .filter(|x| x.detail.contains("problem A"))
        .collect();
    assert_eq!(f.len(), 1);
    assert!(f[0].detail.contains("in 2 features"), "{}", f[0].detail);
    assert_eq!(f[0].value, Value::Text("ZZZZZZZZZZZZZZ".into()));
}

#[test]
fn library_entries_that_contradict_their_precursor_are_reported_by_inchikey() {
    let mut c = ctx();
    let row = |ppm: f64| MzlibCheck {
        pol: "POS".into(),
        name: "Bad entry".into(),
        inchi_key: Some("ABCDEFGHIJKLMN-XXXXXXXXXX-N".into()),
        mol_formula: "C9H9".into(),
        structure_ppm: ppm,
        structure_ok: Some(false),
    };
    c.mzlib = vec![(
        "A".into(),
        vec![
            row(300.0),
            row(-500.0),
            MzlibCheck {
                structure_ok: Some(true),
                ..row(1.0)
            },
        ],
    )];
    let f = check_names(&c);
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].inchikey_first_block, "ABCDEFGHIJKLMN");
    assert!(
        f[0].detail.contains("2 entries (POS) carry InChIKey")
            && f[0].detail.contains("(median 400 ppm)"),
        "{}",
        f[0].detail
    );
    assert_eq!(f[0].value, Value::Int(2));
}

#[test]
fn the_isotonic_fit_is_monotone_and_a_late_peak_is_an_outlier() {
    let x: Vec<f64> = (0..10).map(f64::from).collect();
    let y = [1.0, 2.0, 1.5, 3.0, 4.0, 3.5, 5.0, 6.0, 7.0, 8.0];
    let fit = isotonic_fit(&x, &y);
    assert!(fit.windows(2).all(|w| w[0] <= w[1]));
    assert_eq!(fit[1], 1.75);
    let rows: Vec<FitInput> = (0..30)
        .map(|i| FitInput {
            pol: "POS".into(),
            id: i,
            xlogp: Some(f64::from(i as i32) / 5.0),
            rt: Some(
                2.0 + f64::from(i as i32) * 0.2 + [0.0, 0.3, -0.2, 0.1, -0.3][(i % 5) as usize],
            ),
        })
        .collect();
    let fitted = build_batch_fit(&rows);
    let (pred, z) = rt_vs_batch(&fitted, "POS", 2.0, Some(12.0));
    assert!(pred.is_some() && z.expect("z").abs() > Z_FLAG);
    assert_eq!(rt_vs_batch(&fitted, "NEG", 2.0, Some(4.0)), (None, None));
    let mut c = ctx();
    c.fits.insert("A".into(), fitted);
    c.evidence = vec![Evidence {
        rt: Some(12.0),
        ..evidence("A", 1, 195.0877, None)
    }];
    c.compounds
        .get_mut("RYYVLZVUVIJVGH")
        .expect("caffeine")
        .xlogp = Some(2.0);
    let f = check_xlogp_rt(&c);
    assert_eq!(f.len(), 1);
    assert!(
        f[0].detail.starts_with("POS 1: RT 12.00 min, expected ~"),
        "{}",
        f[0].detail
    );
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
fn real_campaigns_get_a_level_and_a_basis_for_every_feature() {
    let Some(base) = dati_percorsi::radice() else {
        eprintln!("SKIP attributio real levels: MZLAB_DATI is not set");
        return;
    };
    let mut found = Vec::new();
    find_campaigns(&base, 4, &mut found);
    if found.is_empty() {
        eprintln!(
            "SKIP attributio real levels: no campaign.toml under {}",
            base.display()
        );
        return;
    }
    let oro = std::env::var_os("MZLAB_ORO_LIVELLI").map(PathBuf::from);
    for dir in found {
        check_campaign(&dir, oro.as_deref());
    }
}

/// The cascade on one real campaign: every feature has a level and a basis, and the levels equal the reference tool's when its output is given.
fn check_campaign(dir: &Path, oro: Option<&Path>) {
    let at = |p: &str| {
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
    let text =
        String::from_utf8_lossy(&std::fs::read(dir.join("campaign.toml")).expect("campaign.toml"))
            .into_owned();
    let name = dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let c = Campaign::from_toml(&text, "campaign.toml", &name, Env::default())
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
    let s = load_sources(&c, &io).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
    let a = load_annotations(&c, &s, None, &BasicCuration, &gnps_comparer)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
    let mut per = [0usize; 5];
    for f in &a.features {
        assert!(
            f.level <= 4 && !f.level_basis.is_empty(),
            "{} {} {}",
            c.id,
            f.pol,
            f.id
        );
        assert_eq!(
            f.level == 2 || f.level == 3,
            !f.level_sub.is_empty(),
            "{} {} {}: sub-level",
            c.id,
            f.pol,
            f.id
        );
        per[f.level as usize] += 1;
    }
    eprintln!(
        "REAL {}: {} features, levels 0..4 = {per:?}",
        c.id,
        a.features.len()
    );
    if let Some(o) = oro {
        same_as_reference(&c.id, &a.features, o);
    }
}

/// The levels equal the reference tool's `livelli.json` with no tolerance.
fn same_as_reference(id: &str, feats: &[Feature], o: &Path) {
    let g: serde_json::Value =
        serde_json::from_slice(&std::fs::read(o.join("livelli.json")).expect("livelli.json"))
            .expect("JSON");
    for w in g["features"].as_array().expect("features") {
        let (pol, n) = (
            w["_pol"].as_str().unwrap_or(""),
            w["_id"].as_i64().unwrap_or(-1),
        );
        let f = feats
            .iter()
            .find(|f| f.pol == pol && f.id == n)
            .expect("feature");
        let got = (f.level as i64, f.level_sub.as_str(), f.level_basis.as_str());
        let want = (
            w["level"].as_i64().expect("l"),
            w["level_sub"].as_str().unwrap_or(""),
            w["level_basis"].as_str().expect("b"),
        );
        assert_eq!(got, want, "{id} {pol} {n}");
    }
}
