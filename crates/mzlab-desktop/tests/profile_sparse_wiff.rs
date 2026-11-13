//! The spectrum of sparse product-ion scans of a 3200 QTRAP `.wiff` (`tools/wiff_prodotto_rado.py --spaced` and `--single`: no two neighbouring bins,
//! one bin a scan): the app, the routes the CLI opens on the same `.mzidx` and the engine `.wasm` (`tests/rotte_wasm_node.js`, `MZLAB_WASM_PKG`, run
//! by `tests/test_parita_rotte_app.py`) give the same text, and it is a profile line (the index alone answers). Skipped without python3.
use mzlab_core::idxread::Index;
use mzlab_core::routes::{Query, Routes};
use mzlab_desktop::rpc::Answer;
use mzlab_desktop::State;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command;

/// The windows the page asks: one scan of one bin, one scan of several, the whole run.
const WINDOWS: [&str; 3] = [
    "?k=0&rt0=0.015&rt1=0.035&level=2&precursor=",
    "?k=0&rt0=0.04&rt1=0.06&level=2&precursor=",
    "?k=0&rt0=0&rt1=10&level=2&precursor=",
];

fn pair(dir: &Path, flag: &str) -> Option<PathBuf> {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/wiff_prodotto_rado.py");
    let ok = Command::new("python3")
        .arg(script)
        .arg(dir)
        .arg(flag)
        .status()
        .is_ok_and(|s| s.success());
    let name = if flag == "--spaced" {
        "spaziato"
    } else {
        "singolo"
    };
    ok.then(|| dir.join(format!("{name}.wiff")))
}

fn text(a: Answer) -> String {
    match a {
        Answer::Text(t) => t,
        Answer::Bytes(_) => panic!("bytes"),
    }
}

/// The pair put and indexed in the app (sample 0, as the page sends a native sample), open in slot 0; returns its name and its `.mzidx`.
fn register(st: &State, dir: &Path, wiff: &Path) -> (String, PathBuf) {
    let n = wiff.file_name().unwrap().to_str().unwrap().to_string();
    let mut members = Vec::new();
    for (name, p) in [
        (n.clone(), wiff.to_path_buf()),
        (
            format!("{n}.scan"),
            PathBuf::from(format!("{}.scan", wiff.display())),
        ),
    ] {
        let (fid, _) = st.pick(p).unwrap();
        members.push(json!({"name": name, "fid": fid}));
    }
    let nat = json!({"key": n, "c": 0, "id": 0, "centroid": false, "members": members});
    st.rpc("lr_put", &json!({"name": n, "nat": nat}), &[])
        .unwrap();
    st.rpc("lr_index", &json!({"name": n}), &[]).unwrap();
    st.rpc("idxopen", &json!({"slot": 0, "name": n}), &[])
        .unwrap();
    let idx = std::fs::read_dir(dir.join("idx"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.extension().is_some_and(|x| x == "mzidx"))
        .unwrap();
    (n, idx)
}

#[test]
fn sparse_wiff_spectra_match_across_site_desktop_and_cli() {
    for flag in ["--spaced", "--single"] {
        let dir = tempfile::tempdir().unwrap();
        let Some(wiff) = pair(&dir.path().join("in"), flag) else {
            eprintln!("SKIP profile_sparse_wiff: python3 not available");
            return;
        };
        let st = State::new();
        st.set_data_folder(dir.path().join("app"));
        st.set_index_folder(dir.path().join("idx")).unwrap();
        let (_, idx) = register(&st, dir.path(), &wiff);
        let len = std::fs::metadata(&idx).unwrap().len();
        let mut cli = Routes::new(Index::open(std::fs::File::open(&idx).unwrap(), len).unwrap());
        let mut app = Vec::new();
        for q in WINDOWS {
            let desktop = text(
                st.rpc(
                    "route",
                    &json!({"slot": 0, "path": "/api/spectrum", "query": q}),
                    &[],
                )
                .unwrap(),
            );
            let v: Value = serde_json::from_str(&desktop)
                .unwrap_or_else(|_| panic!("{flag} {q}: {desktop:.200}"));
            assert_eq!(v["mode"], "profile", "{flag} {q}");
            assert!(
                v["pmz"].as_array().unwrap().len() >= 3,
                "{flag} {q}: {desktop:.200}"
            );
            assert_eq!(
                cli.handle("/api/spectrum", &Query::parse(q))
                    .unwrap()
                    .unwrap(),
                desktop,
                "desktop vs cli {flag} {q}"
            );
            app.push(desktop);
        }
        let Ok(pkg) = std::env::var("MZLAB_WASM_PKG") else {
            continue;
        };
        let mut plan = vec![json!({"op": "idxopen", "slot": 0})];
        plan.extend(
            WINDOWS
                .iter()
                .map(|q| json!({"op": "route", "slot": 0, "path": "/api/spectrum", "query": q})),
        );
        let plan_file = dir.path().join("plan.json");
        std::fs::write(&plan_file, serde_json::to_string(&plan).unwrap()).unwrap();
        let out = Command::new("node")
            .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/rotte_wasm_node.js"))
            .args([Path::new(&pkg), &idx, &wiff, &plan_file])
            .output()
            .expect("node runs");
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let wasm: Vec<Value> = serde_json::from_slice(&out.stdout).unwrap();
        for (k, a) in app.iter().enumerate() {
            assert_eq!(
                wasm[k + 1]["s"].as_str(),
                Some(a.as_str()),
                "site vs desktop {flag} {}",
                WINDOWS[k]
            );
        }
    }
}
