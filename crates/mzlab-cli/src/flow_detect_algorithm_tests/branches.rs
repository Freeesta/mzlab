//! The recipe text of the browser pool (`job_recipe_text`) over a plan with two branches, a preset on one detection and a gap filling on one alignment only:
//! the chain the page runs with it (each detection on its own index, each table on the first) gives the tables `flow run` writes, byte for byte.

use crate::flow_run_tests::{fixture, flow_run, p};
use mzlab_core::idxread::Index;
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::upload::prepare;
use mzlab_ops::run::feat::run;
use mzlab_ops::run::feat::run::meta::job_recipe_text;

const PLAN: &str = r#"{"format":"mzflow/2","inputs":[{"name":"s_t000.mzML","type":"raw"},{"name":"s_t010.mzML","type":"raw"},{"name":"s_t030.mzML","type":"raw"}],"steps":[
{"id":"idx","step":"index.mzidx@1","in":{"source":["file:s_t000.mzML","file:s_t010.mzML","file:s_t030.mzML"]}},
{"id":"detb","step":"features.detect@1","in":{"index":["idx.index"]},"params":{"min_points":3,"min_height":5000}},
{"id":"deta","step":"features.detect@1","preset":"hr","in":{"index":["idx.index"]}},
{"id":"alia","step":"features.align@1","in":{"peaks":["deta.peaks"]}},
{"id":"alib","step":"features.align@1","in":{"peaks":["detb.peaks"]},"params":{"rt_gap":0.05}},
{"id":"gapa","step":"features.gapfill@1","in":{"table":"alia.table","index":["idx.index"]}}]}"#;

/// The table the page makes for the alignment `ali` (detections of `det`): each file detected with the recipe text of `det` on its own index, the table with
/// the one of `ali` on the first.
fn page(files: &[(String, Vec<u8>)], det: &str, ali: &str) -> Vec<u8> {
    let reg = crate::flow::steps();
    let idxs: Vec<Vec<u8>> = files
        .iter()
        .map(|(name, b)| {
            let mut src = b.clone();
            let pr = prepare(&mut src, b.len() as u64, name).unwrap();
            build_mzml_to_vec(&pr.table, b, 64 << 20).unwrap().0
        })
        .collect();
    let open = |k: usize| Index::open(idxs[k].clone(), idxs[k].len() as u64);
    let dt = job_recipe_text(PLAN.as_bytes(), &reg, det, true).unwrap();
    let peaks: Vec<Vec<u8>> = files
        .iter()
        .enumerate()
        .map(|(k, (n, _))| {
            let r = run::recipe_for(&dt, &mut open(k).unwrap()).unwrap();
            run::detect(&mut open(k).unwrap(), n, &r).unwrap()
        })
        .collect();
    let at = job_recipe_text(PLAN.as_bytes(), &reg, ali, true).unwrap();
    let r = run::recipe_for(&at, &mut open(0).unwrap()).unwrap();
    let (tables, origin) = run::read_files(&peaks).unwrap();
    run::align(&tables, &origin, &r, open).unwrap()
}

#[test]
fn each_branch_of_the_plan_gets_its_own_recipe_on_the_page_as_in_flow_run() {
    let (dir, files) = fixture("recipe_branches");
    std::fs::write(dir.join("plan.json"), PLAN).unwrap();
    let out = dir.join("out");
    let o = flow_run(&[
        p(&dir.join("plan.json")),
        "--in",
        p(&dir.join("data")),
        "--out",
        p(&out),
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let read = |n: &str| std::fs::read(out.join(n)).unwrap();
    // the alignment a gap filling reads is kept filled on the page: its table is the one of the filling
    assert!(
        page(&files, "deta", "alia") == read("gapa.mzfeat"),
        "branch a (preset hr, filled)"
    );
    assert!(
        page(&files, "detb", "alib") == read("alib.mzfeat"),
        "branch b (own detection parameters, no filling)"
    );
    assert_ne!(read("alia.mzfeat"), read("alib.mzfeat"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_recipe_text_holds_only_the_line_of_the_job() {
    let reg = crate::flow::steps();
    let text = |i: &str| job_recipe_text(PLAN.as_bytes(), &reg, i, true).unwrap();
    assert_eq!(
        text("detb"),
        r#"{"fill":false,"layers":[{"step":"features.detect","params":{"min_height":5000,"min_points":3}}]}"#
    );
    // the preset writes its values, as `flow run` does
    assert!(
        text("deta").contains(r#""floor":20000"#),
        "{}",
        text("deta")
    );
    assert!(!text("alib").contains("gapfill") && text("alib").starts_with(r#"{"fill":false"#));
    assert_eq!(text("alia"), text("gapa"));
    assert!(text("gapa").starts_with(r#"{"fill":true"#) && !text("gapa").contains("min_points"));
    // the alignment alone (the record of a run makes it again without the filling)
    let alone = job_recipe_text(PLAN.as_bytes(), &reg, "alia", false).unwrap();
    assert!(
        alone.starts_with(r#"{"fill":false"#) && !alone.contains("gapfill"),
        "{alone}"
    );
}
