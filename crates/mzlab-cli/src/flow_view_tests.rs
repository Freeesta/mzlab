//! A `table.filter` view wired into a step that wants a table: the plan runs, and the step writes the bytes it writes on the table the view stands for
//! (`table_of_view` of the `table` `.wasm`, called here as the page calls it) given as a file. Invented areas of `flow_stats_tests` (a blank, three
//! references, three treated), a filter that drops rows and normalises; also a view that keeps no row and a view of a view.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use mzlab_core::campioni;
use mzlab_core::disegno;
use mzlab_core::step::DataType;
use mzlab_feat::store::read_table;
use mzlab_feat_table_wasm::core::table_of_view;

use crate::flow_stats_tests::{campioni_2, design, table};

/// The steps that read a table and need no file this test does not make.
const CONSUMERS: &[&str] = &[
    "features.group",
    "table.kmd",
    "table.pca",
    "table.stats",
    "table.plausible",
    "table.drift",
    "table.filter",
];

const FILTER: &str = r#"{"min_detected":2,"blank_ratio":3,"normalize":"sum"}"#;
const NONE_KEPT: &str = r#"{"min_area":1e12}"#;

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_cli_view_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("work")).unwrap();
    std::fs::write(d.join("table.mzfeat"), table()).unwrap();
    // the plan names it; the tests write the table of the view over it
    std::fs::write(d.join("mat.mzfeat"), table()).unwrap();
    std::fs::write(d.join("design.json"), disegno::to_json(&design())).unwrap();
    std::fs::write(d.join("samples.json"), campioni::to_json(&campioni_2())).unwrap();
    d
}

