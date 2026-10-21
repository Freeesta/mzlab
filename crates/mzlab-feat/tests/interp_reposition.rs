#![cfg(feature = "interp")]
//! `interp::reposition`: the sites of the interpreted rows of a `.mzfeat` again. With the default thresholds the `site` the block stored comes back byte for
//! byte; with others it is made again from the consensus MS2 of the row (the tree is not in the file, so a row voted by the tree keeps its site).

use mzlab_core::elements::parse_formula;
use mzlab_core::json::{round, Out};
use mzlab_feat::chem::grafo::Mol;
use mzlab_feat::chem::posizioni::{expected_fragments, sites, Peak, Pol};
use mzlab_feat::interp::reposition::reposition;
use mzlab_feat::interp::sito::{analyse_with, candidates, Facts, SiteParams};
use mzlab_feat::interp::sito_json::site_json;
use mzlab_feat::interp::store::append;
use mzlab_feat::recipe::Recipe;
use mzlab_feat::store::{write_table, Origin, Slice};
use mzlab_feat::table::{Aligned, CELL_DETECTED};

const PHENOL: &str = "CCc1ccc(O)cc1";

/// The peaks of one candidate site, rounded as the block writes the consensus (m/z 4 decimals, relative intensity 1).
fn peaks(atom: usize) -> Vec<Peak> {
    let mol = Mol::parse(PHENOL).unwrap();
    let all = sites(&mol, "+O").unwrap();
    let site = all.iter().find(|x| x.atom == atom).unwrap();
    let mut p: Vec<Peak> = expected_fragments(&mol, site, Pol::Pos)
        .iter()
        .enumerate()
        .map(|(i, f)| Peak {
            mz: round(f.mz, 4),
            inten: round(100.0 - i as f64, 1),
        })
        .collect();
    p.sort_by(|a, b| a.mz.total_cmp(&b.mz));
    p.dedup_by(|a, b| (a.mz - b.mz).abs() < 1e-6);
    p
}

const FACTS: Facts = Facts {
    scans: 3,
    clean: 3,
    isomers: "single",
    has_reference: true,
};

fn site_of(p: &[Peak], params: &SiteParams) -> String {
    let c = candidates(
        PHENOL,
        &parse_formula("C8H10O").unwrap(),
        &parse_formula("C8H10O2").unwrap(),
    )
    .unwrap();
    site_json(&analyse_with(&c, p, Pol::Pos, &FACTS, None, params))
}

/// A row of the block: the change, the consensus MS2, three clean scans and the `site` given.
fn row(id: u32, p: &[Peak], site: &str) -> (u32, String) {
    let mut o = Out::default();
    o.raw("{\"schema\":\"interp/1\",\"id\":").int(i64::from(id));
    o.raw(",\"change\":{\"delta\":\"+O\",\"neutral_formula\":\"C8H10O2\",\"ion_formula\":\"C8H11O2\",\"ppm\":0.1,\"source\":\"rule\",\"alternatives\":[]}");
    o.raw(",\"ms1\":null,\"ms2\":{\"scans\":[");
    for k in 0..3 {
        o.raw(if k > 0 { "," } else { "" })
            .raw("{\"file\":0,\"scan\":1,\"chimeric\":false}");
    }
    o.raw("],\"files_with_ms2\":1,\"consensus\":{\"mz\":");
    o.floats(&p.iter().map(|x| x.mz).collect::<Vec<_>>());
    o.raw(",\"rel\":");
    o.floats(&p.iter().map(|x| x.inten).collect::<Vec<_>>());
    o.raw("},\"isomers\":\"single\"},\"vs_parent\":{\"ref\":\"dda\"},\"tree\":null,\"site\":");
    o.raw(site).raw(",\"drawing\":null,\"hidden\":null}");
    (id, o.0)
}

fn file(rows: &[(u32, String)]) -> Vec<u8> {
    let mut t = Aligned::with_capacity(1, 2);
    for r in 0..2 {
        t.mz.push(100.0 + r as f64);
        t.rt.push(1.0);
        t.area.push(1e6);
        t.height.push(1e5);
        t.apex_rt.push(1.0);
        t.width.push(0.1);
        t.state.push(CELL_DETECTED);
    }
    let bytes = write_table(&t, &Recipe::default(), &Origin::default());
    let mut out = Vec::new();
    append(
        &mut Slice(&bytes),
        bytes.len() as u64,
        &mut out,
        "{\"version\":1}",
        rows,
    )
    .unwrap();
    out
}

#[test]
fn the_default_thresholds_give_back_the_stored_sites_and_others_make_them_again() {
    let (a, b) = (peaks(1), peaks(3));
    let defaults = SiteParams::default();
    let rows = [
        row(1, &a, &site_of(&a, &defaults)),
        row(2, &b, &site_of(&b, &defaults)),
    ];
    let bytes = file(&rows);
    let parent = parse_formula("C8H10O").unwrap();
    let run = |p: &SiteParams| {
        reposition(
            &mut Slice(&bytes),
            bytes.len() as u64,
            (PHENOL, &parent),
            Pol::Pos,
            p,
        )
        .unwrap()
    };
    let same = run(&defaults);
    assert!(same.iter().all(|s| !s.recomputed));
    assert_eq!(same[0].site, site_of(&a, &defaults));
    assert!(
        same[0].site.contains("\"confidence\":\"site\""),
        "{}",
        same[0].site
    );
    assert!(same[1].site.contains("\"confidence\":\"region\""));

    // other thresholds: made again from the consensus of the row, like `analyse_with` on the same peaks
    let strict = SiteParams {
        min_informative: 1000,
        ..defaults.clone()
    };
    let again = run(&strict);
    assert!(again.iter().all(|s| s.recomputed));
    assert_eq!(again[0].site, site_of(&a, &strict));
    assert!(again[0].site.contains("few_fragments"), "{}", again[0].site);
    // a margin a hair wider is a different set of thresholds that gives the same answer for these rows
    let hair = SiteParams {
        margin: 2.0 + 1e-9,
        ..defaults.clone()
    };
    assert_eq!(run(&hair)[0].site, same[0].site);
}

#[test]
fn a_row_voted_by_the_tree_keeps_its_site() {
    let a = peaks(1);
    let tree =
        site_of(&a, &SiteParams::default()).replace("\"source\":\"cuts\"", "\"source\":\"tree\"");
    assert!(tree.contains("\"source\":\"tree\""));
    let bytes = file(&[row(1, &a, &tree)]);
    let parent = parse_formula("C8H10O").unwrap();
    let strict = SiteParams {
        min_informative: 1000,
        ..SiteParams::default()
    };
    let got = reposition(
        &mut Slice(&bytes),
        bytes.len() as u64,
        (PHENOL, &parent),
        Pol::Pos,
        &strict,
    )
    .unwrap();
    assert!(!got[0].recomputed);
    assert_eq!(got[0].site, tree);
}
