//! `spectrum get --peak auto`: the document is the engine's `/api/specpeak` (its evidence byte for byte, its lines as the points), a known apex over a flat
//! background gives that apex and a clean spectrum, a flat trace gives a verdict and no point, and the exit codes of a bad request are pinned.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use mzlab_core::json::{parse, Value};
use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter};
use mzlab_core::routes::Query;

use crate::extract::tests::{dir, doc_points, page_routes, route_points, run};

const APEX: f64 = 4.0;
const SIGMA: f64 = 0.05;

/// 300 product-ion scans of precursor 250.3 every 0.02 min from 1.0: m/z 100.3 rises over a flat 20 cps (`peak`), m/z 150.3 is a flat 50 cps.
fn mzml(tag: &str, peak: bool) -> (std::path::PathBuf, String) {
    let d = dir(tag);
    let mut w = MzmlWriter::new(
        Vec::new(),
        &Header {
            run: "r".into(),
            ..Default::default()
        },
    )
    .unwrap();
    w.open_spectra(300).unwrap();
    for i in 0..300 {
        let rt = 1.0 + i as f64 * 0.02;
        let rise = if peak {
            (1000.0 * (-(rt - APEX).powi(2) / (2.0 * SIGMA * SIGMA)).exp()).round()
        } else {
            0.0
        };
        let s = Spectrum {
            index: i,
            id: format!("scan={}", i + 1),
            level: 2,
            rt,
            polarity: 1,
            precursor: Some(250.3),
            mz: vec![100.3, 150.3],
            intensity: vec![(20.0 + rise) as f32, 50.0],
            ..Default::default()
        };
        w.spectrum(&s).unwrap();
    }
    w.close_spectra().unwrap();
    let f = d.join("a.mzML");
    std::fs::write(&f, w.finish().unwrap()).unwrap();
    (d, f.to_string_lossy().into_owned())
}

fn ok(a: &[&str]) -> (Value, String, String) {
    let o = run(a);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let text = String::from_utf8(o.stdout).unwrap();
    (parse(text.as_bytes()).unwrap(), text, o.stderr)
}

const ARGS: [&str; 8] = [
    "--rt",
    "2,6",
    "--ms-level",
    "2",
    "--precursor",
    "250.3",
    "--peak",
    "auto",
];

