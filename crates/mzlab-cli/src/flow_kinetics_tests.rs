//! `quant.kinetics@1` through the plan runner on a synthetic series with known rates: 13 indexes (a shared t0 and two conditions of six times, invented
//! injection factors), `quant.targets` for the areas of the parent in a fixed window, the UV peaks of the reference as a file; the rates come back
//! within a few percent, the bytes are those of the shared text function, and the step runs without the UV input.

use std::path::{Path, PathBuf};

use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::upload::prepare;

#[path = "../../mzlab-feat/tests/common_quant/mod.rs"]
#[allow(dead_code)]
mod common_quant;
use common_quant::{mzml, Ion};

const TIMES: [i64; 6] = [5, 10, 20, 30, 45, 60];
const INJ: [f64; 13] = [
    1.0, 0.93, 1.08, 0.97, 1.06, 0.91, 1.04, 1.09, 0.95, 1.02, 0.92, 1.07, 0.98,
];
const COND: [(&str, f64); 2] = [("Fe", 0.04), ("Cu", 0.015)];

/// `(file stem, time, k)` of the 13 files in order of injection.
fn series() -> Vec<(String, f64, f64)> {
    let mut v = vec![("t0".to_string(), 0.0, 0.04)];
    for (c, k) in COND {
        v.extend(TIMES.iter().map(|t| (format!("{c}_t{t}"), *t as f64, k)));
    }
    v
}

fn index(ions: &[Ion], seed: u64) -> Vec<u8> {
    let x = mzml(ions, seed);
    let mut src = x.clone();
    let p = prepare(&mut src, x.len() as u64, "x.mzML").unwrap();
    build_mzml_to_vec(&p.table, &x, 64 << 20).unwrap().0
}

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_cli_kinetics_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("work")).unwrap();
    let mut uv = String::from(
        "file,kind,rt_apex_min,rt_lo_min,rt_hi_min,fwhm_s,height_mau,area_mau_s,snr\n",
    );
    for (i, (n, t, k)) in series().iter().enumerate() {
        let h = 100000.0 * (-k * t).exp() * INJ[i];
        std::fs::write(
            d.join(format!("{n}.mzidx")),
            index(&[Ion(200.0, 5.0, h, 0.1)], 1 + i as u64),
        )
        .unwrap();
        uv += &format!("{n}.mzML,narrow,4.3,4.2,4.4,6,10,{},90\n", 500.0 * INJ[i]);
    }
    std::fs::write(d.join("uv.csv"), uv).unwrap();
    std::fs::write(
        d.join("targets.csv"),
        "id,mz,rt_min,rt_max\nparent,200.0,4.0,6.0\n",
    )
    .unwrap();
    let files: Vec<(String, String, String)> = series()
        .iter()
        .map(|s| (format!("{}.mzML", s.0), String::new(), String::new()))
        .collect();
    std::fs::write(
        d.join("samples.json"),
        mzlab_core::samples::to_json(&mzlab_core::samples::propose(&files)),
    )
    .unwrap();
    d
}

fn plan(with_uv: bool, params: &str) -> String {
    let raws: Vec<String> = series()
        .iter()
        .map(|s| format!("{{\"name\":\"{}.mzidx\",\"type\":\"mzidx\"}}", s.0))
        .collect();
    let refs: Vec<String> = series()
        .iter()
        .map(|s| format!("\"file:{}.mzidx\"", s.0))
        .collect();
    let uv = if with_uv {
        r#","uv":"file:uv.csv""#
    } else {
        ""
    };
    format!(
        r#"{{"format":"mzflow/2","inputs":[{},{{"name":"samples.json","type":"samples"}},{{"name":"uv.csv","type":"peaks"}},{{"name":"targets.csv","type":"report"}}],"steps":[
        {{"id":"q","step":"quant.targets@1","in":{{"raw":[{}],"targets":"file:targets.csv"}}}},
        {{"id":"k","step":"quant.kinetics@1","in":{{"areas":"q.table","samples":"file:samples.json"{uv}}},"params":{params}}}]}}"#,
        raws.join(","),
        refs.join(",")
    )
}

fn run(d: &Path, with_uv: bool, params: &str) -> String {
    let (run, prov) = crate::flow::run_plan_cached(
        plan(with_uv, params).as_bytes(),
        d,
        &d.join("work"),
        1,
        None,
        Some(false),
    )
    .unwrap();
    assert!(
        run.skipped.is_empty() && !prov.contains("\"error\""),
        "{prov}"
    );
    std::fs::read_to_string(d.join("work/k.json")).unwrap()
}

fn k_of(doc: &str, series: &str) -> f64 {
    let v = mzlab_core::json::parse(doc.as_bytes()).unwrap();
    let f = v
        .get("fits")
        .unwrap()
        .arr()
        .iter()
        .find(|f| f.get("series").unwrap().str() == Some(series))
        .unwrap();
    f.get("k").unwrap().f64().unwrap()
}

