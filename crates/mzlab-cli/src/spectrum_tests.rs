//! `mzlab-cli spectrum get|list`: the points and rows are the number tokens of `api/spectrum`, `api/nearest_scan` and `api/scanlist` on the same bytes.

use mzlab_core::json::parse;
use mzlab_core::routes::Query;

use crate::extract::tests::{
    dir, doc_points, mzml_file, page_routes, route_points, run, synthetic,
};
use crate::extract::tokens;

fn text(o: crate::Outcome) -> String {
    assert_eq!(o.code, 0, "{}", o.stderr);
    String::from_utf8(o.stdout).unwrap()
}

#[test]
fn a_window_is_the_page_spectrum_route() {
    let (d, f) = mzml_file("spec_win");
    let doc = text(run(&["spectrum", "get", &f, "--rt", "2.9,3.1"]));
    assert!(doc.starts_with("{\"schema\":\"spectrum/1\","), "{doc}");
    let want = page_routes(&f)
        .handle("/api/spectrum", &Query::parse("rt0=2.9&rt1=3.1&level=1"))
        .unwrap()
        .unwrap();
    assert_eq!(doc_points(&doc), route_points(&want, "mz"));
    let v = parse(doc.trim().as_bytes()).unwrap();
    assert_eq!(v.get("x_unit").unwrap().str(), Some("m/z"));
    let ans = v.get("evidence").unwrap().get("answer").unwrap();
    assert_eq!(ans.get("scans").unwrap().f64(), Some(5.0), "{doc}");
    assert!(
        doc_points(&doc)
            .iter()
            .any(|(mz, _)| mz.starts_with("241.")),
        "{doc}"
    );
    let csv = text(run(&[
        "spectrum", "get", &f, "--rt", "2.9,3.1", "--format", "csv",
    ]));
    assert!(csv.starts_with("mz,intensity\n") && csv.lines().count() == doc_points(&doc).len() + 1);
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn a_time_and_a_scan_number_pick_the_same_scan() {
    let (d, f) = mzml_file("spec_scan");
    let by_rt = text(run(&["spectrum", "get", &f, "--rt", "3.01"]));
    let by_no = text(run(&["spectrum", "get", &f, "--scan", "51"]));
    assert_eq!(doc_points(&by_rt), doc_points(&by_no));
    for doc in [&by_rt, &by_no] {
        assert!(
            doc.contains("\"scan\":{\"scan_id\":50,\"scan_no\":51,\"rt_min\":3,\"ms_level\":1}"),
            "{doc}"
        );
    }
    let want = page_routes(&f)
        .handle("/api/spectrum", &Query::parse("rt0=3&rt1=3&level=1"))
        .unwrap()
        .unwrap();
    assert_eq!(doc_points(&by_no), route_points(&want, "mz"));
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn an_unknown_scan_is_exit_5_and_bad_requests_exit_2() {
    let (d, f) = mzml_file("spec_err");
    let o = run(&["spectrum", "get", &f, "--scan", "9999", "--json"]);
    assert_eq!(o.code, 5);
    assert!(
        o.stderr.contains("Scan 9999 does not exist"),
        "{}",
        o.stderr
    );
    assert_eq!(
        parse(&o.stdout).unwrap().get("schema").unwrap().str(),
        Some("error/1")
    );
    for a in [
        vec!["spectrum", "get", &f],
        vec!["spectrum", "get", &f, "--rt", "1", "--scan", "2"],
        vec!["spectrum", "get", &f, "--rt", "x"],
        vec!["spectrum", "get", &f, "--scan", "x"],
        vec!["spectrum", "get", &f, "--rt", "1", "--ms-level", "0"],
        vec!["spectrum", "list", &f, "--ms-level", "x"],
        vec!["spectrum", "list"],
    ] {
        assert_eq!(run(&a).code, 2, "{a:?}");
    }
    assert_eq!(
        run(&["spectrum", "get", &f, "--rt", "1", "--ms-level", "2"]).code,
        5
    );
    assert_eq!(run(&["spectrum", "get", "--help"]).code, 0);
    assert_eq!(run(&["spectrum", "list", "--help"]).code, 0);
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn the_scan_list_is_the_page_route() {
    let (d, f) = mzml_file("spec_list");
    let doc = text(run(&["spectrum", "list", &f]));
    assert!(doc.starts_with("{\"schema\":\"scanlist/1\","), "{doc}");
    let v = parse(doc.trim().as_bytes()).unwrap();
    assert_eq!(v.get("n_scans").unwrap().f64(), Some(120.0));
    let want = page_routes(&f)
        .handle("/api/scanlist", &Query::parse("level=1"))
        .unwrap()
        .unwrap();
    let (rt, tic) = (tokens(&want, "rt").unwrap(), tokens(&want, "tic").unwrap());
    let rows = v.get("scans").unwrap().arr();
    assert_eq!(rows.len(), 120);
    // the CSV keeps the route's own tokens, so compare the text of a row
    let csv = text(run(&["spectrum", "list", &f, "--format", "csv"]));
    let mut lines = csv.lines();
    assert_eq!(
        lines.next(),
        Some("scan_id,scan_no,ms_level,rt_min,tic,precursor_mz,base_peak_mz,base_peak_intensity")
    );
    let first: Vec<&str> = lines.next().unwrap().split(',').collect();
    assert_eq!(
        (first[0], first[1], first[2], first[3], first[4], first[5]),
        ("0", "1", "1", rt[0], tic[0], "")
    );
    assert_eq!(csv.lines().count(), 121);
    assert_eq!(
        rows[0].get("precursor_mz"),
        Some(&mzlab_core::json::Value::Null)
    );
    let none = text(run(&["spectrum", "list", &f, "--ms-level", "2"]));
    assert!(none.contains("\"n_scans\":0"), "{none}");
    let _ = std::fs::remove_dir_all(d);
}

/// A DDA-like file: the MS2 scans list their precursors, and `--scan` of one gives its fragments with the precursor of the scan.
#[test]
fn an_ms2_scan_is_read_with_its_own_level_and_precursor() {
    let Some(d) = synthetic() else { return };
    let p = d.join("B_MS2.mzML").to_string_lossy().into_owned();
    let list = text(run(&["spectrum", "list", &p, "--ms-level", "2"]));
    let v = parse(list.trim().as_bytes()).unwrap();
    // the scan with the most signal: the first scans of a run can be empty
    let first = v
        .get("scans")
        .unwrap()
        .arr()
        .iter()
        .max_by(|a, b| {
            a.get("tic")
                .unwrap()
                .f64()
                .unwrap()
                .total_cmp(&b.get("tic").unwrap().f64().unwrap())
        })
        .unwrap();
    assert_eq!(first.get("ms_level").unwrap().f64(), Some(2.0));
    assert!(first.get("precursor_mz").unwrap().f64().is_some(), "{list}");
    let no = first.get("scan_no").unwrap().f64().unwrap() as i64;
    let doc = text(run(&["spectrum", "get", &p, "--scan", &no.to_string()]));
    assert!(
        doc.contains("\"ms_level\":2") && doc.contains("precursor="),
        "{doc}"
    );
    assert!(!doc_points(&doc).is_empty(), "{doc}");
    // all levels together are in time order
    let all = text(run(&["spectrum", "list", &p]));
    let rts: Vec<f64> = parse(all.trim().as_bytes())
        .unwrap()
        .get("scans")
        .unwrap()
        .arr()
        .iter()
        .map(|r| r.get("rt_min").unwrap().f64().unwrap())
        .collect();
    assert!(rts.windows(2).all(|w| w[0] <= w[1]));
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn a_file_without_scans_lists_none() {
    let d = dir("spec_mrm");
    let f = d.join("m.mzML");
    std::fs::write(
        &f,
        "<mzML><run><chromatogramList count=\"0\"></chromatogramList></run></mzML>",
    )
    .unwrap();
    let doc = text(run(&["spectrum", "list", &f.to_string_lossy()]));
    assert!(doc.contains("\"n_scans\":0"), "{doc}");
    let _ = std::fs::remove_dir_all(d);
}

/// A sample and a blank written in a new folder: the sample has the ion at 257.3 growing with its time, the blank none of it.
fn sample_and_blank(tag: &str) -> (std::path::PathBuf, String, String) {
    let d = dir(tag);
    let (f, b) = (d.join("sample.mzML"), d.join("blank.mzML"));
    std::fs::write(&f, crate::tests::mzml(10.0)).unwrap();
    std::fs::write(&b, crate::tests::mzml(0.0)).unwrap();
    let name = |p: &std::path::Path| p.to_string_lossy().into_owned();
    (d.clone(), name(&f), name(&b))
}

#[test]
fn a_blank_is_the_engine_spectrum_with_a_background_byte_for_byte() {
    let (d, f, b) = sample_and_blank("spec_minus");
    let doc = text(run(&[
        "spectrum", "get", &f, "--rt", "2.9,3.1", "--minus", &b,
    ]));
    let (mut main, mut bg) = (page_routes(&f), page_routes(&b));
    let q = Query::parse("rt0=2.9&rt1=3.1&level=1&bgk=1&bgrt0=2.9&bgrt1=3.1");
    let req = main.diff_request(&q).unwrap().unwrap();
    let want = mzlab_core::specdiff::spectrum_diff(&mut main, Some(&mut bg), &req)
        .unwrap()
        .unwrap();
    assert_eq!(doc_points(&doc), route_points(&want, "mz"));
    // the ion both files hold at 3.0 min is gone, the one only the sample holds later is left
    assert!(
        !doc_points(&doc)
            .iter()
            .any(|(mz, _)| mz.starts_with("241.")),
        "{doc}"
    );
    let later = text(run(&[
        "spectrum", "get", &f, "--rt", "4.1,4.3", "--minus", &b,
    ]));
    assert!(
        doc_points(&later)
            .iter()
            .any(|(mz, _)| mz.starts_with("257.")),
        "{later}"
    );
    // every choice is in the evidence: the file, the window, the factor and the clip
    let v = parse(doc.trim().as_bytes()).unwrap();
    let ev = v.get("evidence").unwrap();
    let m = ev.get("minus").unwrap();
    assert_eq!(m.get("file").unwrap().str(), Some("blank.mzML"));
    assert_eq!(m.get("window_min").unwrap().arr().len(), 2);
    assert_eq!(m.get("factor").unwrap().f64(), Some(1.0));
    assert!(doc.contains("\"diff\":true"), "{doc}");
    let csv = text(run(&[
        "spectrum", "get", &f, "--rt", "2.9,3.1", "--minus", &b, "--format", "csv",
    ]));
    assert_eq!(csv.lines().count(), doc_points(&doc).len() + 1);
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn the_blank_window_is_given_or_taken_from_the_main_one_or_the_nearest_scan() {
    let (d, f, b) = sample_and_blank("spec_minus_win");
    // a window of the sample's own file as the blank (3.0 min holds 241.3): the later spectrum keeps 257.3 and, clipped, loses nothing else
    let own = text(run(&[
        "spectrum",
        "get",
        &f,
        "--rt",
        "4.1,4.3",
        "--minus",
        &f,
        "--minus-rt",
        "2.9,3.1",
    ]));
    let v = parse(own.trim().as_bytes()).unwrap();
    let w = v.get("evidence").unwrap().get("minus").unwrap();
    assert_eq!(w.get("window_min").unwrap().arr()[0].f64(), Some(2.9));
    assert!(w.get("scan").unwrap().f64().is_none() && own.contains("\"scan\":null"));
    // a single time: the blank's scan nearest to it is used and named
    let one = text(run(&["spectrum", "get", &f, "--rt", "3.01", "--minus", &b]));
    assert!(one.contains("\"minus\":{\"file\":\"blank.mzML\""), "{one}");
    assert!(one.contains("\"scan\":{\"scan_no\":"), "{one}");
    let by_no = text(run(&["spectrum", "get", &f, "--scan", "51", "--minus", &b]));
    assert_eq!(doc_points(&one), doc_points(&by_no));
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn a_bad_blank_is_usage_or_a_missing_file_with_an_error_document() {
    let (d, f, b) = sample_and_blank("spec_minus_bad");
    for a in [
        vec![
            "spectrum", "get", &f, "--rt", "3", "--minus", &b, "--id", "tic",
        ],
        vec!["spectrum", "get", &f, "--rt", "3", "--minus-rt", "1,2"],
        vec!["spectrum", "get", &f, "--rt", "3", "--minus-sample", "2"],
        vec![
            "spectrum",
            "get",
            &f,
            "--rt",
            "3",
            "--minus",
            &b,
            "--minus-rt",
            "x",
        ],
    ] {
        assert_eq!(run(&a).code, 2, "{a:?}");
    }
    let o = run(&[
        "spectrum",
        "get",
        &f,
        "--rt",
        "3",
        "--minus",
        "/nonexistent/b.mzML",
        "--json",
    ]);
    assert_eq!(o.code, 4, "{}", o.stderr);
    assert!(String::from_utf8(o.stdout)
        .unwrap()
        .contains("\"schema\":\"error/1\""));
    let _ = std::fs::remove_dir_all(d);
}

/// A profile mzML of MS1 scans only (no MS2 level to take a mean of).
fn profile_ms1(tag: &str) -> (std::path::PathBuf, String) {
    use mzlab_core::mzmlwrite::{Header, MzmlWriter};
    let d = dir(tag);
    let mut w = MzmlWriter::new(Vec::new(), &Header::default()).unwrap();
    w.open_spectra(4).unwrap();
    for k in 0..4 {
        let (mz, y): (Vec<f64>, Vec<f32>) = (0..200)
            .map(|i| {
                let m = 100.0 + f64::from(i) * 0.01;
                (
                    m,
                    (1000.0 * (-((m - 101.0) / 0.2).powi(2)).exp() + 5.0) as f32,
                )
            })
            .unzip();
        w.spectrum(&mzlab_core::mzml::Spectrum {
            index: k,
            id: format!("scan={}", k + 1),
            level: 1,
            rt: k as f64,
            polarity: 1,
            profile: true,
            mz,
            intensity: y,
            ..Default::default()
        })
        .unwrap();
    }
    w.close_spectra().unwrap();
    let f = d.join("p.mzML");
    std::fs::write(&f, w.finish().unwrap()).unwrap();
    (d, f.to_string_lossy().into_owned())
}

#[test]
fn a_blank_with_no_spectrum_to_subtract_from_is_exit_1_with_its_key() {
    let (d, f) = profile_ms1("spec_minus_none");
    let o = run(&[
        "spectrum",
        "get",
        &f,
        "--rt",
        "0,2",
        "--ms-level",
        "2",
        "--minus",
        &f,
        "--minus-rt",
        "0,2",
        "--json",
    ]);
    assert_eq!(o.code, 1, "{}", o.stderr);
    let e = String::from_utf8(o.stdout).unwrap();
    assert!(e.contains("\"schema\":\"error/1\""), "{e}");
    assert!(e.contains("err.spectrum.minus"), "{e}");
    let _ = std::fs::remove_dir_all(d);
}
