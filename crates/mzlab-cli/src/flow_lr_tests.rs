//! mzFlow against the Dig of mzFinder on the five demo full-scan files of the site (`mzlab/web/esempi/FullScan_t*.mzML`, unit resolution): the base
//! chain gives the table of the page byte for byte and `tp.dig_lr` the JSON of `lr::dig` over it. `tp.link@2` reads the same table through the gates of `tp/2` in Da.

#[path = "flow_lr_tol_tests.rs"]
mod tol;

use std::path::{Path, PathBuf};

use mzlab_core::idxread::Index;
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::upload::prepare;
use mzlab_feat::legacy::lr::{self, summary::summary};
use mzlab_ops::run::feat::run;

const NAMES: [&str; 5] = [
    "FullScan_t0.mzML",
    "FullScan_t5.mzML",
    "FullScan_t10.mzML",
    "FullScan_t15.mzML",
    "FullScan_t30.mzML",
];
const FORMULA: &str = "C14H13F4N3O2S";

fn fixture(tag: &str) -> PathBuf {
    fixture_with(tag, None, &[])
}

/// `fixture` with the file `std` marked as the standard of the parent in `samples.json` (no time) and the `series` (condition) of some files.
fn fixture_with(tag: &str, std: Option<&str>, series: &[(&str, &str)]) -> PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_cli_lr_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("work")).unwrap();
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mzlab/web/esempi");
    for n in NAMES {
        std::fs::copy(src.join(n), d.join(n)).unwrap();
    }
    let files: Vec<_> = NAMES
        .iter()
        .map(|n| (n.to_string(), String::new(), String::new()))
        .collect();
    let mut cs = mzlab_core::samples::propose(&files);
    for c in cs.iter_mut().filter(|c| Some(c.file.as_str()) == std) {
        c.kind = "std".into();
        c.set_time(None, "min");
    }
    for c in cs.iter_mut() {
        if let Some((_, s)) = series.iter().find(|(f, _)| *f == c.file) {
            c.series = s.to_string();
        }
    }
    let samples = mzlab_core::samples::to_json(&cs);
    std::fs::write(d.join("samples.json"), samples).unwrap();
    std::fs::write(
        d.join("parent.json"),
        format!(r#"{{"formula":"{FORMULA}"}}"#),
    )
    .unwrap();
    d
}

fn rules_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../TP_Mine/data")
        .join(name)
}

