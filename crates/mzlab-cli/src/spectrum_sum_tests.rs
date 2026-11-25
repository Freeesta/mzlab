//! `spectrum get --sum`: the summed spectrum is the engine's `mode=sum`, its evidence is the bytes of `/api/specsum`, the exit codes of a bad request are
//! pinned, and the mean mode points at the sum (the hint) while its points stay what they were.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use mzlab_core::json::{parse, Value};
use mzlab_core::routes::Query;

use crate::extract::tests::{doc_points, mzml_file, page_routes, route_points, run, synthetic};

fn ok(a: &[&str]) -> (Value, String, String) {
    let o = run(a);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let text = String::from_utf8(o.stdout).unwrap();
    (parse(text.as_bytes()).unwrap(), text, o.stderr)
}

#[test]
fn the_sum_is_the_route_mode_sum_and_its_evidence_the_route_bytes() {
    let (d, f) = mzml_file("sum_route");
    let (v, doc, err) = ok(&["spectrum", "get", &f, "--rt", "2.9,3.1", "--sum"]);
    let mut r = page_routes(&f);
    let want = r
        .handle(
            "/api/spectrum",
            &Query::parse("rt0=2.9&rt1=3.1&level=1&mode=sum"),
        )
        .unwrap()
        .unwrap();
    assert_eq!(doc_points(&doc), route_points(&want, "mz"));
    let ev = mzlab_core::rawinfo::handle(
        &mut r,
        "/api/specsum",
        &Query::parse("rt0=2.9&rt1=3.1&level=1"),
    )
    .unwrap()
    .unwrap();
    // the evidence is the route's text, byte for byte
    assert!(doc.contains(&format!("\"sum\":{ev}}}")), "{doc}");
    let e = v.get("evidence").unwrap();
    assert_eq!(
        e.get("answer").unwrap().get("scans").unwrap().f64(),
        e.get("sum").unwrap().get("n_scans").unwrap().f64()
    );
    assert_eq!(
        e.get("sum").unwrap().get("n_scans").unwrap().f64(),
        Some(5.0)
    );
    assert_eq!(
        e.get("sum")
            .unwrap()
            .get("background")
            .unwrap()
            .get("applied")
            .unwrap()
            .str(),
        Some("none")
    );
    assert!(e.get("query").unwrap().str().unwrap().ends_with("mode=sum"));
    // no fill time in the file: ions null, and the terminal says why
    assert_eq!(e.get("sum").unwrap().get("ions"), Some(&Value::Null));
    assert!(err.contains("does not give the fill time"), "{err}");
    // a fill time the user states gives the ions
    let (v, _, _) = ok(&[
        "spectrum",
        "get",
        &f,
        "--rt",
        "2.9,3.1",
        "--sum",
        "--fill-ms",
        "20",
    ]);
    let s = v.get("evidence").unwrap().get("sum").unwrap();
    let c = s.get("counts_cps").unwrap().f64().unwrap();
    assert!(
        (s.get("ions").unwrap().f64().unwrap() - c * 0.02).abs() <= 0.1,
        "{s:?}"
    );
    assert_eq!(
        s.get("fill_time").unwrap().get("source").unwrap().str(),
        Some("user")
    );
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn the_mean_mode_keeps_its_points_and_points_at_the_sum() {
    let (d, f) = mzml_file("sum_hint");
    let (v, doc, _) = ok(&["spectrum", "get", &f, "--rt", "2.9,3.1"]);
    let want = page_routes(&f)
        .handle("/api/spectrum", &Query::parse("rt0=2.9&rt1=3.1&level=1"))
        .unwrap()
        .unwrap();
    assert_eq!(doc_points(&doc), route_points(&want, "mz"));
    let h = v.get("evidence").unwrap().get("hint").unwrap();
    assert_eq!(h.get("key").unwrap().str(), Some("note.spec.mean.sumHint"));
    assert!(h.get("command").unwrap().str().unwrap().ends_with("--sum"));
    // csv has no evidence: the hint goes to the terminal
    let o = run(&["spectrum", "get", &f, "--rt", "2.9,3.1", "--format", "csv"]);
    assert!(o.stderr.contains("--sum adds them"), "{}", o.stderr);
    // one scan, and the sum itself, have nothing to point at
    for a in [vec!["--rt", "3.01"], vec!["--rt", "2.9,3.1", "--sum"]] {
        let mut args = vec!["spectrum", "get", f.as_str()];
        args.extend(a);
        let (v, _, _) = ok(&args);
        assert!(v.get("evidence").unwrap().get("hint").is_none());
    }
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn bad_sum_requests_are_usage_errors_exit_2() {
    let (d, f) = mzml_file("sum_err");
    for a in [
        vec!["--rt", "3.0", "--sum"],
        vec!["--scan", "51", "--sum"],
        vec!["--rt", "2.9,3.1", "--sum", "--minus", f.as_str()],
        vec!["--rt", "2.9,3.1", "--fill-ms", "20"],
        vec!["--rt", "2.9,3.1", "--sum", "--fill-ms", "x"],
        vec!["--rt", "2.9,3.1", "--sum", "--fill-ms", "0"],
    ] {
        let mut args = vec!["spectrum", "get", f.as_str()];
        args.extend(a.iter().copied());
        let o = run(&args);
        assert_eq!(o.code, 2, "{a:?}: {}", o.stderr);
    }
    // the refusal of the route has its key and text
    let o = run(&[
        "spectrum",
        "get",
        &f,
        "--rt",
        "2.9,3.1",
        "--sum",
        "--fill-ms",
        "0",
        "--json",
    ]);
    assert_eq!(
        parse(&o.stdout).unwrap().get("error_key").unwrap().str(),
        Some("err.spec.fill")
    );
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn the_sum_can_be_found_from_the_help_the_task_words_and_the_catalog() {
    assert!(
        super::HELP_GET.contains("--sum")
            && super::HELP_GET.contains("spectrum get f.mzML --rt 6,7 --ms-level 2 --sum")
    );
    let o = run(&[
        "commands",
        "--find",
        "sum the scans of an MS2 window and count the ions",
        "--json",
    ]);
    let text = String::from_utf8(o.stdout).unwrap();
    let v = parse(text.as_bytes()).unwrap();
    let first: Vec<&str> = v
        .get("matches")
        .unwrap()
        .arr()
        .iter()
        .take(3)
        .filter_map(|m| m.get("name").and_then(Value::str))
        .collect();
    assert!(first.contains(&"spectrum get"), "{first:?}");
}

#[test]
fn a_channel_is_not_summed_and_its_mean_carries_no_hint_at_the_sum() {
    let Some(d) = synthetic() else { return };
    let p = d.join("C_IDA.mzML").to_string_lossy().into_owned();
    let get = [
        "spectrum",
        "get",
        &p,
        "--rt",
        "0,20",
        "--id",
        "ms:ms2 364",
        "--ms-level",
        "2",
    ];
    let o = run(&[&get[..], &["--sum"]].concat());
    assert_eq!(o.code, 2, "{}", o.stderr);
    assert!(o.stderr.contains("--sum is not for --id"), "{}", o.stderr);
    // the mean of the channel's window is as before, with no pointer to a command that would refuse it
    let o = run(&get);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert!(!String::from_utf8_lossy(&o.stdout).contains("\"hint\""));
}
