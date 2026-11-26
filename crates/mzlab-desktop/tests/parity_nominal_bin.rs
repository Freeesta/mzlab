//! `rebin=nominal|0.1` and `mzoff=` of `/api/spectrum` (`spectrum get --bin`): the app, the site's engine (an index in memory), the CLI (an index on disk),
//! the CLI command's own bytes and the engine `.wasm` (`tests/rotte_wasm_node.js`, `MZLAB_WASM_PKG`, run by `tests/test_parita_rotte_app.py`) give the same
//! text on a unit-resolution profile file whose fragment is split between 106.7 and 107.0; a question the route refuses has the same error everywhere.
use mzlab_core::idxread::Index;
use mzlab_core::routes::{Query, Routes};
use mzlab_desktop::rpc::Answer;
use mzlab_desktop::State;
use serde_json::{json, Value};
use std::path::Path;
use std::process::Command;

/// Four profile scans of 0.1 Da points on 100.02 + 0.1 i, a baseline of 5 and a fragment of six points from 106.62 to 107.12. Numbers invented.
fn split_fragment_mzml() -> Vec<u8> {
    use mzlab_core::mzml::Spectrum;
    use mzlab_core::mzmlwrite::{Header, MzmlWriter};
    let mut w = MzmlWriter::new(Vec::new(), &Header::default()).unwrap();
    w.open_spectra(4).unwrap();
    let peak = [400.0f32, 800.0, 1000.0, 1000.0, 800.0, 400.0];
    for k in 0..4 {
        let mz: Vec<f64> = (0..100).map(|i| 100.02 + 0.1 * i as f64).collect();
        let intensity: Vec<f32> = (0..100usize)
            .map(|i| {
                5.0 + i
                    .checked_sub(66)
                    .and_then(|j| peak.get(j))
                    .copied()
                    .unwrap_or(0.0)
                    + k as f32
            })
            .collect();
        let s = Spectrum {
            index: k,
            id: format!("scan={}", k + 1),
            level: 1,
            rt: k as f64,
            polarity: 1,
            profile: true,
            filter: "ITMS + p ESI Full ms [100.00-110.00]".into(),
            mz,
            intensity,
            ..Default::default()
        };
        w.spectrum(&s).unwrap();
    }
    w.close_spectra().unwrap();
    w.finish().unwrap()
}

const QUERIES: [&str; 6] = [
    "?k=0&rt0=1&rt1=1&level=1&rebin=nominal",
    "?k=0&rt0=1&rt1=1&level=1&rebin=nominal&mzoff=0.24",
    "?k=0&rt0=0&rt1=3&level=1&rebin=nominal&mzoff=0.1",
    "?k=0&rt0=0&rt1=3&level=1&rebin=0.1",
    "?k=0&rt0=1&rt1=1&level=1&rebin=nominal&mzoff=auto:300",
    "?k=0&rt0=1&rt1=1&level=1&rebin=7",
];

fn text(a: Answer) -> String {
    match a {
        Answer::Text(t) => t,
        Answer::Bytes(_) => panic!("bytes"),
    }
}

/// The file, the app's state with it open in slot 0 and the index the site and the CLI read.
struct Setup {
    dir: tempfile::TempDir,
    file: std::path::PathBuf,
    st: State,
    idx_path: std::path::PathBuf,
}

fn setup() -> Setup {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("split.mzML");
    std::fs::write(&file, split_fragment_mzml()).unwrap();
    let st = State::new();
    st.set_data_folder(dir.path().join("app"));
    st.set_index_folder(dir.path().join("idx")).unwrap();
    let (fid, _) = st.pick(file.clone()).unwrap();
    st.rpc("lr_put", &json!({"name": "split.mzML", "fid": fid}), &[])
        .unwrap();
    st.rpc("lr_index", &json!({"name": "split.mzML"}), &[])
        .unwrap();
    st.rpc("idxopen", &json!({"slot": 0, "name": "split.mzML"}), &[])
        .unwrap();
    let idx_path = dir.path().join("idx").join("split.mzML.mzidx");
    Setup {
        dir,
        file,
        st,
        idx_path,
    }
}

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

fn json_of(a: &Option<String>) -> Value {
    serde_json::from_str(a.as_ref().unwrap()).unwrap()
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
