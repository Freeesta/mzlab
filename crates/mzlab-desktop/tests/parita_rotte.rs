//! The routes both engines share, one by one: the same plan of steps (index routes, file info, channels, scans from the mzML, maps, TIC/XIC/spectra of a whole run)
//! goes to the desktop engine and to the engine `.wasm` (`mzlab-wasm`, wasm-bindgen `--target nodejs`, run by `tests/rotte_wasm_node.js`) on the same example and the
//! same `.mzidx`, and every answer must be the same: the text, the numbers or the bytes, or the key of the error. `MZLAB_WASM_PKG` names the folder of the built module;
//! `tests/test_parita_rotte_app.py` builds it and runs this test. Without it (or without node) the test does nothing.
use mzlab_desktop::rpc::Answer;
use mzlab_desktop::State;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command;

const NAME: &str = "FullScan_t10.mzML";

fn example() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../mzlab/web/esempi")
        .join(NAME)
}

/// One answer, as both engines give it.
#[derive(Debug, PartialEq)]
enum Got {
    Text(String),
    Numbers(Vec<f64>),
    Bytes(Vec<u8>),
    Err(String),
}

fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

/// The answer of the wasm side: `{s}`, `{f64}`, `{b}` or `{e}` (the JSON `{error_key, ...}` the module throws, or the `E`-text of a user's error).
fn from_wasm(a: &Value) -> Got {
    if let Some(s) = a["s"].as_str() {
        return Got::Text(s.to_string());
    }
    if let Some(v) = a["f64"].as_array() {
        return Got::Numbers(v.iter().map(|x| x.as_f64().unwrap()).collect());
    }
    if let Some(b) = a["b"].as_str() {
        return Got::Bytes(hex(b));
    }
    let e = a["e"].as_str().unwrap_or("");
    Got::Err(
        serde_json::from_str::<Value>(e)
            .ok()
            .and_then(|j| j["error_key"].as_str().map(String::from))
            .unwrap_or_else(|| e.to_string()),
    )
}

/// The same step on the desktop engine.
fn on_desktop(st: &State, fid: u32, s: &Value) -> Got {
    let slot = s["slot"].as_u64().unwrap_or(0) as u32;
    let mut p = s.clone();
    let op = s["op"].as_str().unwrap().to_string();
    let rpc = |p: &Value| match st.rpc(&op, p, &[]) {
        Ok(Answer::Text(t)) => Got::Text(t),
        Ok(Answer::Bytes(b)) => Got::Bytes(b),
        Err(e) => Got::Err(e.key.to_string()),
    };
    let num = |k: &str| s[k].as_f64().unwrap_or(0.0);
    let nums = |r: mzlab_core::Result<Vec<f64>>| {
        r.map_or_else(|e| Got::Err(e.key.to_string()), Got::Numbers)
    };
    match op.as_str() {
        "open" => st
            .open(slot, fid)
            .map_or_else(|e| Got::Err(e.key.to_string()), Got::Text),
        "tic" => nums(st.tic(slot, num("level") as u8, num("px") as u32)),
        "xic" => nums(st.xic(
            slot,
            num("level") as u8,
            num("mz"),
            num("tol"),
            num("px") as u32,
        )),
        "spectra" => nums(st.spectra(
            slot,
            num("level") as u8,
            num("i0") as u32,
            num("i1") as u32,
            num("bin"),
        )),
        _ => {
            p["fid"] = json!(fid);
            rpc(&p)
        }
    }
}

