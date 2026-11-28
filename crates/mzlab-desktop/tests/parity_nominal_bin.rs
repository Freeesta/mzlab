//! `rebin=nominal|0.1` and `mzoff=` of `/api/spectrum` (`spectrum get --bin`): the app, the site's engine (an index in memory), the CLI (an index on disk),
//! the CLI command's own bytes and the engine `.wasm` (`tests/rotte_wasm_node.js`, `MZLAB_WASM_PKG`, run by `tests/test_parita_rotte_app.py`) give the same
//! text on a unit-resolution profile file whose fragment is split between 106.7 and 107.0; a question the route refuses has the same error everywhere.
mod nominal_common;
use mzlab_core::idxread::Index;
use mzlab_core::routes::{Query, Routes};
use mzlab_desktop::State;
use nominal_common::{json_of, setup, text, Setup};
use serde_json::{json, Value};
use std::path::Path;
use std::process::Command;

const QUERIES: [&str; 6] = [
    "?k=0&rt0=1&rt1=1&level=1&rebin=nominal",
    "?k=0&rt0=1&rt1=1&level=1&rebin=nominal&mzoff=0.24",
    "?k=0&rt0=0&rt1=3&level=1&rebin=nominal&mzoff=0.1",
    "?k=0&rt0=0&rt1=3&level=1&rebin=0.1",
    "?k=0&rt0=1&rt1=1&level=1&rebin=nominal&mzoff=auto:300",
    "?k=0&rt0=1&rt1=1&level=1&rebin=7",
];

/// The three native engines answer each query alike (or refuse it with the same key); `None` is a refusal.
fn native_answers(s: &Setup) -> Vec<Option<String>> {
    let idx_bytes = std::fs::read(&s.idx_path).unwrap();
    let len = idx_bytes.len() as u64;
    let mut site = Routes::new(Index::open(idx_bytes, len).unwrap());
    let mut cli = Routes::new(Index::open(std::fs::File::open(&s.idx_path).unwrap(), len).unwrap());
    let mut answers = Vec::new();
    for q in QUERIES {
        let app =
            s.st.rpc(
                "route",
                &json!({"slot": 0, "path": "/api/spectrum", "query": q}),
                &[],
            )
            .map(text);
        let site = site.handle("/api/spectrum", &Query::parse(q));
        let cli = cli.handle("/api/spectrum", &Query::parse(q));
        match (&app, &site, &cli) {
            (Ok(a), Ok(Some(s)), Ok(Some(c))) => {
                assert_eq!((a, a), (s, c), "{q}");
                answers.push(Some(a.clone()));
            }
            (Err(_), Err(s), Err(c)) => {
                assert_eq!(
                    (s.key, c.key),
                    ("err.spectrum.rebin", "err.spectrum.rebin"),
                    "{q}"
                );
                answers.push(None);
            }
            (a, s, c) => panic!(
                "{q}: app {:?}, site {:?}, cli {:?}",
                a.as_ref().err().map(|e| e.to_string()),
                s.as_ref().err().map(|e| e.key),
                c.as_ref().err().map(|e| e.key)
            ),
        }
    }
    answers
}

/// The CLI command holds the route's tokens: bin, centre of mass, counts and points of the offset question.
fn cli_command_matches(s: &Setup, want: &Value) {
    let v: Vec<String> = [
        "spectrum",
        "get",
        &s.file.to_string_lossy(),
        "--rt",
        "1",
        "--bin",
        "nominal",
        "--mz-offset",
        "0.24",
    ]
    .iter()
    .map(|x| x.to_string())
    .collect();
    let o = mzlab_cli::execute(&v);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let got: Value = serde_json::from_slice(&o.stdout).unwrap();
    for (i, b) in got["bins"].as_array().unwrap().iter().enumerate() {
        assert_eq!(b["bin"], want["bin"][i]);
        assert_eq!(b["mz"], want["mz"][i]);
        assert_eq!(b["counts"], want["y"][i]);
        assert_eq!(b["points"], want["points"][i]);
    }
}

