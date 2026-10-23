//! `table.plausible@1` in a plan: a `.mzfeat` written by the real store on invented rows gives the same text as the engine called by hand, every limit
//! of the step reaches the engine, and a second run writes the same bytes.

use std::path::Path;

use mzlab_feat::plausible::{to_json, Limits};
use mzlab_feat::recipe::Recipe;
use mzlab_feat::store::{write_table, Origin};
use mzlab_feat::table::{Aligned, CELL_DETECTED};

/// (m/z, RT): two sound rows, one at m/z 3 and one at 900 min.
const POINTS: [(f64, f64); 4] = [(200.1, 5.0), (350.2, 6.0), (3.0, 7.0), (410.4, 900.0)];

fn table() -> Aligned {
    let mut t = Aligned::with_capacity(1, POINTS.len());
    for (mz, rt) in POINTS {
        t.mz.push(mz);
        t.rt.push(rt);
        t.area.push(1.0e6);
        t.height.push(1.0e5);
        t.apex_rt.push(rt as f32);
        t.width.push(0.1);
        t.state.push(CELL_DETECTED);
    }
    t
}

fn dir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_cli_plausible_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("work")).unwrap();
    let origin = Origin {
        inputs: vec![("a.mzML".to_string(), String::new())],
        ..Origin::default()
    };
    std::fs::write(
        d.join("table.mzfeat"),
        write_table(&table(), &Recipe::default(), &origin),
    )
    .unwrap();
    d
}

fn run(d: &Path, params: &str) -> String {
    let plan = format!(
        r#"{{"format":"mzflow/2","inputs":[{{"name":"table.mzfeat","type":"table"}}],"steps":[
        {{"id":"p","step":"table.plausible@1","in":{{"table":"file:table.mzfeat"}},"params":{params}}}]}}"#
    );
    let (run, prov) = crate::flow::run_plan(plan.as_bytes(), d, &d.join("work"), 1, None).unwrap();
    assert!(
        run.skipped.is_empty() && !prov.contains("\"error\""),
        "{prov}"
    );
    std::fs::read_to_string(d.join("work").join("p.json")).unwrap()
}

/// The count of a check in the text.
fn count(text: &str, id: &str) -> i64 {
    let j = mzlab_core::json::parse(text.as_bytes()).unwrap();
    let c = j
        .get("checks")
        .unwrap()
        .arr()
        .iter()
        .find(|c| c.get("id").and_then(|x| x.str()) == Some(id))
        .unwrap();
    c.get("count").and_then(|x| x.f64()).unwrap() as i64
}

#[test]
fn the_plan_gives_the_engine_text_and_the_same_bytes_twice() {
    let d = dir("default");
    let got = run(&d, "{}");
    assert_eq!(got, to_json(&table(), false, &Limits::default()));
    assert_eq!((count(&got, "mz_range"), count(&got, "rt_range")), (1, 1));
    assert_eq!(got, run(&d, "{}"));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn every_limit_reaches_the_engine() {
    let d = dir("limits");
    assert_eq!(count(&run(&d, r#"{"mz_min":1}"#), "mz_range"), 0);
    assert_eq!(count(&run(&d, r#"{"mz_max":300}"#), "mz_range"), 3);
    assert_eq!(count(&run(&d, r#"{"rt_max":6.5}"#), "rt_range"), 2);
    let width = run(&d, r#"{"width_max":0.05}"#);
    assert_eq!(count(&width, "peak_shape"), 4);
    let tol = run(&d, r#"{"mz_tol_da":0.3}"#);
    assert!(
        tol.contains("\"id\":\"ppm_error\",\"flag\":\"na\""),
        "{tol}"
    );
    let _ = std::fs::remove_dir_all(&d);
}
