//! `quant.curve@1` through `flow run` on the synthetic series of `tools/synthetic_data_calibration.py` (committed in `tests/data/calibration`): the
//! two outputs equal the golden files (`MZLAB_UPDATE_GOLDEN=1` rewrites them), a second run gives the same bytes, the plan check takes the step.

use std::path::{Path, PathBuf};

fn data() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/calibration")
}

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_cli_curve_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("work")).unwrap();
    for f in ["responses.csv", "standards.csv"] {
        std::fs::copy(data().join(f), d.join(f)).unwrap();
    }
    d
}

/// The plan of the golden files: the parameters of the browser test (`tests/test_flowcal_curve_wasm.py`) are the same.
pub(crate) const PARAMS: &str = r#"{"istd":"IS","exclude":"B|std8|saturated detector"}"#;

fn run(d: &Path, params: &str) -> Result<(String, String, String), String> {
    let plan = format!(
        r#"{{"format":"mzflow/2","inputs":[{{"name":"responses.csv","type":"report"}},{{"name":"standards.csv","type":"report"}}],"steps":[
        {{"id":"c","step":"quant.curve@1","in":{{"responses":"file:responses.csv","standards":"file:standards.csv"}},"params":{params}}}]}}"#
    );
    let (run, prov) = crate::flow::run_plan(plan.as_bytes(), d, &d.join("work"), 1, None)
        .map_err(|e| e.to_string())?;
    if !run.skipped.is_empty() || prov.contains("\"error\"") {
        return Err(prov);
    }
    let read = |f: &str| std::fs::read_to_string(d.join("work").join(f)).unwrap();
    Ok((read("c.json"), read("c.csv"), prov))
}

#[test]
fn the_synthetic_series_gives_the_golden_files() {
    let d = dir("golden");
    let (json, csv, prov) = run(&d, PARAMS).unwrap();
    let (gj, gc) = (data().join("golden.json"), data().join("golden.csv"));
    if std::env::var_os("MZLAB_UPDATE_GOLDEN").is_some() {
        std::fs::write(&gj, &json).unwrap();
        std::fs::write(&gc, &csv).unwrap();
    }
    assert_eq!(
        json,
        std::fs::read_to_string(&gj).unwrap(),
        "MZLAB_UPDATE_GOLDEN=1 rewrites it"
    );
    assert_eq!(csv, std::fs::read_to_string(&gc).unwrap());
    // the texts are those of the shared function
    let p = match mzlab_core::json::parse(PARAMS.as_bytes()).unwrap() {
        mzlab_core::json::Value::Obj(o) => o,
        _ => unreachable!(),
    };
    let (j2, c2, w) = mzlab_ops::run::table::curve::curve_text(
        &p,
        &std::fs::read(data().join("responses.csv")).unwrap(),
        &std::fs::read(data().join("standards.csv")).unwrap(),
    )
    .unwrap();
    assert_eq!((j2, c2), (json.clone(), csv.clone()));
    for k in &w {
        assert!(prov.contains(k.as_str()), "{k}: {prov}");
    }
    for k in ["warn.flow.curve.range", "warn.flow.curve.excluded"] {
        assert!(w.iter().any(|x| x == k), "{k}: {w:?}");
    }
    // u1 is below and u4 above the calibrated range, the QC is back within 15 %
    assert!(csv
        .lines()
        .any(|l| l.starts_with("A,u1,") && l.contains(",below_range,below_loq,")));
    assert!(csv
        .lines()
        .any(|l| l.starts_with("A,u4,") && l.contains(",above_range,detected,")));
    let qc: f64 = csv
        .lines()
        .find(|l| l.starts_with("A,qc1,"))
        .and_then(|l| l.split(',').nth(9))
        .and_then(|v| v.parse().ok())
        .unwrap();
    assert!((qc - 100.0).abs() < 15.0, "{qc}");
    let (again, _, _) = run(&d, PARAMS).unwrap();
    assert_eq!(again, json);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn bad_parameters_fail_the_step_and_the_defaults_run() {
    let d = dir("params");
    assert!(run(&d, r#"{"weight":"1/x3"}"#).is_err());
    assert!(run(&d, r#"{"istd":"nothing"}"#).is_err());
    let (json, _, _) = run(&d, "{}").unwrap();
    assert!(json.starts_with(
        r#"{"format":"calibration/1","signal":"area","model":"linear","weight":"1/x","zero":false"#
    ));
    let (json, _, _) = run(&d, r#"{"model":"quadratic","zero":true,"signal":"height"}"#).unwrap();
    assert!(json.contains(r#""model":"quadratic""#) && json.contains(r#""zero":true"#));
    let _ = std::fs::remove_dir_all(&d);
}
