//! The browser runs `spectra.ms2`, `spectra.match` and `table.match` with the library functions the CLI wraps (`mzlab_feat::flow_ms2`, `mzlab_lib::flowmatch`, `mzlab_feat_table_wasm::rows_mgf`). Each test runs the plan
//! natively and calls the library function the page calls, with the resolved parameters the page sends, and asks for the same bytes.

use std::path::Path;

use mzlab_core::budget::sha256_hex;
use mzlab_core::flow::canonical;
use mzlab_feat::flow_ms2::{ms2_text, Ms2Params};
use mzlab_feat::store::Slice;

use crate::flow_ident_tests::{fixture, mix_fixture};

/// The resolved parameters of the instance `id` of the plan as the canonical JSON the page sends to the `.wasm`.
fn resolved(plan: &str, id: &str) -> String {
    let plan = mzlab_core::flow::parse(plan.as_bytes()).unwrap();
    let inst = plan.instance(id).unwrap();
    let steps = crate::flow::steps();
    let step = steps.iter().find(|s| s.id == inst.step).unwrap();
    canonical(inst, step).unwrap().text()
}

/// `match_json` (the export of the `lib` .wasm) against the CLI on every algorithm the browser runs, with and without decoys.
#[test]
fn match_in_the_library_gives_the_bytes_of_the_cli() {
    for alg in ["spectral_entropy", "cosine", "modified_cosine"] {
        for params in ["{}", r#"{"decoys":"spectrum","top_n":3}"#] {
            let dir = mix_fixture(&format!("web_match_{alg}"));
            let native = crate::flow_ident_tests::run(&dir, alg, params);
            assert!(!native.is_empty());
            let made = std::fs::read_to_string(dir.join("work/m.json")).unwrap();
            let plan = crate::flow_ident_tests::plan(alg, params);
            let (q, l) = (
                std::fs::read(dir.join("q.mgf")).unwrap(),
                std::fs::read(dir.join("lib.msp")).unwrap(),
            );
            let web = mzlab_lib::flowmatch::match_json(
                "spectra.match",
                alg,
                &resolved(&plan, "m"),
                ("q.mgf", &q),
                ("lib.msp", &l),
            )
            .unwrap();
            assert_eq!(web, made, "{alg} {params}");
            let _ = std::fs::remove_dir_all(&dir);
        }
    }
    // an error comes back as the JSON the page reads
    let q = b"BEGIN IONS\nEND IONS\n";
    let e = mzlab_lib::flowmatch::match_json(
        "spectra.match",
        "tree",
        "{}",
        ("q.mgf", q),
        ("lib.msp", q),
    )
    .unwrap_err();
    assert!(
        e.contains("\"error_key\":\"err.flow.match.algorithm\""),
        "{e}"
    );
}

/// `ms2_text` with `Ms2Params::from_json` (what `flow_ms2` of the `hr` .wasm calls, one file at a time) against the plan run by the CLI, with and without a threshold.
#[test]
fn ms2_in_the_library_gives_the_bytes_of_the_cli() {
    let mzml = crate::flow_ms2_tests::dda_mzml();
    for params in ["{}", r#"{"min_intensity":500,"max_isolation":2}"#] {
        let dir: &Path = &fixture("web_ms2");
        std::fs::write(dir.join("dda.mzML"), &mzml).unwrap();
        let plan = format!(
            r#"{{"format":"mzflow/2","inputs":[{{"name":"dda.mzML","type":"mzml"}}],"steps":[{{"id":"s","step":"spectra.ms2@1","in":{{"source":["file:dda.mzML"]}},"params":{params}}}]}}"#
        );
        let (run, prov) =
            crate::flow::run_plan(plan.as_bytes(), dir, &dir.join("work"), 1, None).unwrap();
        assert!(
            run.skipped.is_empty() && !prov.contains("\"error\""),
            "{prov}"
        );
        let made = std::fs::read_to_string(dir.join("work/s.mgf")).unwrap();
        let p = Ms2Params::from_json(&resolved(&plan, "s")).unwrap();
        let web = ms2_text(&p, &[("dda.mzML", &mzml)], None).unwrap();
        assert_eq!(web, made, "{params}");
        let _ = std::fs::remove_dir_all(dir);
    }
}

/// The MGF of the rows of the table (`rows_mgf`, the function of the `table` .wasm) and `match_json` against the `table.match` run by the CLI. Skipped without python3 and numpy.
#[test]
fn table_match_in_the_libraries_gives_the_bytes_of_the_cli() {
    use crate::flow_link_tests::{hypotheses, series};
    use crate::flow_tablematch_tests::{plan, run, NAMES};
    let Some(out) = series("web_tm") else { return };
    let dir = crate::flow_link_tests::fixture(&out, "", &hypotheses());
    std::fs::write(dir.join("lib.mgf"), "BEGIN IONS\nEND IONS\n").unwrap();
    run(&dir, "");
    let w = dir.join("work");
    let table = std::fs::read(w.join("gap.mzfeat")).unwrap();
    let samples = std::fs::read(dir.join("samples.json")).unwrap();
    let step = r#",{"id":"tm","step":"table.match@1","in":{"table":"gap.table","samples":"file:samples.json","index":["idx.index"],"library":"file:lib.mgf"},"params":{"tol_da":0.02,"precursor_ppm":10,"top_n":50,"min_shared":1,"min_score":0.5}}"#;
    let params = |p: &str| -> Vec<(String, mzlab_core::json::Value)> {
        match mzlab_core::json::parse(p.as_bytes()).unwrap() {
            mzlab_core::json::Value::Obj(o) => o,
            _ => unreachable!(),
        }
    };
    let open = |k: usize| crate::flow_base::open(&w.join(format!("idx-{}.mzidx", NAMES[k])));
    let all = resolved(&plan(step), "tm");
    let mgf = mzlab_feat_table_wasm::rows_mgf::rows_mgf(
        &mut Slice(&table),
        table.len() as u64,
        &samples,
        &params(&all),
        &sha256_hex(&table),
        open,
    )
    .unwrap();
    assert!(
        mgf.contains("ACCESSION="),
        "the series has no MS2 for the table"
    );
    // a library made of the rows, named after them: every row finds its own entry
    let lib: String = mgf
        .lines()
        .map(|l| match l.strip_prefix("ACCESSION=") {
            Some(id) => format!("NAME=Self{id}"),
            None => l.to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(dir.join("lib.mgf"), &lib).unwrap();
    run(&dir, step);
    let made = std::fs::read_to_string(w.join("tm.json")).unwrap();
    let web = mzlab_lib::flowmatch::match_json(
        "table.match",
        "",
        &resolved(&plan(step), "tm"),
        ("rows.mgf", mgf.as_bytes()),
        ("lib.mgf", lib.as_bytes()),
    )
    .unwrap();
    assert_eq!(web, made);
    let _ = std::fs::remove_dir_all(&dir);
}