/// The engine `.wasm` gives the same text (or a refusal with the same key) when `MZLAB_WASM_PKG` points at its package.
fn wasm_matches(s: &Setup, answers: &[Option<String>]) {
    let Ok(pkg) = std::env::var("MZLAB_WASM_PKG") else {
        return;
    };
    let mut plan = vec![json!({"op": "idxopen", "slot": 0, "name": "split.mzML"})];
    plan.extend(
        QUERIES
            .iter()
            .map(|q| json!({"op": "route", "slot": 0, "path": "/api/spectrum", "query": q})),
    );
    let plan_file = s.dir.path().join("plan.json");
    std::fs::write(&plan_file, serde_json::to_string(&plan).unwrap()).unwrap();
    let out = Command::new("node")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/rotte_wasm_node.js"))
        .args([Path::new(&pkg), &s.idx_path, &s.file, &plan_file])
        .output()
        .expect("node runs");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let wasm: Vec<Value> = serde_json::from_slice(&out.stdout).unwrap();
    for (k, q) in QUERIES.iter().enumerate() {
        match &answers[k] {
            Some(a) => assert_eq!(wasm[k + 1]["s"].as_str(), Some(a.as_str()), "{q}"),
            None => assert!(
                wasm[k + 1]["e"]
                    .as_str()
                    .unwrap()
                    .contains("err.spectrum.rebin"),
                "{q}: {}",
                wasm[k + 1]
            ),
        }
    }
}

#[test]
fn nominal_bins_match_across_site_app_cli_and_wasm() {
    let s = setup();
    let answers = native_answers(&s);
    // the first answer is the one bin the fragment fills: 107, ten points, 4450 + 10 counts (the baseline of the scan)
    let first = json_of(&answers[0]);
    assert_eq!(first["mode"], "bins");
    assert_eq!(first["bin"][7], 107.0);
    assert_eq!(first["points"][7], 10);
    assert_eq!(first["mz_offset_da"], 0.0);
    let off = json_of(&answers[1]);
    assert_eq!(off["mz_offset_da"], 0.24);
    assert_eq!(off["bin"][6], 106.0);
    assert_eq!(json_of(&answers[4])["mz_offset"]["status"], "not_found");
    cli_command_matches(&s, &off);
    wasm_matches(&s, &answers);
}

/// Three scans of a high-resolution PROFILE file (`FTMS` filter, 64-bit m/z every 0.004 Da, so a single-precision index section) with a baseline of
/// 10 and a peak at 305.0702 that the edge 305.05 cuts. Numbers invented.
fn hr_profile_mzml() -> Vec<u8> {
    use mzlab_core::mzml::Spectrum;
    use mzlab_core::mzmlwrite::{Header, MzmlWriter};
    let mut w = MzmlWriter::new(Vec::new(), &Header::default()).unwrap();
    w.open_spectra(3).unwrap();
    for k in 0..3 {
        let mz: Vec<f64> = (0..2500).map(|i| 300.0 + 0.004 * i as f64).collect();
        let intensity = mz
            .iter()
            .map(|m| (10.0 + 5.0e4 * (-0.5 * ((m - 305.0702) / 0.01).powi(2)).exp()) as f32)
            .collect();
        let s = Spectrum {
            index: k,
            id: format!("scan={}", k + 1),
            level: 1,
            rt: k as f64,
            polarity: 1,
            profile: true,
            filter: "FTMS + p ESI Full ms [300.0000-310.0000]".into(),
            mz,
            intensity,
            ..Default::default()
        };
        w.spectrum(&s).unwrap();
    }
    w.close_spectra().unwrap();
    w.finish().unwrap()
}

const HR_QUERIES: [&str; 3] = [
    "?k=0&rt0=1&rt1=1&level=1&rebin=0.1",
    "?k=0&rt0=0&rt1=2&level=1&rebin=nominal&mzoff=0.24",
    "?k=0&rt0=0&rt1=2&level=1&rebin=0.1&mzoff=0.02",
];