/// The steps both engines get, in order; the last three read a slot that was never opened (an error in both).
fn plan() -> Vec<Value> {
    let items = json!([{"slot": 0, "label": NAME, "time": 0.0, "full": true}]).to_string();
    let r =
        |path: &str, query: &str| json!({"op": "route", "slot": 0, "path": path, "query": query});
    let mut steps = vec![
        json!({"op": "idxopen", "slot": 0, "name": NAME}),
        r("/api/chrom", "?k=0&kind=tic&level=1"),
        r("/api/chrom", "?k=0&kind=bpc&level=1"),
        r("/api/xic", "?k=0&mz=300&tol=0.5&level=1"),
        r("/api/filters", ""),
        r("/api/scanlist", "?k=0&level=1"),
        r("/api/nearest_scan", "?k=0&rt=0.5&level=1"),
        r("/api/scan_axis", "?k=0&level=1"),
        r("/api/ms2tr", ""),
        r("/api/spectrum", "?k=0&level=1&rt0=0.2&rt1=0.4"),
        r("/api/dda", "?k=0"),
        r("/api/msn_albero", "?k=0"),
        r("/api/nope", ""),
        json!({"op": "route", "slot": 9, "path": "/api/chrom", "query": ""}),
        json!({"op": "fileinfo", "slot": 0, "name": NAME}),
        json!({"op": "canali", "slot": 0, "name": NAME}),
        json!({"op": "mrm", "slot": 0}),
        json!({"op": "mrmchrom", "slot": 0, "kind": "tic"}),
        json!({"op": "mzmlroute", "slot": 0, "path": "/api/spectra", "query": "?k=0&level=1&rt0=0.2&rt1=0.4"}),
        json!({"op": "mzmlroute", "slot": 0, "path": "/api/scan", "query": "?k=0&sid=3"}),
        json!({"op": "scanbin", "slot": 0, "query": "?k=0&sid=3"}),
        json!({"op": "specexact", "slot": 0, "query": "?k=0&level=1&rt0=0.2&rt1=0.2"}),
        json!({"op": "map", "path": "/api/map", "items": items, "query": "?k=0&level=1"}),
        json!({"op": "map", "path": "/api/mapsnap", "items": items, "query": "?k=0&level=1"}),
        json!({"op": "open", "slot": 7}),
        json!({"op": "tic", "slot": 7, "level": 1, "px": 0}),
        json!({"op": "tic", "slot": 7, "level": 1, "px": 200}),
        json!({"op": "xic", "slot": 7, "level": 1, "mz": 300.0, "tol": 0.5, "px": 0}),
        json!({"op": "spectra", "slot": 7, "level": 1, "i0": 0, "i1": 5, "bin": 0.0}),
        json!({"op": "spectra", "slot": 7, "level": 1, "i0": 0, "i1": 5, "bin": 1.0}),
    ];
    steps.extend(stat_steps());
    steps.extend(name_steps());
    steps.extend([
        json!({"op": "tic", "slot": 8, "level": 1, "px": 0}),
        json!({"op": "xic", "slot": 8, "level": 1, "mz": 300.0, "tol": 0.5, "px": 0}),
        json!({"op": "spectra", "slot": 8, "level": 1, "i0": 0, "i1": 5, "bin": 0.0}),
    ]);
    steps
}

/// The name operations of `campioni` (`variabili`, `nome_gruppo`): the worked list of the study, the manual variables, a long name and a bad operation.
fn name_steps() -> Vec<Value> {
    let mut names = vec![
        "MQ_blk",
        "MQ_Wash_1",
        "MQ_Wash_2",
        "TW_blk",
        "MQ_std_BTZ_1ppm",
        "QC_Std_5ppm_mid",
    ]
    .into_iter()
    .map(String::from)
    .collect::<Vec<_>>();
    for (m, conds) in [
        ("MQ", ["H2O2", "N2", "O2", "PMS"]),
        ("TW", ["AIR", "H2O2", "", ""]),
    ] {
        for c in conds.iter().filter(|c| !c.is_empty()) {
            for t in ["T00", "T01", "T02", "T05"] {
                names.push(format!("{m}_{c}_{t}"));
            }
        }
    }
    let files: Vec<Value> = names.iter().map(|n| json!({"name": n})).collect();
    let step =
        |cop: &str, body: Value| json!({"op": "lr_campioni", "cop": cop, "body": body.to_string()});
    vec![
        step("variabili", json!({"files": files})),
        step(
            "variabili",
            json!({"files": files, "manuale": {"pos": -1, "sep": "_"}}),
        ),
        step(
            "variabili",
            json!({"files": files, "manuale": {"has": ["O2", "H2O2"], "substring": true}}),
        ),
        step(
            "nome_gruppo",
            json!({"names": ["MQ_H2O2_T00", "MQ_H2O2_T01"]}),
        ),
        step("variabili", json!({"files": [{"name": "x".repeat(257)}]})),
        step("nome_gruppo", json!({"names": [1]})),
        step("nope", json!({})),
    ]
}

