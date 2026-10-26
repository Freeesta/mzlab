//! mzFlow against the Dig of mzFinder on the five demo full-scan files of the site (`mzlab/web/esempi/FullScan_t*.mzML`, unit resolution): the base
//! chain gives the table of the page byte for byte and `tp.dig_lr` the JSON of `lr::dig` over it. `tp.link@2` reads the same table through the gates of `tp/1` in Da.

use std::path::{Path, PathBuf};

use mzlab_core::idxread::Index;
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::upload::prepare;
use mzlab_feat::legacy::lr::{self, summary::summary};
use mzlab_feat_wasm::run;

const NAMES: [&str; 5] = [
    "FullScan_t0.mzML",
    "FullScan_t5.mzML",
    "FullScan_t10.mzML",
    "FullScan_t15.mzML",
    "FullScan_t30.mzML",
];
const FORMULA: &str = "C14H13F4N3O2S";

fn fixture(tag: &str) -> PathBuf {
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
    let samples = mzlab_core::campioni::to_json(&mzlab_core::campioni::proponi(&files));
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
        r#"{{"files":[{}],"campioni":{}}}"#,
        names.join(","),
        samples.trim()
    );
    let parent = format!(r#"{{"name":"parent","neutral":"{FORMULA}","adduct":"[M+H]+"}}"#);
    let rules = std::fs::read(rules_path("regole_tp_2.json")).unwrap();
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
