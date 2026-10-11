//! The `mzlab://` protocol on an example mzML against the golden data of the Python engine (`tests/golden/`): the bytes the window
//! gets must hold the same TIC and XIC (RT to 4 decimals, intensities to 1) and the file chunks must be the file.

use std::path::{Path, PathBuf};

use mzlab_desktop::{handle, State};
use serde::Deserialize;

#[derive(Deserialize)]
struct Trace {
    rt: Vec<f64>,
    y: Vec<f64>,
}
#[derive(Deserialize)]
struct GXic {
    mz_lo: f64,
    mz_hi: f64,
    #[serde(flatten)]
    t: Trace,
}
#[derive(Deserialize)]
struct Golden {
    tic: Trace,
    xic: Vec<GXic>,
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn floats(b: &[u8]) -> Vec<f64> {
    b.chunks_exact(8)
        .map(|c| f64::from_le_bytes(c.try_into().unwrap()))
        .collect()
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9_f64.max(1e-4 * a.abs().max(b.abs()))
}

fn check(what: &str, body: &[u8], g: &Trace) {
    let v = floats(body);
    let n = g.rt.len();
    assert_eq!(v.len(), 2 * n, "{what}: number of points");
    for i in 0..n {
        assert!((v[i] - g.rt[i]).abs() <= 2e-4, "{what}: rt[{i}]");
        assert!(
            close(v[n + i], g.y[i]),
            "{what}: y[{i}] {} vs {}",
            v[n + i],
            g.y[i]
        );
    }
}

#[test]
fn tic_xic_chunks_and_errors() {
    let file = root().join("mzlab/web/esempi/FullScan_t10.mzML");
    if !file.exists() {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "example file missing"
        );
        return;
    }
    let g: Golden = serde_json::from_reader(
        std::fs::File::open(root().join("tests/golden/esempio_FullScan_t10.json")).unwrap(),
    )
    .unwrap();
    let st = State::new();
    let (id, p) = st.pick(file.clone()).unwrap();
    assert_eq!(p.name, "FullScan_t10.mzML");

    let r = handle(&st, "/open", &format!("slot=0&id={id}"));
    assert_eq!(r.status, 200, "{}", String::from_utf8_lossy(&r.body));
    let info = String::from_utf8(r.body).unwrap();
    assert!(
        info.contains("\"unit\":true") && info.contains("\"levels\":{\"1\":"),
        "{info}"
    );

    let r = handle(&st, "/tic", "slot=0&level=1&px=0");
    assert_eq!(
        (r.status, r.content_type),
        (200, "application/octet-stream")
    );
    check("tic", &r.body, &g.tic);

    for x in &g.xic {
        let (mz, tol) = ((x.mz_lo + x.mz_hi) / 2.0, (x.mz_hi - x.mz_lo) / 2.0);
        let r = handle(
            &st,
            "/xic",
            &format!("slot=0&level=1&mz={mz}&tol={tol}&px=0"),
        );
        check(&format!("xic {mz}"), &r.body, &x.t);
    }

    // a scan: [n_total, count, sid, rt, k, mz(k), y(k)]
    let r = handle(&st, "/spectra", "slot=0&level=1&i0=0&i1=0&bin=0.1");
    let v = floats(&r.body);
    let k = v[4] as usize;
    assert!(v[0] >= 1.0 && v[1] == 1.0 && k > 0 && v.len() == 5 + 2 * k);
    assert!(handle(
        &st,
        "/spectra",
        "slot=0&level=1&i0=999999&i1=999999&bin=0.1"
    )
    .body
    .is_empty());

    // the file in chunks is the file
    let whole = std::fs::read(&file).unwrap();
    let mut back = Vec::new();
    let mut off = 0usize;
    while off < whole.len() {
        let r = handle(&st, "/file", &format!("id={id}&off={off}&len=100000"));
        assert_eq!(r.status, 200);
        back.extend_from_slice(&r.body);
        off += 100_000;
    }
    assert!(back == whole, "chunks differ from the file");

    // the page knows files by name and size
    let r = handle(
        &st,
        "/lookup",
        &format!("name=FullScan_t10.mzML&size={}", whole.len()),
    );
    assert!(String::from_utf8(r.body)
        .unwrap()
        .contains(&format!("\"id\":{id}")));
    assert_eq!(handle(&st, "/lookup", "name=x.mzML&size=1").status, 400);

    // errors are JSON with a key
    let r = handle(&st, "/open", "slot=1&id=999");
    assert_eq!(r.status, 400);
    assert!(String::from_utf8(r.body).unwrap().contains("\"error_key\""));
    assert_eq!(handle(&st, "/tic", "slot=7&level=1").status, 400);
    assert_eq!(handle(&st, "/nope", "").status, 404);

    handle(&st, "/close", "slot=0");
    assert_eq!(handle(&st, "/tic", "slot=0&level=1").status, 400);
}

#[test]
fn rawprogress_of_a_file_that_never_converted_is_empty() {
    let st = State::new();
    let r = handle(&st, "/rawprogress", "id=7");
    assert_eq!(r.status, 200);
    assert_eq!(String::from_utf8(r.body).unwrap(), "{}");
}
