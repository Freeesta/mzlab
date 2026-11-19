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
