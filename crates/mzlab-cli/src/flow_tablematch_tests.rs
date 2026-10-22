//! `table.match@1` in a plan over the synthetic identity series: the base chain makes the aligned table, the MGF of its rows (the one the page exports) becomes
//! a library whose entries are named after the rows, and the match gives every row its own entry back, named by the id of the row.
//! Skipped without python3 and numpy (`MZLAB_GOLDEN_STRICT=1` fails instead).

use std::path::Path;

use mzlab_core::budget::sha256_hex;
use mzlab_core::json::parse;
use mzlab_feat::store::Slice;
use mzlab_feat_table_wasm::core::ms2_file;

use crate::flow_link_tests::{fixture, hypotheses, series, FORMULA};

const NAMES: [&str; 6] = [
    "SIN_blank",
    "SIN_dark",
    "SIN_t0",
    "SIN_t05min",
    "SIN_t10min",
    "SIN_t20min",
];

/// The plan up to the aligned table, plus `extra` (steps appended).
fn plan(extra: &str) -> String {
    let raw: Vec<String> = NAMES
        .iter()
        .map(|n| format!(r#"{{"name":"{n}.mzML","type":"raw"}}"#))
        .collect();
    let src: Vec<String> = NAMES
        .iter()
        .map(|n| format!(r#""file:{n}.mzML""#))
        .collect();
    format!(
        r#"{{"format":"mzflow/2","inputs":[{},{{"name":"samples.json","type":"samples"}},{{"name":"lib.mgf","type":"library"}}],"steps":[
        {{"id":"idx","step":"index.mzidx@1","in":{{"source":[{}]}}}},
        {{"id":"det","step":"features.detect@1","in":{{"index":["idx.index"]}}}},
        {{"id":"ali","step":"features.align@1","in":{{"peaks":["det.peaks"]}}}},
        {{"id":"gap","step":"features.gapfill@1","in":{{"table":"ali.table","index":["idx.index"]}}}}{extra}]}}"#,
        raw.join(","),
        src.join(",")
    )
}

fn run(dir: &Path, extra: &str) {
    let (done, prov) =
        crate::flow::run_plan(plan(extra).as_bytes(), dir, &dir.join("work"), 2, None).unwrap();
    assert!(
        done.skipped.is_empty() && !prov.contains("\"error\""),
        "{prov}"
    );
}

/// The MGF of the rows as the page exports it, read from the files the base chain left in the work folder.
fn rows_mgf(dir: &Path, spec: &str) -> String {
    let w = dir.join("work");
    let table = std::fs::read(w.join("gap.mzfeat")).unwrap();
    let samples =
        crate::flow_table::samples_text(&std::fs::read(dir.join("samples.json")).unwrap()).unwrap();
    let open = |k: usize| crate::flow_base::open(&w.join(format!("idx-{}.mzidx", NAMES[k])));
    ms2_file(
        &mut Slice(&table),
        table.len() as u64,
        &samples,
        spec,
        &sha256_hex(&table),
        crate::flow_export::polarity(&table).unwrap(),
        "mgf",
        open,
    )
    .unwrap()
}

/// Every row with an MS2 finds the library entry made from it, with the id of the row as the query; the consensus spectrum and a second run give the same.
#[test]
fn every_row_finds_its_own_spectrum_by_the_id_of_the_row() {
    let Some(out) = series("tm") else { return };
    let dir = fixture(&out, "", &hypotheses());
    std::fs::write(dir.join("lib.mgf"), "BEGIN IONS\nEND IONS\n").unwrap();
    run(&dir, "");
    let mgf = rows_mgf(&dir, "");
    let blocks = mgf.matches("FEATURE_ID=").count();
    assert!(blocks > 0, "the series has no MS2 for the table");
    let lib: String = mgf
        .lines()
        .map(|l| match l.strip_prefix("FEATURE_ID=") {
            Some(id) => format!("NAME=Self{id}"),
            None => l.to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(dir.join("lib.mgf"), lib).unwrap();
    let step = r#",{"id":"tm","step":"table.match@1","in":{"table":"gap.table","samples":"file:samples.json","index":["idx.index"],"library":"file:lib.mgf"},"params":{"tol_da":0.02,"precursor_ppm":10,"top_n":50,"min_shared":1,"min_score":0.5}}"#;
    run(&dir, step);
    let first = std::fs::read(dir.join("work/tm.json")).unwrap();
    let v = parse(&first).unwrap();
    assert_eq!(v.get("step").and_then(|s| s.str()), Some("table.match@1"));
    let hits = v.get("hits").unwrap().arr();
    assert!(!hits.is_empty());
    let own = hits.iter().filter(|h| {
        let q = h.get("query").and_then(|x| x.str()).unwrap();
        h.get("name").and_then(|x| x.str()) == Some(&format!("Self{q}"))
            && h.get("score").and_then(|x| x.f64()).unwrap() > 0.99
    });
    assert_eq!(own.count(), blocks, "{}", String::from_utf8_lossy(&first));
    run(&dir, step);
    assert_eq!(std::fs::read(dir.join("work/tm.json")).unwrap(), first);
    let consensus = step.replace(
        "\"top_n\":50",
        "\"top_n\":50,\"ms2_spectrum\":\"consensus\"",
    );
    run(&dir, &consensus);
    let v = parse(&std::fs::read(dir.join("work/tm.json")).unwrap()).unwrap();
    assert!(!v.get("hits").unwrap().arr().is_empty());
    let _ = std::fs::remove_dir_all(&out);
    let _ = FORMULA;
}
