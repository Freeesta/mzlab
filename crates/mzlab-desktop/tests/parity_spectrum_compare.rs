//! `spectrum compare` in the app (`rpc.rs` op `speccompare`), in the CLI and in the site: the page asks the engine for the `rebin=nominal` spectrum of each side and hands the
//! two answers to the function; the app's rpc, `mzlab_lib::speccmp` (what the library `.wasm` exports) and the bytes of `mzlab-cli spectrum compare` give the same text, and
//! the `.wasm` too when `MZLAB_SPECCMP_WASM_PKG` names its package (`tests/test_spectrum_compare_wasm.py`). A request the function refuses has the same error key everywhere.
use mzlab_desktop::rpc::Answer;
use mzlab_desktop::State;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A product-ion mzML of one scan at 1 min: precursor 253.1, the collision energy given, centroids at whole masses (numbers invented).
fn mzml(ce: f64, peaks: &[(f64, f32)]) -> Vec<u8> {
    use mzlab_core::mzmlwrite::{Header, MzmlWriter, SpectrumExtra};
    let mut w = MzmlWriter::new(Vec::new(), &Header::default()).unwrap();
    w.open_spectra(1).unwrap();
    let s = mzlab_core::mzml::Spectrum {
        index: 0,
        id: "scan=1".into(),
        level: 2,
        rt: 1.0,
        polarity: 1,
        filter: format!("ITMS + c ESI d Full ms2 253.1@cid{ce} [50.00-300.00]"),
        precursor: Some(253.1),
        mz: peaks.iter().map(|p| p.0).collect(),
        intensity: peaks.iter().map(|p| p.1).collect(),
        ..Default::default()
    };
    w.spectrum_with(&s, &SpectrumExtra { ce: Some(ce), ..Default::default() })
        .unwrap();
    w.close_spectra().unwrap();
    w.finish().unwrap()
}

/// The two files, the app with each open in a slot, and the folder.
struct Setup {
    dir: tempfile::TempDir,
    files: [PathBuf; 2],
    st: State,
}

fn setup() -> Setup {
    let dir = tempfile::tempdir().unwrap();
    let data = [
        ("a.mzML", 35.0, vec![(100.0, 4.0), (101.0, 9.0), (102.0, 16.0)]),
        ("b.mzML", 20.0, vec![(100.0, 4.0), (101.0, 9.0), (103.0, 25.0)]),
    ];
    let st = State::new();
    st.set_data_folder(dir.path().join("app"));
    st.set_index_folder(dir.path().join("idx")).unwrap();
    let mut files = [PathBuf::new(), PathBuf::new()];
    for (k, (name, ce, peaks)) in data.iter().enumerate() {
        files[k] = dir.path().join(name);
        std::fs::write(&files[k], mzml(*ce, peaks)).unwrap();
        let (fid, _) = st.pick(files[k].clone()).unwrap();
        st.rpc("lr_put", &json!({"name": name, "fid": fid}), &[]).unwrap();
        st.rpc("lr_index", &json!({"name": name}), &[]).unwrap();
        st.rpc("idxopen", &json!({"slot": k, "name": name}), &[]).unwrap();
    }
    Setup { dir, files, st }
}

fn text(a: Answer) -> String {
    match a {
        Answer::Text(t) => t,
        Answer::Bytes(_) => panic!("bytes"),
    }
}

/// The request the page builds: each side's name, route query, precursor, collision energies and the route's own answer.
fn sides(s: &Setup) -> String {
    let query = "rt0=1&rt1=1&level=2&precursor=253.1&rebin=nominal";
    let mut out = Vec::new();
    for (k, (name, ce)) in [("a.mzML", 35.0), ("b.mzML", 20.0)].iter().enumerate() {
        let answer = text(
            s.st.rpc("route", &json!({"slot": k, "path": "/api/spectrum", "query": query}), &[])
                .unwrap(),
        );
        out.push(format!(
            "{{\"name\":\"{name}\",\"query\":\"{query}\",\"precursor\":253.1,\"ce_values\":[{ce}],\"spectrum\":{answer}}}"
        ));
    }
    format!("{{\"a\":{},\"b\":{}}}", out[0], out[1])
}

/// The CLI command's own bytes for the same two selections.
fn cli(s: &Setup, set: Option<&str>) -> String {
    let mut a: Vec<String> = ["spectrum", "compare"].iter().map(|x| x.to_string()).collect();
    a.extend(s.files.iter().map(|f| f.to_string_lossy().into_owned()));
    a.extend(["--a-rt", "1", "--b-rt", "1", "--a-precursor", "253.1", "--b-precursor", "253.1"].map(String::from));
    if let Some(x) = set {
        a.extend(["--set".to_string(), x.to_string()]);
    }
    let o = mzlab_cli::execute(&a);
    assert_eq!(o.code, 0, "{}", o.stderr);
    String::from_utf8(o.stdout).unwrap()
}

/// The `.wasm` through Node, `{s}` or `{e}`.
fn wasm(s: &Setup, params: &str, sides: &str) -> Option<serde_json::Value> {
    let pkg = std::env::var("MZLAB_SPECCMP_WASM_PKG").ok()?;
    let (p, q) = (s.dir.path().join("params.json"), s.dir.path().join("sides.json"));
    std::fs::write(&p, params).unwrap();
    std::fs::write(&q, sides).unwrap();
    let out = Command::new("node")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/speccmp_wasm_node.js"))
        .args([Path::new(&pkg), &p, &q])
        .output()
        .expect("node runs");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    Some(serde_json::from_slice(&out.stdout).unwrap())
}

#[test]
fn spectrum_compare_matches_across_app_cli_and_wasm() {
    let s = setup();
    let sides = sides(&s);
    for (params, set) in [("{}", None), ("{\"tol\":1,\"ce_tol\":20}", Some("tol=1,ce_tol=20"))] {
        let want = mzlab_lib::speccmp::compare_text(params, &sides).unwrap();
        let app = text(
            s.st.rpc("speccompare", &json!({"params": params, "sides": sides}), &[])
                .unwrap(),
        );
        assert_eq!(app, want, "{params}");
        assert_eq!(cli(&s, set), format!("{want}\n"), "{params}");
        if let Some(w) = wasm(&s, params, &sides) {
            assert_eq!(w["s"].as_str(), Some(want.as_str()), "{params}: {w}");
        }
    }
    // the known cosine of these spectra: 13 / sqrt(29 * 38)
    let v: serde_json::Value = serde_json::from_str(&mzlab_lib::speccmp::compare_text("{}", &sides).unwrap()).unwrap();
    assert_eq!(v["cosine"], 0.391609);
    assert_eq!(v["warnings"][0]["key"], "warn.spectrum.ceDiffer");
}

#[test]
fn a_refused_request_has_the_same_error_key_everywhere() {
    let s = setup();
    let sides = sides(&s);
    for params in ["{\"nope\":1}", "{\"tol\":-1}"] {
        let e = mzlab_lib::speccmp::compare_json(params, &sides).unwrap_err();
        let app = s.st.rpc("speccompare", &json!({"params": params, "sides": sides}), &[]);
        let key = if params.contains("nope") { "err.spectrum.compare.param" } else { "err.spectrum.compare.value" };
        assert!(e.contains(key), "{e}");
        assert!(app.err().unwrap().to_string().contains(key), "{params}");
        if let Some(w) = wasm(&s, params, &sides) {
            assert!(w["e"].as_str().unwrap().contains(key), "{w}");
        }
    }
}
