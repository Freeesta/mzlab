//! `tp.position@1` in a plan over a `.mzfeat` with the block `interp/1` written by the real store on invented rows of a textbook molecule (4-ethylphenol,
//! `+O`): the default thresholds give back the `site` the block stored, other thresholds make the site again from the consensus MS2 of the row, and a
//! parent without a structure is refused.

use std::path::Path;

use mzlab_core::elements::parse_formula;
use mzlab_core::json::{round, Out};
use mzlab_feat::chem::grafo::Mol;
use mzlab_feat::chem::posizioni::{expected_fragments, sites, Peak, Pol};
use mzlab_feat::interp::sito::{analyse_with, candidates, Facts, SiteParams};
use mzlab_feat::interp::sito_json::site_json;
use mzlab_feat::interp::store::append;
use mzlab_feat::recipe::Recipe;
use mzlab_feat::store::{write_table, Origin, Slice};
use mzlab_feat::table::{Aligned, CELL_DETECTED};

const PHENOL: &str = "CCc1ccc(O)cc1";
const FACTS: Facts = Facts {
    scans: 3,
    clean: 3,
    isomers: "single",
    has_reference: true,
};

/// The peaks the cleavages of one candidate site predict, rounded as the block writes the consensus.
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

fn site_of(p: &[Peak]) -> String {
    let c = candidates(
        PHENOL,
        &parse_formula("C8H10O").unwrap(),
        &parse_formula("C8H10O2").unwrap(),
    )
    .unwrap();
    let params = SiteParams::default();
    site_json(&analyse_with(&c, p, Pol::Pos, &FACTS, None, &params))
}

fn row(id: u32, p: &[Peak]) -> (u32, String) {
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
    o.raw(&site_of(p)).raw(",\"drawing\":null,\"hidden\":null}");
    (id, o.0)
}

fn tp_file(recipe: &Recipe) -> Vec<u8> {
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
    let bytes = write_table(&t, recipe, &Origin::default());
    let rows = [row(1, &peaks(1)), row(2, &peaks(3))];
    let mut out = Vec::new();
    let meta = "{\"version\":1}";
    append(
        &mut Slice(&bytes),
        bytes.len() as u64,
        &mut out,
        meta,
        &rows,
    )
    .unwrap();
    out
}

fn dir(tag: &str, parent: &str) -> std::path::PathBuf {
    dir_with(tag, parent, &Recipe::default())
}

fn dir_with(tag: &str, parent: &str, recipe: &Recipe) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_cli_position_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("work")).unwrap();
    std::fs::write(d.join("tp.mzfeat"), tp_file(recipe)).unwrap();
    std::fs::write(d.join("parent.txt"), parent).unwrap();
    d
}

fn run(d: &Path, params: &str) -> Result<String, String> {
    run_with(d, params, "msn_tree")
}

fn run_with(d: &Path, params: &str, algorithm: &str) -> Result<String, String> {
    let plan = format!(
        r#"{{"format":"mzflow/2","inputs":[{{"name":"tp.mzfeat","type":"tp"}},{{"name":"parent.txt","type":"structure"}}],"steps":[
        {{"id":"pos","step":"tp.position@1","algorithm":"{algorithm}","in":{{"tp":"file:tp.mzfeat","parent":"file:parent.txt"}},"params":{params}}}]}}"#
    );
    let (done, prov) = crate::flow::run_plan(plan.as_bytes(), d, &d.join("work"), 1, None)
        .map_err(|e| e.key.to_string())?;
    if !done.skipped.is_empty() || prov.contains("\"error\"") {
        return Err(prov);
    }
    Ok(std::fs::read_to_string(d.join("work/pos.json")).unwrap())
}

#[test]
fn the_plan_gives_the_stored_sites_and_other_thresholds_make_them_again() {
    let d = dir("sites", PHENOL);
    let got = run(&d, "{}").unwrap();
    assert!(got.starts_with("{\"schema\":\"sites/1\""), "{got}");
    assert!(got.contains(&site_of(&peaks(1))) && got.contains(&site_of(&peaks(3))));
    assert!(!got.contains("\"recomputed\":true"));
    assert_eq!(got, run(&d, "{}").unwrap());

    let strict = run(&d, r#"{"min_informative":1000}"#).unwrap();
    assert!(
        strict.contains("\"recomputed\":true") && strict.contains("few_fragments"),
        "{strict}"
    );
    // the other thresholds reach the engine too: only the strongest fragment is informative at 100 %, and the weights of the algorithm are read
    let rel = run(&d, r#"{"min_rel":100}"#).unwrap();
    assert!(rel.contains("few_fragments"), "{rel}");
    let flat = r#"{"ll_support":0,"ll_against":0}"#;
    let by_ll = run_with(&d, flat, "cleavage_likelihood").unwrap();
    assert!(by_ll.contains("\"recomputed\":true"), "{by_ll}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_parent_without_a_structure_is_refused() {
    let d = dir("nosmiles", "C8H10O");
    assert!(run(&d, "{}").is_err());
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_polarity_of_the_recipe_reaches_the_sites() {
    let mut recipe = Recipe::default();
    recipe.detect.polarity = Some(-1);
    let neg = dir_with("neg", PHENOL, &recipe);
    let pos = dir("pos", PHENOL);
    // a recomputation: the expected fragments of the two polarities are not the same ions
    let params = r#"{"min_informative":1000}"#;
    let (n, p) = (run(&neg, params).unwrap(), run(&pos, params).unwrap());
    assert_ne!(n, p);
    assert_eq!(n, run(&neg, params).unwrap());
    let _ = std::fs::remove_dir_all(&neg);
    let _ = std::fs::remove_dir_all(&pos);
}
