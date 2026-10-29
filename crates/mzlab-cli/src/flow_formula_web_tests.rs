//! The browser runs `formula.ms1`, `formula.ms2` and `annot.level` with the library functions the CLI wraps (`mzlab_feat::flow_formula_ms1`, `flow_formula_ms2`, `flow_level`). Each test runs the plan
//! natively on the synthetic caffeine file and calls the function the page calls, with the resolved parameters the page sends, and asks for the same bytes.

use mzlab_feat::flow_formula_ms1::{ms1_join, ms1_rows, ms1_text};
use mzlab_feat::flow_formula_ms2::ms2_text;
use mzlab_feat::flow_level::level_text;
use mzlab_feat::flow_ms2::json_params;

use crate::flow_ident_tests::{caffeine_mgf, caffeine_mzml, fixture};
use crate::flow_match_web_tests::resolved;

const PLAN: &str = r#"{"format":"mzflow/2","inputs":[{"name":"caf.mzML","type":"mzml"},{"name":"caf.mgf","type":"spectra"},{"name":"hits.json","type":"candidates"}],"steps":[
    {"id":"t","step":"features.table@1","in":{"source":["file:caf.mzML"]},"params":{"polarity":1}},
    {"id":"f1","step":"formula.ms1@1","in":{"features":"t.report","index":["file:caf.mzML"]},"params":{"tol_ppm":3,"max_results":4}},
    {"id":"f2","step":"formula.ms2@1","in":{"spectra":"file:caf.mgf","formulas":"f1.formulas"}},
    {"id":"f3","step":"formula.ms2@1","in":{"spectra":"file:caf.mgf","formulas":"f1.formulas"},"algorithm":"h_rearrangement"},
    {"id":"l","step":"annot.level@1","in":{"candidates":"file:hits.json","formulas":"f1.formulas"}}]}"#;

/// One library hit on the feature of the file, in the `candidates/1` the matcher writes.
fn hits(feature: &str) -> String {
    format!(
        r#"{{"format":"candidates/1","hits":[{{"query":"{feature}","source":"library","algorithm":"spectral_entropy","name":"caffeine","inchikey":"RYYVLZVUVIJVGH-UHFFFAOYSA-N","score":0.95,"shared":8,"mass_ppm":1.0,"library_peaks":9}}]}}"#
    )
}

#[test]
fn formulas_and_levels_in_the_libraries_give_the_bytes_of_the_cli() {
    let dir = fixture("web_formula");
    let (mzml, mgf) = (caffeine_mzml(), caffeine_mgf());
    std::fs::write(dir.join("caf.mzML"), &mzml).unwrap();
    std::fs::write(dir.join("caf.mgf"), &mgf).unwrap();
    // the feature id is the one of the table, so the hit is written after a first run
    std::fs::write(dir.join("hits.json"), hits("0")).unwrap();
    let first = PLAN.replace(
        r#",
    {"id":"l","step":"annot.level@1","in":{"candidates":"file:hits.json","formulas":"f1.formulas"}}"#,
        "",
    );
    crate::flow::run_plan(first.as_bytes(), &dir, &dir.join("w0"), 1, None).unwrap();
    let f1 = mzlab_core::json::parse(&std::fs::read(dir.join("w0/f1.json")).unwrap()).unwrap();
    let id = f1.get("features").unwrap().arr()[0]
        .get("id")
        .and_then(|v| v.str())
        .unwrap()
        .to_string();
    std::fs::write(dir.join("hits.json"), hits(&id)).unwrap();
    let (run, prov) =
        crate::flow::run_plan(PLAN.as_bytes(), &dir, &dir.join("work"), 1, None).unwrap();
    assert!(
        run.skipped.is_empty() && !prov.contains("\"error\""),
        "{prov}"
    );
    let made = |n: &str| std::fs::read_to_string(dir.join("work").join(n)).unwrap();

    // formula.ms1: the whole text, and the rows of each file joined as the page does
    let table = std::fs::read(dir.join("work/t.csv")).unwrap();
    let p1 = json_params(&resolved(PLAN, "f1")).unwrap();
    assert_eq!(
        ms1_text(&p1, &table, &[("caf.mzML", &mzml)]).unwrap(),
        made("f1.json")
    );
    let rows = ms1_rows(&p1, &table, &["caf.mzML"], (0, &mzml)).unwrap();
    let joined: Vec<String> = rows.into_iter().map(|r| r.1).collect();
    assert_eq!(ms1_join(&joined), made("f1.json"));
    // a feature the file does not hold is refused as the CLI does
    let e = ms1_text(&p1, &table, &[]).unwrap_err();
    assert_eq!(e.key, "err.flow.formula.file");

    // formula.ms2, both algorithms
    let f1_text = made("f1.json");
    for (id, alg) in [("f2", ""), ("f3", "h_rearrangement")] {
        let p = json_params(&resolved(PLAN, id)).unwrap();
        let web = ms2_text(alg, &p, ("caf.mgf", mgf.as_bytes()), &[f1_text.as_bytes()]).unwrap();
        assert_eq!(web, made(&format!("{id}.json")), "{id}");
    }

    // annot.level: the hit and the formulas of the feature give a level other than the one without evidence
    let level = level_text(&[hits(&id).as_bytes()], &[f1_text.as_bytes()]).unwrap();
    assert_eq!(level, made("l.json"));
    assert!(
        !level.contains("\"level\":\"5\"") && !level.contains("\"level\":\"0\""),
        "{level}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