/// The first case of each statistics route of the committed reference (`tests/golden/ref/statistica.json`, invented tables): profile, blank, reference, PCA.
fn stat_steps() -> Vec<Value> {
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/ref/statistica.json");
    let cases: Vec<Value> = serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();
    let mut seen = Vec::new();
    let mut out = Vec::new();
    for c in &cases {
        let path = c["path"].as_str().unwrap().to_string();
        if !seen.contains(&path) {
            out.push(json!({"op": "lr_stat", "path": path, "body": c["body"].to_string()}));
            seen.push(path);
        }
    }
    out
}

/// Runs the plan on the built `.wasm` (Node) over the same `.mzidx` and example.
fn run_wasm(pkg: &str, dir: &Path, idx: &Path, mzml: &Path, plan: &[Value]) -> Vec<Value> {
    let plan_file = dir.join("plan.json");
    std::fs::write(&plan_file, serde_json::to_string(&plan).unwrap()).unwrap();
    let out = Command::new("node")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/rotte_wasm_node.js"))
        .arg(pkg)
        .arg(idx)
        .arg(mzml)
        .arg(&plan_file)
        .output()
        .expect("node runs");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let wasm: Vec<Value> = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(wasm.len(), plan.len());
    wasm
}

/// Every answer of the app must be the one of the `.wasm`; the plan must not be vacuous (the main routes answer).
fn compare(st: &State, fid: u32, plan: &[Value], wasm: &[Value], answers: &[usize]) {
    let mut differ = Vec::new();
    for (s, w) in plan.iter().zip(wasm) {
        let (app, site) = (on_desktop(st, fid, s), from_wasm(w));
        if app != site {
            differ.push(format!("{s}\n  app:  {app:.300?}\n  wasm: {site:.300?}"));
        }
    }
    assert!(
        differ.is_empty(),
        "{} of {} routes differ:\n{}",
        differ.len(),
        plan.len(),
        differ.join("\n")
    );
    // the plan is not vacuous: the main routes do answer
    for &k in answers {
        let nothing = match from_wasm(&wasm[k]) {
            Got::Err(_) => true,
            Got::Text(t) => t.is_empty(),
            Got::Numbers(v) => v.is_empty(),
            Got::Bytes(b) => b.is_empty(),
        };
        assert!(
            !nothing,
            "step {k} gave nothing: {} -> {:?}",
            plan[k], wasm[k]
        );
    }
}

#[test]
fn every_common_route_gives_the_same_answer_in_the_app_and_in_the_wasm() {
    let Ok(pkg) = std::env::var("MZLAB_WASM_PKG") else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let st = State::new();
    st.set_data_folder(dir.path().join("app"));
    st.set_index_folder(dir.path().join("idx")).unwrap();
    let (fid, _) = st.pick(example()).unwrap();
    st.rpc("lr_put", &json!({"name": NAME, "fid": fid}), &[])
        .unwrap();
    st.rpc("lr_index", &json!({"name": NAME}), &[]).unwrap();
    let plan = plan();
    let wasm = run_wasm(
        &pkg,
        dir.path(),
        &dir.path().join("idx").join(format!("{NAME}.mzidx")),
        &example(),
        &plan,
    );
    compare(
        &st,
        fid,
        &plan,
        &wasm,
        &[1, 14, 15, 22, 25, 26, 30, 31, 32, 33],
    );
    // a slot that was never opened is an error with the same key in both
    for w in &wasm[wasm.len() - 3..] {
        assert_eq!(from_wasm(w), Got::Err("err.file.raw".into()));
    }
}