/// The app (`specexact`), the site's engine and the CLI's engine (`mzml_route` over the mzML) answer each HR query alike; returns the answers and the
/// index path. The index route alone has no spacing for the single-precision section until the first `specexact`, as for the unbinned spectrum.
fn hr_native_answers(dir: &Path, file: &Path) -> (Vec<String>, std::path::PathBuf) {
    let st = State::new();
    st.set_data_folder(dir.join("app"));
    st.set_index_folder(dir.join("idx")).unwrap();
    let (fid, _) = st.pick(file.to_path_buf()).unwrap();
    let rpc = |op: &str, p: Value| text(st.rpc(op, &p, &[]).unwrap());
    rpc("lr_put", json!({"name": "hrp.mzML", "fid": fid}));
    rpc("lr_index", json!({"name": "hrp.mzML"}));
    rpc("idxopen", json!({"slot": 0, "name": "hrp.mzML"}));
    let idx_path = dir.join("idx").join("hrp.mzML.mzidx");
    let idx_bytes = std::fs::read(&idx_path).unwrap();
    let len = idx_bytes.len() as u64;
    let mut site = Routes::new(Index::open(idx_bytes, len).unwrap());
    let mut cli = Routes::new(Index::open(std::fs::File::open(&idx_path).unwrap(), len).unwrap());
    let (mut mz_site, mut mz_cli) = (
        std::fs::File::open(file).unwrap(),
        std::fs::File::open(file).unwrap(),
    );
    let mut answers = Vec::new();
    for (n, q) in HR_QUERIES.into_iter().enumerate() {
        let route = rpc(
            "route",
            json!({"slot": 0, "path": "/api/spectrum", "query": q}),
        );
        // only before the first `specexact` is the spacing unknown; it then stays with the open index
        assert_eq!(
            route.is_empty(),
            n == 0,
            "{q}: the index alone gives no spacing"
        );
        let app = rpc("specexact", json!({"slot": 0, "fid": fid, "query": q}));
        assert!(route.is_empty() || route == app, "{q}");
        let query = Query::parse(q);
        let s = site
            .mzml_route("/api/spectrum", &query, &mut mz_site)
            .unwrap();
        let c = cli
            .mzml_route("/api/spectrum", &query, &mut mz_cli)
            .unwrap();
        assert_eq!((Some(&app), Some(&app)), (s.as_ref(), c.as_ref()), "{q}");
        answers.push(app);
    }
    (answers, idx_path)
}

/// The CLI command holds the route's tokens: bin, centre of mass, counts and points.
fn hr_cli_command_matches(file: &Path, want: &Value) {
    let a = [
        "spectrum",
        "get",
        &file.to_string_lossy(),
        "--rt",
        "0,2",
        "--bin",
        "nominal",
        "--mz-offset",
        "0.24",
    ];
    let o = mzlab_cli::execute(&a.iter().map(|x| x.to_string()).collect::<Vec<_>>());
    assert_eq!(o.code, 0, "{}", o.stderr);
    let got: Value = serde_json::from_slice(&o.stdout).unwrap();
    for (i, b) in got["bins"].as_array().unwrap().iter().enumerate() {
        let w = (
            &want["bin"][i],
            &want["mz"][i],
            &want["y"][i],
            &want["points"][i],
        );
        assert_eq!((&b["bin"], &b["mz"], &b["counts"], &b["points"]), w);
    }
}

/// The engine `.wasm` (`MZLAB_WASM_PKG`) gives the same text through `spectrum_exact`.
fn hr_wasm_matches(dir: &Path, (file, idx_path): (&Path, &Path), answers: &[String]) {
    let Ok(pkg) = std::env::var("MZLAB_WASM_PKG") else {
        return;
    };
    let mut plan = vec![json!({"op": "idxopen", "slot": 0, "name": "hrp.mzML"})];
    plan.extend(
        HR_QUERIES
            .iter()
            .map(|q| json!({"op": "specexact", "slot": 0, "query": q})),
    );
    let plan_file = dir.join("plan.json");
    std::fs::write(&plan_file, serde_json::to_string(&plan).unwrap()).unwrap();
    let out = Command::new("node")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/rotte_wasm_node.js"))
        .args([Path::new(&pkg), idx_path, file, &plan_file])
        .output()
        .expect("node runs");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let wasm: Vec<Value> = serde_json::from_slice(&out.stdout).unwrap();
    for (k, q) in HR_QUERIES.iter().enumerate() {
        assert_eq!(wasm[k + 1]["s"].as_str(), Some(answers[k].as_str()), "{q}");
    }
}

#[test]
fn bins_of_an_hr_profile_file_match_across_site_app_cli_and_wasm() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("hrp.mzML");
    std::fs::write(&file, hr_profile_mzml()).unwrap();
    let (answers, idx_path) = hr_native_answers(dir.path(), &file);
    let first: Value = serde_json::from_str(&answers[0]).unwrap();
    assert_eq!(first["mode"], "bins");
    assert_eq!(first["width_da"].as_f64(), Some(0.1));
    assert_eq!(first["mz_offset_da"].as_f64(), Some(0.0));
    let off: Value = serde_json::from_str(&answers[1]).unwrap();
    assert_eq!(off["mz_offset_da"].as_f64(), Some(0.24));
    assert_eq!(off["rule"], "nominal");
    hr_cli_command_matches(&file, &off);
    hr_wasm_matches(dir.path(), (&file, &idx_path), &answers);
}
