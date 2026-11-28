//! `rebin` with a blank (`bgk`) and with `mode=sum` of `/api/spectrum` (`spectrum get --bin --minus|--sum`): the app, the site's engine, the CLI and the
//! engine `.wasm` (`MZLAB_WASM_PKG`, run by `tests/test_parita_rotte_app.py`) give the same text, or the same refusal key.
mod nominal_common;
use mzlab_core::idxread::Index;
use mzlab_core::routes::{Query, Routes};
use nominal_common::{json_of, setup, text, Setup};
use serde_json::{json, Value};
use std::path::Path;
use std::process::Command;

/// `rebin` with a blank (`bgk`: each side cut into bins, then subtracted; here the blank is the first scan of the same file) and with `mode=sum`.
/// The last query is refused with the same key everywhere.
const BLANK_QUERIES: [(&str, &str); 4] = [
    (
        "specbg",
        "?k=0&rt0=2&rt1=3&level=1&bgk=0&bgrt0=0&bgrt1=0&rebin=nominal",
    ),
    (
        "specbg",
        "?k=0&rt0=2&rt1=3&level=1&bgk=0&bgrt0=0&bgrt1=0&rebin=nominal&mzoff=0.24",
    ),
    (
        "route",
        "?k=0&rt0=0&rt1=3&level=1&mode=sum&rebin=nominal&mzoff=0.24",
    ),
    (
        "specbg",
        "?k=0&rt0=2&rt1=3&level=1&bgk=0&bgrt0=0&bgrt1=0&rebin=7",
    ),
];

/// The three native engines answer each blank or sum query alike (`None` is a refusal with `err.spectrum.rebin`).
fn blank_native_answers(s: &Setup) -> Vec<Option<String>> {
    let idx_bytes = std::fs::read(&s.idx_path).unwrap();
    let len = idx_bytes.len() as u64;
    let mut site = Routes::new(Index::open(idx_bytes, len).unwrap());
    let mut cli = Routes::new(Index::open(std::fs::File::open(&s.idx_path).unwrap(), len).unwrap());
    fn diff<R: mzlab_core::idxread::ReadAt>(
        r: &mut Routes<R>,
        q: &str,
    ) -> mzlab_core::error::Result<Option<String>> {
        let req = r
            .diff_request(&Query::parse(q))?
            .expect("a background was asked");
        mzlab_core::specdiff::spectrum_diff(r, None, &req)
    }
    let mut answers = Vec::new();
    for (op, q) in BLANK_QUERIES {
        let app =
            s.st.rpc(
                op,
                &json!({"slot": 0, "path": "/api/spectrum", "query": q}),
                &[],
            )
            .map(text);
        let (a, b) = if op == "specbg" {
            (diff(&mut site, q), diff(&mut cli, q))
        } else {
            (
                site.handle("/api/spectrum", &Query::parse(q)),
                cli.handle("/api/spectrum", &Query::parse(q)),
            )
        };
        match (&app, &a, &b) {
            (Ok(x), Ok(Some(y)), Ok(Some(z))) => {
                assert_eq!((x, x), (y, z), "{q}");
                answers.push(Some(x.clone()));
            }
            (Err(_), Err(y), Err(z)) => {
                assert_eq!(
                    (y.key, z.key),
                    ("err.spectrum.rebin", "err.spectrum.rebin"),
                    "{q}"
                );
                answers.push(None);
            }
            _ => panic!(
                "{q}: app {:?}, site {:?}, cli {:?}",
                app.as_ref().err().map(|e| e.to_string()),
                a.as_ref().err().map(|e| e.key),
                b.as_ref().err().map(|e| e.key)
            ),
        }
    }
    answers
}

#[test]
fn bins_of_a_blank_and_of_a_sum_match_across_site_app_cli_and_wasm() {
    let s = setup();
    let answers = blank_native_answers(&s);
    let plain = json_of(&answers[0]);
    // the blank's scan is taken off bin by bin: the order and the blank's own offset are in the answer
    assert_eq!(plain["mode"], "bins");
    assert_eq!(plain["order"], "bin_then_subtract");
    assert_eq!(plain["blank_mz_offset_da"], 0.0);
    assert!(plain["bin"]
        .as_array()
        .unwrap()
        .iter()
        .all(|b| b.as_f64().unwrap() > 0.0));
    assert_eq!(json_of(&answers[1])["blank_mz_offset_da"], 0.24);
    let sum = json_of(&answers[2]);
    assert_eq!(sum["mode"], "bins");
    assert!(sum.get("order").is_none());
    // the CLI command holds the route's tokens
    let file = s.file.to_string_lossy().into_owned();
    let v: Vec<String> = [
        "spectrum",
        "get",
        &file,
        "--rt",
        "2,3",
        "--minus",
        &file,
        "--minus-rt",
        "0,0",
        "--bin",
        "nominal",
    ]
    .iter()
    .map(|x| x.to_string())
    .collect();
    let o = mzlab_cli::execute(&v);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let got: Value = serde_json::from_slice(&o.stdout).unwrap();
    for (i, b) in got["bins"].as_array().unwrap().iter().enumerate() {
        assert_eq!(
            (&b["bin"], &b["mz"], &b["counts"], &b["points"]),
            (
                &plain["bin"][i],
                &plain["mz"][i],
                &plain["y"][i],
                &plain["points"][i]
            )
        );
    }
    // the engine `.wasm` gives the same text (or a refusal with the same key) when `MZLAB_WASM_PKG` points at its package
    let Ok(pkg) = std::env::var("MZLAB_WASM_PKG") else {
        return;
    };
    let mut plan = vec![json!({"op": "idxopen", "slot": 0, "name": "split.mzML"})];
    plan.extend(
        BLANK_QUERIES
            .iter()
            .map(|(op, q)| json!({"op": op, "slot": 0, "path": "/api/spectrum", "query": q})),
    );
    let plan_file = s.dir.path().join("plan_blank.json");
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
    for (k, (_, q)) in BLANK_QUERIES.iter().enumerate() {
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
