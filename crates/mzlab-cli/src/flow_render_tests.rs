//! `structure.render@1` in a plan: a `sites/1` file of invented rows of a textbook molecule (4-ethylphenol, `+O`) gives, for each row, the bytes of the
//! `drawing` that `interp::disegno` makes from the report (a site, a ring region, an undetermined row, a row with no site), twice the same.

use std::path::Path;

use mzlab_core::elements::parse_formula;
use mzlab_feat::chem::grafo::Mol;
use mzlab_feat::chem::posizioni::{expected_fragments, sites, Peak, Pol};
use mzlab_feat::interp::disegno::drawing;
use mzlab_feat::interp::sito::{analyse, candidates, undetermined, Facts, SiteReport};
use mzlab_feat::interp::sito_json::{drawing_json, site_json};

const PHENOL: &str = "CCc1ccc(O)cc1";
const FACTS: Facts = Facts {
    scans: 3,
    clean: 3,
    isomers: "single",
    has_reference: true,
};

fn peaks(atom: usize) -> Vec<Peak> {
    let mol = Mol::parse(PHENOL).unwrap();
    let all = sites(&mol, "+O").unwrap();
    let site = all.iter().find(|x| x.atom == atom).unwrap();
    let mut p: Vec<Peak> = expected_fragments(&mol, site, Pol::Pos)
        .iter()
        .enumerate()
        .map(|(i, f)| Peak {
            mz: f.mz,
            inten: 100.0 - i as f64,
        })
        .collect();
    p.sort_by(|a, b| a.mz.total_cmp(&b.mz));
    p.dedup_by(|a, b| (a.mz - b.mz).abs() < 1e-6);
    p
}

fn report(p: &[Peak]) -> SiteReport {
    let c = candidates(
        PHENOL,
        &parse_formula("C8H10O").unwrap(),
        &parse_formula("C8H10O2").unwrap(),
    )
    .unwrap();
    analyse(&c, p, Pol::Pos, &FACTS, None)
}

/// The reports of the rows, and the `sites/1` text that carries their `site` (the fourth row has none).
fn rows() -> (Vec<SiteReport>, String) {
    let reports = vec![
        report(&peaks(1)),
        report(&peaks(3)),
        undetermined("+O", vec!["no_ms2"]),
    ];
    let mut items: Vec<String> = reports
        .iter()
        .enumerate()
        .map(|(i, r)| {
            format!(
                "{{\"id\":{i},\"recomputed\":false,\"site\":{}}}",
                site_json(r)
            )
        })
        .collect();
    items.push("{\"id\":3,\"recomputed\":false,\"site\":null}".into());
    (
        reports,
        format!(
            "{{\"schema\":\"sites/1\",\"parent\":\"{PHENOL}\",\"rows\":[{}]}}",
            items.join(",")
        ),
    )
}

fn dir(tag: &str, sites: &str, parent: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_cli_render_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("work")).unwrap();
    std::fs::write(d.join("sites.json"), sites).unwrap();
    std::fs::write(d.join("parent.txt"), parent).unwrap();
    d
}

fn run(d: &Path) -> Result<String, String> {
    let plan = r#"{"format":"mzflow/2","inputs":[{"name":"sites.json","type":"transform"},{"name":"parent.txt","type":"structure"}],"steps":[
        {"id":"r","step":"structure.render@1","in":{"sites":"file:sites.json","parent":"file:parent.txt"}}]}"#;
    let (run, prov) = crate::flow::run_plan(plan.as_bytes(), d, &d.join("work"), 1, None)
        .map_err(|e| e.key.to_string())?;
    if !run.skipped.is_empty() || prov.contains("\"error\"") {
        return Err(prov);
    }
    Ok(std::fs::read_to_string(d.join("work/r.json")).unwrap())
}

#[test]
fn every_row_is_the_drawing_of_the_engine() {
    let (reports, sites_text) = rows();
    let d = dir("rows", &sites_text, PHENOL);
    let got = run(&d).unwrap();
    let mut want = format!("{{\"schema\":\"drawings/1\",\"parent\":\"{PHENOL}\",\"rows\":[");
    for (i, r) in reports.iter().enumerate() {
        want += &format!(
            "{}{{\"id\":{i},\"drawing\":{}}}",
            if i > 0 { "," } else { "" },
            drawing_json(&drawing(PHENOL, r))
        );
    }
    want += ",{\"id\":3,\"drawing\":null}]}";
    assert_eq!(got, want);
    assert!(
        got.contains("\"mode\":\"structure\"") && got.contains("\"mode\":\"multicenter\""),
        "{got}"
    );
    assert_eq!(got, run(&d).unwrap());
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_parent_without_a_structure_or_a_wrong_file_is_refused() {
    let (_, sites_text) = rows();
    let bare = sites_text.replace(&format!("\"parent\":\"{PHENOL}\""), "\"parent\":\"\"");
    let d = dir("nosmiles", &bare, "C8H10O");
    assert!(run(&d).is_err());
    let d2 = dir("wrong", "{\"schema\":\"other/1\"}", PHENOL);
    assert!(run(&d2).is_err());
    let _ = (std::fs::remove_dir_all(&d), std::fs::remove_dir_all(&d2));
}