fn plan(tail: &str) -> String {
    let raw: Vec<String> = NAMES
        .iter()
        .map(|n| format!(r#"{{"name":"{n}","type":"raw"}}"#))
        .collect();
    let src: Vec<String> = NAMES.iter().map(|n| format!(r#""file:{n}""#)).collect();
    format!(
        r#"{{"format":"mzflow/2","inputs":[{},{{"name":"samples.json","type":"samples"}},{{"name":"parent.json","type":"structure"}}],"steps":[
        {{"id":"idx","step":"index.mzidx@1","in":{{"source":[{}]}}}},
        {{"id":"det","step":"features.detect@1","in":{{"index":["idx.index"]}}}},
        {{"id":"ali","step":"features.align@1","in":{{"peaks":["det.peaks"]}}}},
        {{"id":"gap","step":"features.gapfill@1","in":{{"table":"ali.table","index":["idx.index"]}}}}{tail}]}}"#,
        raw.join(","),
        src.join(",")
    )
}

fn run_flow(d: &Path, tail: &str) -> String {
    let (run, prov) =
        crate::flow::run_plan(plan(tail).as_bytes(), d, &d.join("work"), 2, None).unwrap();
    assert!(run.skipped.is_empty(), "{:?}", run.skipped);
    prov
}

/// The chain of the page (`feat-worker.js`): indexes in memory, each file detected with its own recipe, aligned with the first one.
fn page(d: &Path) -> (Vec<u8>, String) {
    let idxs: Vec<Vec<u8>> = NAMES
        .iter()
        .map(|n| {
            let b = std::fs::read(d.join(n)).unwrap();
            let mut src = b.clone();
            let p = prepare(&mut src, b.len() as u64, n).unwrap();
            build_mzml_to_vec(&p.table, &b, 64 << 20).unwrap().0
        })
        .collect();
    let open = |k: usize| Index::open(idxs[k].clone(), idxs[k].len() as u64).unwrap();
    let r = run::recipe_for("", &mut open(0)).unwrap();
    let det: Vec<Vec<u8>> = NAMES
        .iter()
        .enumerate()
        .map(|(k, n)| {
            run::detect(&mut open(k), n, &run::recipe_for("", &mut open(k)).unwrap()).unwrap()
        })
        .collect();
    let (f, origin) = run::read_files(&det).unwrap();
    let table = run::align(&f, &origin, &r, |k| Ok(open(k))).unwrap();
    let samples = std::fs::read_to_string(d.join("samples.json")).unwrap();
    let names: Vec<String> = NAMES
        .iter()
        .map(|n| format!(r#"{{"name":"{n}","kind":"full"}}"#))
        .collect();
    let files = format!(
        r#"{{"files":[{}],"samples":{}}}"#,
        names.join(","),
        samples.trim()
    );
    let parent = format!(r#"{{"name":"parent","neutral":"{FORMULA}","adduct":"[M+H]+"}}"#);
    let rules = std::fs::read(rules_path("regole_tp_3.json")).unwrap();
    let mut ix: Vec<Index<Vec<u8>>> = (0..NAMES.len()).map(open).collect();
    let refs: Vec<_> = f.iter().collect();
    let h = run::hypotheses(&mut ix, &refs, &rules, "", FORMULA, None, &r).unwrap();
    let dg = lr::dig(
        &files,
        &parent,
        "{}",
        &table,
        &h.registry.to_json(),
        &mut |k| Ok(open(k)),
    )
    .unwrap();
    (table, summary(&dg, "{}") + "\n")
}

#[test]
fn the_base_chain_and_the_lr_dig_give_what_the_page_gives_on_the_demo_files() {
    let d = fixture("dig");
    let tail = r#",{"id":"dig","step":"tp.dig_lr@1","in":{"table":"gap.table","samples":"file:samples.json","parent":"file:parent.json","index":["idx.index"]}}"#;
    run_flow(&d, tail);
    let (table, dig) = page(&d);
    let gap = std::fs::read(d.join("work/gap.mzfeat")).unwrap();
    assert!(
        gap == table,
        "table: flow {} bytes, page {}",
        gap.len(),
        table.len()
    );
    let got = std::fs::read_to_string(d.join("work/dig.json")).unwrap();
    assert!(
        got == dig,
        "dig JSON differs: flow {} bytes, page {}",
        got.len(),
        dig.len()
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// A plan with a standard: the Flow table aligns all five files, the Dig reads the standard as a control column (it used to stop on "the series has
/// 5 files, the Dig 4") and gives what the page gives on the same files and roles.
#[test]
fn a_plan_with_a_standard_digs_as_the_page_does() {
    let d = fixture_with("dig_std", Some("FullScan_t30.mzML"), &[]);
    let tail = r#",{"id":"dig","step":"tp.dig_lr@1","in":{"table":"gap.table","samples":"file:samples.json","parent":"file:parent.json","index":["idx.index"]}}"#;
    run_flow(&d, tail);
    let (_, dig) = page(&d);
    let got = std::fs::read_to_string(d.join("work/dig.json")).unwrap();
    assert!(
        got == dig,
        "dig JSON differs: flow {} bytes, page {}",
        got.len(),
        dig.len()
    );
    let v: serde_json::Value = serde_json::from_str(&got).unwrap();
    let roles: Vec<&str> = v["rows"][0]["points"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["role"].as_str().unwrap())
        .collect();
    assert_eq!(roles, ["t0", "sample", "sample", "sample", "std"]);
    assert!(v["rows"][0]["controls"]["standard"].is_number());
    let _ = std::fs::remove_dir_all(&d);
}

/// Two series in one plan: one alignment of every file, one `tp.dig_lr` that judges each condition on its own columns (the shared t0 for both)
/// and writes the block `groups` and the evidence per condition, as the page does on the same table.
#[test]
fn two_series_in_one_plan_are_dug_on_one_table_as_the_page_does() {
    let series = [
        ("FullScan_t5.mzML", "A"),
        ("FullScan_t15.mzML", "A"),
        ("FullScan_t10.mzML", "B"),
        ("FullScan_t30.mzML", "B"),
    ];
    let d = fixture_with("dig_two", None, &series);
    let tail = r#",{"id":"dig","step":"tp.dig_lr@1","in":{"table":"gap.table","samples":"file:samples.json","parent":"file:parent.json","index":["idx.index"]}}"#;
    run_flow(&d, tail);
    let (_, dig) = page(&d);
    let got = std::fs::read_to_string(d.join("work/dig.json")).unwrap();
    assert!(
        got == dig,
        "dig JSON differs: flow {} bytes, page {}",
        got.len(),
        dig.len()
    );
    let v: serde_json::Value = serde_json::from_str(&got).unwrap();
    let ids: Vec<&str> = v["groups"]["conditions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["A", "B"]);
    let parent = &v["rows"][0];
    let conds: Vec<&str> = parent["points"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["cond"].as_str().unwrap())
        .collect();
    assert_eq!(conds, ["", "A", "B", "A", "B"]);
    let by: Vec<(&str, u64)> = parent["by_cond"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| (c["cond"].as_str().unwrap(), c["k_points"].as_u64().unwrap()))
        .collect();
    // each condition: its two times and the shared t0
    assert_eq!(by, [("A", 3), ("B", 3)]);
    assert_eq!(parent["k_points"], 5);
    let _ = std::fs::remove_dir_all(&d);
}

/// `tp.link@2` reads the unit-resolution table: the gates look for the parent within the Dig's offset and compare fragments in Da, so the view of the
/// products has the parent and accepted products, among them the m/z the Dig proposes.
#[test]
fn tp_link_reads_a_unit_resolution_table() {
    let d = fixture("link");
    let tail = r#",{"id":"gen","step":"tp.generate@1","in":{"parent":"file:parent.json"}},
        {"id":"lnk","step":"tp.link@2","in":{"table":"gap.table","samples":"file:samples.json","parent":"file:parent.json","index":["idx.index"],"hypotheses":"gen.hypotheses"}}"#;
    let (_, prov) =
        crate::flow::run_plan(plan(tail).as_bytes(), &d, &d.join("work"), 2, None).unwrap();
    assert!(!prov.contains("\"error\""), "{prov}");
    let bytes = std::fs::read(d.join("work/lnk.mzfeat")).unwrap();
    let (t, _) = mzlab_feat::store::read_table_from(
        &mut mzlab_feat::store::Slice(&bytes),
        bytes.len() as u64,
    )
    .unwrap();
    assert_eq!(
        t.tp.status
            .iter()
            .filter(|&&s| s == mzlab_feat::tp::ST_PARENT)
            .count(),
        1
    );
    assert!(t.tp.status.contains(&mzlab_feat::tp::ST_TP));
    let parent = t.tp.parent_row as usize;
    assert!((t.mz[parent] - 364.074).abs() < 0.6, "{}", t.mz[parent]);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_parent_file_gives_the_adduct_that_sets_the_polarity_of_the_dig() {
    use mzlab_feat::parent::parent_of;
    let p = parent_of(br#"{"neutral": "C10H12N2O3S", "adduct": "[M-H]-"}"#).unwrap();
    assert_eq!(p.adduct, "[M-H]-");
    assert_eq!(parent_of(b"C10H12N2O3S").unwrap().adduct, "[M+H]+");
    assert_eq!(
        parent_of(br#"{"neutral": "C10H12N2O3S", "adduct": "nonsense"}"#)
            .unwrap()
            .adduct,
        "[M+H]+"
    );
}