#[test]
fn the_document_is_the_route_evidence_and_its_lines() {
    let (d, f) = mzml("peak_route", true);
    let mut a = vec!["spectrum", "get", &f];
    a.extend(ARGS);
    a.extend(["--fill-ms", "20"]);
    let (v, doc, err) = ok(&a);
    let text = mzlab_core::rawinfo::handle(
        &mut page_routes(&f),
        "/api/specpeak",
        &Query::parse("rt0=2&rt1=6&level=2&precursor=250.3&fill_ms=20"),
    )
    .unwrap()
    .unwrap();
    let cut = text.rfind(",\"mz\":[").unwrap();
    assert!(
        doc.contains(&format!("\"peak\":{}}}", &text[..cut])),
        "{doc}"
    );
    assert_eq!(doc_points(&doc), route_points(&text, "mz"));
    assert!(err.is_empty(), "{err}");
    let p = v.get("evidence").unwrap().get("peak").unwrap();
    assert_eq!(p.get("verdict").unwrap().str(), Some("peak"));
    assert!((p.get("apex").unwrap().get("rt_min").unwrap().f64().unwrap() - APEX).abs() < 0.03);
    assert_eq!(v.get("n_points").unwrap().f64(), Some(1.0));
    assert!(p.get("n_scans").unwrap().f64().unwrap() > 3.0);
    assert!(p.get("total_ions").unwrap().f64().unwrap() > 0.0);
    assert_eq!(
        p.get("sum")
            .unwrap()
            .get("background")
            .unwrap()
            .get("applied")
            .unwrap()
            .str(),
        Some("sides")
    );
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn set_changes_the_window_and_csv_is_the_lines() {
    let (d, f) = mzml("peak_set", true);
    let width = |extra: &[&str]| {
        let mut a = vec!["spectrum", "get", f.as_str()];
        a.extend(ARGS);
        a.extend(extra);
        let (v, _, _) = ok(&a);
        let w = v
            .get("evidence")
            .unwrap()
            .get("peak")
            .unwrap()
            .get("window_min")
            .unwrap()
            .arr()
            .to_vec();
        w[1].f64().unwrap() - w[0].f64().unwrap()
    };
    let (two, three) = (width(&[]), width(&["--set", "factor=3,side_min=0.2"]));
    assert!((three / two - 1.5).abs() < 0.01, "{two} {three}");
    let mut a = vec!["spectrum", "get", f.as_str()];
    a.extend(ARGS);
    a.extend(["--format", "csv"]);
    let o = run(&a);
    assert_eq!(o.code, 0);
    assert!(String::from_utf8(o.stdout)
        .unwrap()
        .starts_with("mz,intensity\n100.3,"));
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn a_flat_trace_gives_a_verdict_and_no_spectrum() {
    let (d, f) = mzml("peak_flat", false);
    let mut a = vec!["spectrum", "get", &f];
    a.extend(ARGS);
    let (v, _, err) = ok(&a);
    let p = v.get("evidence").unwrap().get("peak").unwrap();
    assert_eq!(p.get("verdict").unwrap().str(), Some("no_apex"));
    assert_eq!(v.get("n_points").unwrap().f64(), Some(0.0));
    assert!(err.contains("no_apex"), "{err}");
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn bad_requests_are_usage_errors_exit_2() {
    let (d, f) = mzml("peak_bad", true);
    let code = |a: &[&str]| {
        let mut args = vec!["spectrum", "get", f.as_str()];
        args.extend(a);
        run(&args).code
    };
    for a in [
        vec!["--rt", "4", "--ms-level", "2", "--peak", "auto"],
        vec!["--scan", "3", "--peak", "auto"],
        vec!["--rt", "2,6", "--ms-level", "2", "--peak", "now"],
        vec!["--rt", "2,6", "--ms-level", "2", "--peak", "auto", "--sum"],
        vec![
            "--rt",
            "2,6",
            "--ms-level",
            "2",
            "--peak",
            "auto",
            "--minus",
            &f,
        ],
        vec![
            "--rt",
            "2,6",
            "--ms-level",
            "2",
            "--peak",
            "auto",
            "--id",
            "ms:1",
        ],
        vec![
            "--rt",
            "2,6",
            "--ms-level",
            "2",
            "--peak",
            "auto",
            "--set",
            "nope=1",
        ],
        vec![
            "--rt",
            "2,6",
            "--ms-level",
            "2",
            "--peak",
            "auto",
            "--set",
            "factor=0",
        ],
        vec![
            "--rt",
            "2,6",
            "--ms-level",
            "2",
            "--peak",
            "auto",
            "--fill-ms",
            "0",
        ],
        vec![
            "--rt",
            "2,6",
            "--ms-level",
            "2",
            "--peak",
            "auto",
            "--set",
            "tol_ppm=0",
        ],
        vec!["--rt", "2,6", "--ms-level", "2", "--set", "factor=3"],
        vec!["--rt", "2,6", "--ms-level", "2", "--precursor", "250.3"],
    ] {
        assert_eq!(code(&a), 2, "{a:?}");
    }
    let _ = std::fs::remove_dir_all(d);
}

/// The same peak as `mzml` read as high resolution (an `FTMS` filter): the lines are matched in ppm, the cell and the tolerance are named, `--set tol_ppm` changes it.
#[test]
fn a_high_resolution_level_names_its_cell_and_takes_a_tolerance() {
    let d = dir("peak_hr");
    let mut w = MzmlWriter::new(Vec::new(), &Header { run: "r".into(), ..Default::default() }).unwrap();
    w.open_spectra(300).unwrap();
    for i in 0..300 {
        let rt = 1.0 + i as f64 * 0.02;
        let rise = (1000.0 * (-(rt - APEX).powi(2) / (2.0 * SIGMA * SIGMA)).exp()).round();
        let s = Spectrum {
            index: i,
            id: format!("scan={}", i + 1),
            level: 2,
            rt,
            polarity: 1,
            filter: "FTMS + c ESI Full ms2 250.3@hcd30.00 [50.00-500.00]".into(),
            precursor: Some(250.3),
            mz: vec![100.3, 150.3],
            intensity: vec![(20.0 + rise) as f32, 50.0],
            ..Default::default()
        };
        w.spectrum(&s).unwrap();
    }
    w.close_spectra().unwrap();
    let f = d.join("a.mzML");
    std::fs::write(&f, w.finish().unwrap()).unwrap();
    let f = f.to_string_lossy().into_owned();
    let peak = |extra: &[&str]| {
        let mut a = vec!["spectrum", "get", f.as_str()];
        a.extend(ARGS);
        a.extend(extra);
        let (v, _, _) = ok(&a);
        (v.get("evidence").unwrap().get("peak").unwrap().clone(), v.get("points").unwrap().arr().len())
    };
    let ((p, n), (q, _)) = (peak(&[]), peak(&["--set", "tol_ppm=10"]));
    let tol = |v: &Value| v.get("cell").unwrap().get("tol_ppm").unwrap().f64().unwrap();
    assert_eq!(p.get("verdict").unwrap().str(), Some("peak"));
    assert_eq!(p.get("cell").unwrap().get("kind").unwrap().str(), Some("hr_log"));
    assert_eq!((tol(&p), tol(&q), n), (5.0, 10.0, 1));
    let _ = std::fs::remove_dir_all(d);
}
