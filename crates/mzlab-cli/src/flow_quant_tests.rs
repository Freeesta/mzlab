//! `quant.calibrate@1` on an invented table (no lab data): the curves are applied as written, flags and warnings follow the range, a second run gives the same bytes, a curve file that cannot be read fails.

use std::path::Path;

use mzlab_feat::recipe::Recipe;
use mzlab_feat::samples::{self, propose};
use mzlab_feat::store::{write_table, Origin};
use mzlab_feat::table::{Aligned, CELL_ABSENT, CELL_DETECTED};

const NAMES: [&str; 4] = ["blank.mzML", "a1.mzML", "a2.mzML", "a3.mzML"];

fn files() -> Vec<(String, String)> {
    NAMES
        .iter()
        .map(|n| (n.to_string(), "00".repeat(32)))
        .collect()
}

/// Row 0 (250.1, 6.0) and row 1 (300.2, 7.0); the second one is not detected in `a1`.
fn table() -> Vec<u8> {
    let rows: [(f64, f64, [f64; 4]); 2] = [
        (250.1, 6.0, [0.0, 1000.0, 5000.0, 100000.0]),
        (300.2, 7.0, [0.0, 0.0, 6000.0, 8000.0]),
    ];
    let mut t = Aligned::with_capacity(4, rows.len());
    for (mz, rt, areas) in rows {
        t.mz.push(mz);
        t.rt.push(rt);
        for (i, a) in areas.into_iter().enumerate() {
            t.area.push(a);
            t.height.push((a / 10.0) as f32);
            t.apex_rt.push(rt as f32);
            t.width.push(0.1);
            t.state.push(if a > 0.0 || i == 0 && mz < 260.0 {
                CELL_DETECTED
            } else {
                CELL_ABSENT
            });
        }
    }
    let origin = Origin {
        inputs: files(),
        ..Origin::default()
    };
    write_table(&t, &Recipe::default(), &origin)
}

const CURVES: &str = "name,mz,rt,slope,intercept,unit,range_min,range_max,dilution\n\
    Alpha,250.0,6.1,1000,0,mg/L,0.5,20,2\n\
    \"Beta, test\",300.3,7.0,500,2000,mg/L,,,\n\
    Gamma,999,1,1,0,mg/L,,,\n";

fn dir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_cli_quant_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("work")).unwrap();
    std::fs::write(d.join("table.mzfeat"), table()).unwrap();
    std::fs::write(d.join("curves.csv"), CURVES).unwrap();
    std::fs::write(d.join("samples.json"), samples::to_json(&propose(&files()))).unwrap();
    d
}

fn run(d: &Path, params: &str) -> Result<(String, String), String> {
    let plan = format!(
        r#"{{"format":"mzflow/2","inputs":[{{"name":"table.mzfeat","type":"table"}},{{"name":"curves.csv","type":"report"}},{{"name":"samples.json","type":"samples"}}],"steps":[
        {{"id":"q","step":"quant.calibrate@1","in":{{"table":"file:table.mzfeat","curves":"file:curves.csv","samples":"file:samples.json"}},"params":{params}}}]}}"#
    );
    let (run, prov) = crate::flow::run_plan(plan.as_bytes(), d, &d.join("work"), 1, None)
        .map_err(|e| e.to_string())?;
    if !run.skipped.is_empty() || prov.contains("\"error\"") {
        return Err(prov);
    }
    Ok((std::fs::read_to_string(d.join("work/q.csv")).unwrap(), prov))
}

#[test]
fn curves_are_applied_and_flagged() {
    let d = dir("apply");
    let (csv, prov) = run(&d, "{}").unwrap();
    let want = "compound,mz,rt,sample,signal,concentration,unit,flag\n\
        Alpha,250.1,6,a1.mzML,1000,2.000000e0,mg/L,ok\n\
        Alpha,250.1,6,a2.mzML,5000,1.000000e1,mg/L,ok\n\
        Alpha,250.1,6,a3.mzML,100000,2.000000e2,mg/L,above_range\n\
        \"Beta, test\",300.2,7,a1.mzML,,,mg/L,not_detected\n\
        \"Beta, test\",300.2,7,a2.mzML,6000,8.000000e0,mg/L,ok\n\
        \"Beta, test\",300.2,7,a3.mzML,8000,1.200000e1,mg/L,ok\n";
    assert_eq!(csv, want);
    for key in ["warn.flow.quant.noRow", "warn.flow.quant.range"] {
        assert!(prov.contains(key), "{key} missing: {prov}");
    }
    assert!(!prov.contains("warn.flow.quant.negative"));
    // the same inputs give the same bytes
    let (again, _) = run(&d, "{}").unwrap();
    assert_eq!(csv, again);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn parameters_change_the_matching_and_the_signal() {
    let d = dir("params");
    // a tolerance too small for the curve at m/z 250.0 leaves Alpha without a row
    let (csv, _) = run(&d, r#"{"mz_tol":0.05}"#).unwrap();
    assert!(!csv.contains("Alpha"), "{csv}");
    // height is a tenth of the area in the table; the intercept makes Beta negative
    let (csv, prov) = run(&d, r#"{"signal":"height"}"#).unwrap();
    assert!(
        csv.contains("a2.mzML,600,-2.800000e0,mg/L,negative"),
        "{csv}"
    );
    assert!(prov.contains("warn.flow.quant.negative"));
    // the blank is not a sample unless asked for
    let (csv, _) = run(&d, r#"{"kinds":"blank"}"#).unwrap();
    assert!(
        csv.contains("blank.mzML") && !csv.contains("a1.mzML"),
        "{csv}"
    );
    assert!(run(&d, r#"{"kinds":"nothing"}"#).is_err());
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_curve_file_that_cannot_be_read_fails() {
    let d = dir("bad");
    let head = "name;mz;rt;slope;intercept\n";
    for (text, why) in [
        (format!("{head}A;250,1;6,0;0;0\n"), "zero slope"),
        (
            format!("{head}A;250,1;6,0;1;0\nA;300,1;7,0;1;0\n"),
            "duplicate name",
        ),
        (
            "name,mz,rt,slope\nA,1,1,1\n".to_string(),
            "no intercept column",
        ),
        (format!("{head}A;x;6;1;0\n"), "text for a number"),
        (head.to_string(), "no curve"),
    ] {
        std::fs::write(d.join("curves.csv"), text).unwrap();
        assert!(run(&d, "{}").is_err(), "{why}");
    }
    // a spreadsheet with ; and a decimal comma is read
    std::fs::write(d.join("curves.csv"), format!("{head}A;250,1;6,0;1000;0\n")).unwrap();
    assert!(run(&d, "{}").unwrap().0.contains("a2.mzML,5000,5.000000e0"));
    let _ = std::fs::remove_dir_all(&d);
}