/// A small SRM acquisition made for the test: three MS1 scans and three SRM traces, written by the writer of the core.
fn srm_mzml() -> Vec<u8> {
    use mzlab_core::mzml::{ChromKind, Chromatogram, Spectrum};
    use mzlab_core::mzmlwrite::{Header, MzmlWriter};
    let h = Header {
        run: "srm".into(),
        sources: vec!["srm.mzML".into()],
        software: ("mzLab".into(), "0".into()),
        ..Default::default()
    };
    let mut w = MzmlWriter::new(Vec::new(), &h).unwrap();
    w.open_spectra(3).unwrap();
    for t in 0..3 {
        let s = Spectrum {
            index: t,
            id: format!("scan={}", t + 1),
            level: 1,
            rt: t as f64 * 0.1,
            polarity: 1,
            mz: vec![200.0, 300.0],
            intensity: vec![100.0, 50.0 + t as f32],
            ..Default::default()
        };
        w.spectrum(&s).unwrap();
    }
    w.close_spectra().unwrap();
    w.open_chromatograms(3).unwrap();
    for (i, (q1, q3)) in [(300.1, 150.2), (300.1, 180.3), (350.2, 200.1)]
        .into_iter()
        .enumerate()
    {
        let time: Vec<f64> = (0..20).map(|k| k as f64 * 0.05).collect();
        let intensity: Vec<f64> = time
            .iter()
            .map(|t| 1000.0 * (i as f64 + 1.0) * (-((t - 0.5) / 0.1).powi(2)).exp() + 5.0)
            .collect();
        let c = Chromatogram {
            index: i,
            id: format!("SRM SIC Q1={q1} Q3={q3} ce=20 name=T{i}"),
            kind: ChromKind::Srm,
            q1: Some(q1),
            q3: Some(q3),
            unit: String::new(),
            time,
            intensity,
        };
        w.chromatogram(&c).unwrap();
    }
    w.close_chromatograms().unwrap();
    w.finish().unwrap()
}

#[test]
fn an_srm_file_gives_the_same_transitions_and_traces_in_the_app_and_in_the_wasm() {
    let Ok(pkg) = std::env::var("MZLAB_WASM_PKG") else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("srm.mzML");
    std::fs::write(&file, srm_mzml()).unwrap();
    let st = State::new();
    st.set_data_folder(dir.path().join("app"));
    st.set_index_folder(dir.path().join("idx")).unwrap();
    let (fid, _) = st.pick(file.clone()).unwrap();
    st.rpc("lr_put", &json!({"name": "srm.mzML", "fid": fid}), &[])
        .unwrap();
    st.rpc("lr_index", &json!({"name": "srm.mzML"}), &[])
        .unwrap();
    let r =
        |path: &str, query: &str| json!({"op": "route", "slot": 0, "path": path, "query": query});
    let plan = vec![
        json!({"op": "idxopen", "slot": 0, "name": "srm.mzML"}),
        json!({"op": "mrm", "slot": 0}),
        json!({"op": "mrmchrom", "slot": 0, "kind": "tic"}),
        json!({"op": "mrmchrom", "slot": 0, "kind": "bpc"}),
        json!({"op": "canali", "slot": 0, "name": "srm.mzML"}),
        json!({"op": "fileinfo", "slot": 0, "name": "srm.mzML"}),
        r("/api/filters", ""),
        r("/api/chrom", "?k=0&kind=tic&level=1"),
    ];
    let idx = dir.path().join("idx").join("srm.mzML.mzidx");
    let wasm = run_wasm(&pkg, dir.path(), &idx, &file, &plan);
    compare(&st, fid, &plan, &wasm, &[1, 2, 4, 5]);
    // the transitions of the file are in the answer
    let Got::Text(mrm) = from_wasm(&wasm[1]) else {
        panic!("mrm gave no text")
    };
    assert!(mrm.contains("150.2") && mrm.contains("200.1"), "{mrm}");
}