#[test]
fn a_synthetic_series_gives_its_rates_back_and_the_bytes_of_the_shared_text() {
    let d = dir("uv");
    let json = run(&d, true, "{}");
    let (fe, cu) = (k_of(&json, "Fe"), k_of(&json, "Cu"));
    assert!(
        (fe - 0.04).abs() / 0.04 < 0.03 && (cu - 0.015).abs() / 0.015 < 0.03,
        "{fe} {cu}"
    );
    let read = |f: &str| std::fs::read(d.join(f)).unwrap();
    let (text, _) = mzlab_ops::run::table::kinetics::kinetics_text(
        &[],
        &read("work/q.csv"),
        &read("samples.json"),
        &read("uv.csv"),
    )
    .unwrap();
    assert_eq!(text, json);
    // a second run is the same bytes
    assert_eq!(run(&d, true, "{}"), json);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn without_the_uv_input_nothing_is_divided_and_the_log_says_so() {
    let d = dir("nouv");
    let json = run(&d, false, r#"{"boot":50}"#);
    assert!(json.contains(r#""uv":{"wired":false}"#), "{json}");
    assert!(k_of(&json, "Fe") > 0.0);
    let bad = crate::flow::run_plan_cached(
        plan(true, r#"{"uv_mode":"both"}"#).as_bytes(),
        &d,
        &d.join("work2"),
        1,
        None,
        Some(false),
    );
    assert!(bad.is_err() || bad.unwrap().1.contains("err.flow.kinetics.param"));
    let _ = std::fs::remove_dir_all(&d);
}

fn data() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/kinetics")
}

/// The committed inputs of the browser parity test (`tests/test_flowkin_wasm.py`): the 13 files above as plain CSV and JSON (areas of the
/// parent and of an ion that is never a peak, the UV table with a second, smaller peak per file), written by this test when `MZLAB_UPDATE_GOLDEN` is set.
fn write_inputs() {
    let mut areas = String::from("file,id,area,sn,state\n");
    let mut uv = String::from(
        "file,kind,rt_apex_min,rt_lo_min,rt_hi_min,fwhm_s,height_mau,area_mau_s,snr\n",
    );
    for (i, (n, t, k)) in series().iter().enumerate() {
        areas += &format!(
            "{n}.mzML,parent,{},50,detected\n{n}.mzML,ghost,,0,absent\n",
            1.0e6 * (-k * t).exp() * INJ[i]
        );
        uv += &format!(
            "{n}.mzML,narrow,4.3,4.2,4.4,6,10,{},90\n{n}.mzML,narrow,7.7,7.6,7.8,6,3,9.5,20\n",
            500.0 * INJ[i]
        );
    }
    let files: Vec<(String, String, String)> = series()
        .iter()
        .map(|s| (format!("{}.mzML", s.0), String::new(), String::new()))
        .collect();
    std::fs::create_dir_all(data()).unwrap();
    std::fs::write(data().join("areas.csv"), areas).unwrap();
    std::fs::write(data().join("uv.csv"), uv).unwrap();
    std::fs::write(
        data().join("samples.json"),
        mzlab_core::samples::to_json(&mzlab_core::samples::propose(&files)),
    )
    .unwrap();
}

#[test]
fn the_committed_series_gives_the_golden_file() {
    let update = std::env::var_os("MZLAB_UPDATE_GOLDEN").is_some();
    if update {
        write_inputs();
    }
    let d = std::env::temp_dir().join(format!("mzlab_cli_kinetics_golden_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("work")).unwrap();
    for f in ["areas.csv", "uv.csv", "samples.json"] {
        std::fs::copy(data().join(f), d.join(f)).unwrap();
    }
    let plan = r#"{"format":"mzflow/2","inputs":[{"name":"areas.csv","type":"report"},{"name":"samples.json","type":"samples"},{"name":"uv.csv","type":"peaks"}],"steps":[
        {"id":"k","step":"quant.kinetics@1","in":{"areas":"file:areas.csv","samples":"file:samples.json","uv":"file:uv.csv"},"params":{"ids":"parent;ghost"}}]}"#;
    let (run, prov) = crate::flow::run_plan(plan.as_bytes(), &d, &d.join("work"), 1, None).unwrap();
    assert!(
        run.skipped.is_empty() && !prov.contains("\"error\""),
        "{prov}"
    );
    let json = std::fs::read_to_string(d.join("work/k.json")).unwrap();
    let golden = data().join("golden.json");
    if update {
        std::fs::write(&golden, &json).unwrap();
    }
    assert_eq!(
        json,
        std::fs::read_to_string(&golden).unwrap(),
        "MZLAB_UPDATE_GOLDEN=1 rewrites it"
    );
    assert!(prov.contains("warn.flow.kinetics.notFitted"), "{prov}");
    let _ = std::fs::remove_dir_all(&d);
}