/// The `in` of the step `id` reading its table from `table` and the other ports from the files of `dir`.
fn ports(id: &str, table: &str) -> String {
    let reg = crate::flow::steps();
    let s = mzlab_core::step::find(&reg, id).unwrap();
    let wire = |ty: DataType| match ty {
        DataType::Table => table.to_string(),
        DataType::Samples => "file:samples.json".into(),
        DataType::Design => "file:design.json".into(),
        other => panic!("{id}: no file of type {other:?}"),
    };
    let ins: Vec<String> = s
        .ins
        .iter()
        .filter(|p| !p.optional || p.ty == DataType::Table)
        .map(|p| format!(r#""{}":"{}""#, p.name, wire(p.ty)))
        .collect();
    format!("{{{}}}", ins.join(","))
}

const INPUTS: &str = r#"{"name":"table.mzfeat","type":"table"},{"name":"mat.mzfeat","type":"table"},{"name":"samples.json","type":"samples"},{"name":"design.json","type":"design"}"#;

/// Runs the plan; for each instance, the bytes of its output or the key of its error.
fn run_all(d: &Path, steps: &str) -> Result<BTreeMap<String, Result<Vec<u8>, String>>, String> {
    let plan = format!(r#"{{"format":"mzflow/2","inputs":[{INPUTS}],"steps":[{steps}]}}"#);
    let work = d.join("work");
    let (_, prov) = crate::flow::run_plan(plan.as_bytes(), d, &work, 2, None)
        .map_err(|e| format!("{} {:?}", e.key, e.params))?;
    let prov = mzlab_core::json::parse(prov.as_bytes()).unwrap();
    let mut out = BTreeMap::new();
    for (id, act) in prov.get("activity").unwrap().entries() {
        let got = match act.get("error") {
            Some(e) => Err(e
                .get("error_key")
                .and_then(|k| k.str())
                .unwrap_or("")
                .to_string()),
            None => {
                let file = std::fs::read_dir(&work)
                    .unwrap()
                    .flatten()
                    .map(|e| e.path())
                    .find(|p| p.file_stem().is_some_and(|s| s.to_string_lossy() == *id))
                    .unwrap();
                Ok(std::fs::read(file).unwrap())
            }
        };
        out.insert(id.clone(), got);
    }
    Ok(out)
}

/// The output of the instance `c` of a plan.
fn run(d: &Path, steps: &str) -> Result<Vec<u8>, String> {
    run_all(d, steps)?.remove("c").unwrap()
}

/// The view of the plan's filter and the table it stands for, made with the function the page calls.
fn page_table(d: &Path, params: &str) -> (String, Vec<u8>) {
    run(
        d,
        &format!(
            "{},{}",
            filter_step(params),
            consumer("table.kmd", "f.table")
        ),
    )
    .unwrap();
    let view = std::fs::read_to_string(d.join("work/f.json")).unwrap();
    let t = std::fs::read(d.join("table.mzfeat")).unwrap();
    let mat = table_of_view(&t, &view).unwrap();
    (view, mat)
}

fn filter_step(params: &str) -> String {
    format!(
        r#"{{"id":"f","step":"table.filter@{}","in":{{"table":"file:table.mzfeat","samples":"file:samples.json"}},"params":{params}}}"#,
        version("table.filter")
    )
}

fn version(id: &str) -> u32 {
    crate::flow::steps()
        .iter()
        .find(|s| s.id == id)
        .map(|s| s.version)
        .unwrap()
}

fn consumer(id: &str, table: &str) -> String {
    consumer_as("c", id, table)
}

fn consumer_as(name: &str, id: &str, table: &str) -> String {
    format!(
        r#"{{"id":"{name}","step":"{id}@{}","in":{}}}"#,
        version(id),
        ports(id, table)
    )
}

#[test]
fn a_view_runs_every_table_step_as_its_table_does() {
    for (tag, params) in [("some", FILTER), ("none", NONE_KEPT)] {
        let d = dir(tag);
        let (_, mat) = page_table(&d, params);
        std::fs::write(d.join("mat.mzfeat"), &mat).unwrap();
        let kept = read_table(&mat).unwrap().0.len();
        // the filter drops some rows (or, with no row above its area, all of them)
        let all = read_table(&table()).unwrap().0.len();
        assert!(
            if tag == "none" {
                kept == 0
            } else {
                kept > 0 && kept < all
            },
            "{tag}: {kept}"
        );
        let name = |id: &str| id.replace('.', "_");
        let all = |table: &str| -> Vec<String> {
            CONSUMERS
                .iter()
                .map(|id| consumer_as(&name(id), id, table))
                .collect()
        };
        let via_view = run_all(
            &d,
            &format!("{},{}", filter_step(params), all("f.table").join(",")),
        )
        .unwrap();
        let via_file = run_all(&d, &all("file:mat.mzfeat").join(",")).unwrap();
        for id in CONSUMERS {
            let (v, f) = (&via_view[&name(id)], &via_file[&name(id)]);
            assert_eq!(v, f, "{id} on {tag}");
            // only the drift (no quality control in these files) stops; with no row left, a step may refuse with its own key
            assert!(
                v.is_ok() || *id == "table.drift" || tag == "none",
                "{id} on {tag}: {v:?}"
            );
        }
        let _ = std::fs::remove_dir_all(&d);
    }
}

#[test]
fn the_table_of_a_view_keeps_the_rows_and_the_areas_of_the_view() {
    let d = dir("cells");
    let (view, mat) = page_table(&d, FILTER);
    let (t, meta) = read_table(&mat).unwrap();
    let (orig, orig_meta) = read_table(&std::fs::read(d.join("table.mzfeat")).unwrap()).unwrap();
    let v = mzlab_core::json::parse(view.as_bytes()).unwrap();
    let ids: Vec<usize> = v
        .get("righe")
        .unwrap()
        .arr()
        .iter()
        .map(|x| x.f64().unwrap() as usize)
        .collect();
    let factors: Vec<f64> = v
        .get("fattori")
        .unwrap()
        .arr()
        .iter()
        .map(|x| x.f64().unwrap())
        .collect();
    assert_eq!(t.len(), ids.len());
    for (i, id) in ids.iter().enumerate() {
        let r = id - 1;
        assert_eq!(t.mz[i], orig.mz[r]);
        assert_eq!(factors.len(), t.n_files);
        for (k, f) in factors.iter().enumerate() {
            let (a, b) = (t.area[t.cell(i, k)], orig.area[orig.cell(r, k)]);
            if b > 0.0 {
                assert!((a - b * f).abs() <= 1e-9 * a.abs(), "{a} {b}");
            }
            assert_eq!(t.state[t.cell(i, k)], orig.state[orig.cell(r, k)]);
        }
    }
    // the recipe and the files of the table stay, so the next step finds them
    assert_eq!(
        meta.get("recipe").unwrap().str(),
        orig_meta.get("recipe").unwrap().str()
    );
    assert_eq!(
        mzlab_feat::store::input_names(&meta),
        mzlab_feat::store::input_names(&orig_meta)
    );
    // another table, or a view without its samples, is refused
    let other = table_of_view(&mat, &view).unwrap_err();
    assert_eq!(other.key, "err.flow.export.view");
    let cut = view.replace("\"campioni\":", "\"x\":");
    let t0 = std::fs::read(d.join("table.mzfeat")).unwrap();
    assert_eq!(
        table_of_view(&t0, &cut).unwrap_err().key,
        "err.flow.export.view"
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_view_of_a_view_and_an_export_after_it_run() {
    let d = dir("chain");
    let steps = format!(
        r#"{},{{"id":"g","step":"table.filter@{}","in":{{"table":"f.table","samples":"file:samples.json"}},"params":{{"min_detected":3}}}},{{"id":"c","step":"export.table_csv@1","in":{{"table":"g.table","samples":"file:samples.json"}}}}"#,
        filter_step(FILTER),
        version("table.filter")
    );
    let csv = String::from_utf8(run(&d, &steps).unwrap()).unwrap();
    assert!(csv.lines().count() >= 2, "{csv}");
    let _ = std::fs::remove_dir_all(&d);
}
