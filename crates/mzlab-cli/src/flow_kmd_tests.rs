//! `table.kmd@1` in a plan: a `.mzfeat` written by the real store on invented rows (a CF2 homologous series and two loose ions) gives the same text as
//! the engine called by hand (`kmd_json`), the parameters reach the engine, and a second run writes the same bytes.

use std::path::Path;

use mzlab_feat::recipe::Recipe;
use mzlab_feat::stat::kmd_json;
use mzlab_feat::store::{write_table, Origin};
use mzlab_feat::table::{Aligned, CELL_DETECTED};

const CF2: f64 = 49.996806;

/// (m/z, RT): five members of a CF2 series with growing RT, then two unrelated ions.
fn points() -> Vec<(f64, f64)> {
    let mut v: Vec<(f64, f64)> = (0..5)
        .map(|i| (263.0 + CF2 * i as f64, 4.0 + 0.8 * i as f64))
        .collect();
    v.push((301.1234, 3.1));
    v.push((412.3456, 9.2));
    v
}

fn table() -> Vec<u8> {
    let pts = points();
    let mut t = Aligned::with_capacity(1, pts.len());
    for (mz, rt) in &pts {
        t.mz.push(*mz);
        t.rt.push(*rt);
        t.area.push(1.0e6);
        t.height.push(1.0e5);
        t.apex_rt.push(*rt as f32);
        t.width.push(0.1);
        t.state.push(CELL_DETECTED);
    }
    let origin = Origin {
        inputs: vec![("a.mzML".to_string(), String::new())],
        ..Origin::default()
    };
    write_table(&t, &Recipe::default(), &origin)
}

fn dir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_cli_kmd_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("work")).unwrap();
    std::fs::write(d.join("table.mzfeat"), table()).unwrap();
    d
}

fn run(d: &Path, params: &str) -> String {
    let plan = format!(
        r#"{{"format":"mzflow/2","inputs":[{{"name":"table.mzfeat","type":"table"}}],"steps":[
        {{"id":"k","step":"table.kmd@1","in":{{"table":"file:table.mzfeat"}},"params":{params}}}]}}"#
    );
    let (run, prov) = crate::flow::run_plan(plan.as_bytes(), d, &d.join("work"), 1, None).unwrap();
    assert!(
        run.skipped.is_empty() && !prov.contains("\"error\""),
        "{prov}"
    );
    std::fs::read_to_string(d.join("work").join("k.json")).unwrap()
}

/// The engine called by hand on the same rows.
fn by_hand(extra: &str) -> String {
    let rows: Vec<String> = points()
        .iter()
        .map(|(mz, rt)| format!("{{\"mz\":{mz},\"rt\":{rt}}}"))
        .collect();
    let body = format!("{{\"features\":[{}]{extra}}}", rows.join(","));
    kmd_json(body.as_bytes()).unwrap()
}

#[test]
fn the_plan_gives_the_engine_text_and_the_same_bytes_twice() {
    let d = dir("default");
    let got = run(&d, "{}");
    assert_eq!(got, by_hand(""));
    assert!(got.contains("\"id\":\"CF2\""), "{got}");
    assert!(
        got.contains("\"series\":[{"),
        "a CF2 series is found: {got}"
    );
    assert_eq!(got, run(&d, "{}"));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn every_parameter_reaches_the_engine() {
    let d = dir("params");
    let unit = run(&d, r#"{"unit":"CH2"}"#);
    assert_eq!(unit, by_hand(r#","unit":"CH2""#));
    assert!(unit.contains("\"id\":\"CH2\""), "{unit}");
    let tol = run(&d, r#"{"tol_mda":0.5}"#);
    assert_eq!(tol, by_hand(r#","tolMDa":0.5"#));
    let members = run(&d, r#"{"min_members":6}"#);
    assert_eq!(members, by_hand(r#","minMembers":6"#));
    assert!(members.contains("\"series\":[]"), "{members}");
    let _ = std::fs::remove_dir_all(&d);
}
