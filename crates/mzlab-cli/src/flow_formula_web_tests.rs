//! The browser runs `formula.ms1`, `formula.ms2` and `annot.level` with the library functions the CLI wraps (`mzlab_feat::flow_formula_ms1`, `flow_formula_ms2`, `flow_level`). Each test runs the plan
//! natively on the synthetic caffeine file and calls the function the page calls, with the resolved parameters the page sends, and asks for the same bytes.

use mzlab_core::json::Value;
use mzlab_feat::flow_formula_ms1::{ms1_join, ms1_rows, ms1_text};
use mzlab_feat::flow_formula_ms2::{ms2_text, WARN_SUB};
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
        ms1_text(&p1, &table, &[("caf.mzML", &mzml)], usize::MAX).unwrap(),
        made("f1.json")
    );
    let rows = ms1_rows(&p1, &table, &["caf.mzML"], (0, &mzml), (0, usize::MAX)).unwrap();
    let joined: Vec<String> = rows.into_iter().map(|r| r.1).collect();
    assert_eq!(ms1_join(&joined), made("f1.json"));
    // a feature the file does not hold is refused as the CLI does
    let e = ms1_text(&p1, &table, &[], usize::MAX).unwrap_err();
    assert_eq!(e.key, "err.flow.formula.file");
    // below the read estimate both routes refuse the same way, before the file is decoded
    use mzlab_core::mzml_estimate::{bytes, inputs};
    let cap = bytes(&inputs(&mzml), mzml.len()) - (2 << 20);
    let cli = ms1_text(&p1, &table, &[("caf.mzML", &mzml)], cap).unwrap_err();
    let web = ms1_rows(&p1, &table, &["caf.mzML"], (0, &mzml), (mzml.len(), cap)).unwrap_err();
    assert_eq!((cli.key, &cli.params), (web.key, &web.params));
    assert_eq!(cli.key, "error.memoryBudget");
    assert!(
        cli.params.contains(&("fase", "read".to_string())),
        "{cli:?}"
    );

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

// formula.ms2 when the sub-formula grid of an ion is over max_subformulas: the candidate is kept with no fragment and names the limit in
// both algorithms; under the limit nothing is added (synthetic caffeine spectrum, C8H10N4O2, [M+H]+).
const MGF: &str = "BEGIN IONS\nTITLE=caf\nPEPMASS=195.08765\nCHARGE=1+\nIONMODE=Positive\n138.06619 1000\n110.07127 600\n83.06037 150\nEND IONS\n";
const FORMULAS: &str =
    r#"{"features":[{"id":"F1","result":{"candidates":[{"neutral":"C8H10N4O2"}]}}]}"#;

fn run(alg: &str, max: f64) -> String {
    let p: Vec<(String, Value)> = [
        ("ppm_ion", 5.0),
        ("ppm_frag", 5.0),
        ("max_subformulas", max),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), Value::Num(v)))
    .collect();
    ms2_text(alg, &p, ("caf.mgf", MGF.as_bytes()), &[FORMULAS.as_bytes()]).unwrap()
}

#[test]
fn a_grid_over_the_limit_is_named_on_the_candidate() {
    // C8H11N4O2+: 9 x 12 x 5 x 3 = 1620 cells
    for alg in ["subformula", "h_rearrangement"] {
        let over = run(alg, 1000.0);
        assert!(
            over.contains(&format!("\"warning\":\"{WARN_SUB}\"")),
            "{alg}: {over}"
        );
        assert!(over.contains("\"n_explained\":0"), "{alg}: {over}");
        let under = run(alg, 3_000_000.0);
        assert!(!under.contains("warning"), "{alg}: {under}");
        assert!(!under.contains("\"n_explained\":0"), "{alg}: {under}");
    }
}
