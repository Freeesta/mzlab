//! `raw info` and `raw cache` on a synthetic mzML: the document and its members (the same bytes as the engine route the site answers),
//! the plan of `--dry-run`, the exits of a missing, unreadable or badly asked file, and the cache entries `raw cache` builds and checks.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use super::super::extract::tests::{dir, mzml_file, page_routes, run};
use mzlab_core::json::parse;
use mzlab_core::routes::Query;

/// The missing-input cases of the `raw` commands, run by `contract_tests` (exit 4, one `error/1` under `--json`).
pub(crate) const MISSING: &[&[&str]] = &[
    &["raw", "info", "/nonexistent/x.mzML"],
    &[
        "raw",
        "cache",
        "/nonexistent/x.mzML",
        "--cache",
        "/nonexistent/cache",
    ],
    &[
        "raw",
        "xic",
        "/nonexistent/x.mzML",
        "--mz",
        "250",
        "--rt",
        "1,2",
    ],
    &["raw", "scan", "/nonexistent/x.mzML", "--rt", "1,2"],
];

fn json(o: &crate::Outcome) -> mzlab_core::json::Value {
    parse(&o.stdout).unwrap_or_else(|e| panic!("{e:?}: {}", String::from_utf8_lossy(&o.stdout)))
}

#[test]
fn raw_info_describes_the_file_with_the_members_of_the_engine_route() {
    let (d, f) = mzml_file("rawinfo");
    let o = run(&["raw", "info", &f]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let v = json(&o);
    assert_eq!(v.get("schema").unwrap().str(), Some("raw-info/1"));
    let file = &v.get("files").unwrap().arr()[0];
    assert_eq!(file.get("file").unwrap().str(), Some("a.mzML"));
    assert_eq!(file.get("sha256").unwrap().str().map(str::len), Some(64));
    let s = &file.get("samples").unwrap().arr()[0];
    for k in [
        "polarity",
        "rt",
        "n_scans",
        "ms_levels",
        "scan_types",
        "mass_range",
        "probe",
        "grid",
        "ion_quantum",
        "mass_offset",
        "spectrum_type",
    ] {
        assert!(s.get(k).is_some(), "{k}");
    }
    let p = v.get("provenance").unwrap();
    assert_eq!(p.get("program").unwrap().str(), Some("mzLab"));
    assert!(p.get("cite").unwrap().str().unwrap().contains("mzLab"));
    // the site answers `/api/rawinfo` on the index of the same bytes: its members are those of the CLI, byte for byte
    let route = page_routes(&f)
        .raw_info_route(&Query::parse("probe=64"))
        .unwrap()
        .unwrap();
    let out = String::from_utf8(o.stdout.clone()).unwrap();
    assert!(out.contains(&route[1..route.len() - 1]), "{route}\n{out}");
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn the_dry_run_plans_without_reading_and_bad_asks_have_their_exits() {
    let (d, f) = mzml_file("rawplan");
    let o = run(&[
        "raw",
        "info",
        &f,
        "--dry-run",
        "--probe",
        "8",
        "--offset-ion",
        "150.3@1.2",
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let v = json(&o);
    assert_eq!(v.get("schema").unwrap().str(), Some("raw-plan/1"));
    let q = v.get("query").unwrap();
    assert_eq!(q.get("probe").unwrap().f64(), Some(8.0));
    assert_eq!(
        q.get("offset_ion").unwrap().get("rt").unwrap().f64(),
        Some(1.2)
    );
    assert_eq!(
        v.get("files").unwrap().arr()[0].get("cache").unwrap().str(),
        Some("none")
    );
    assert_eq!(run(&["raw", "info", &format!("{f}.absent")]).code, 4);
    assert_eq!(run(&["raw", "info", &f, "--probe", "0"]).code, 2);
    assert_eq!(run(&["raw", "info", &f, "--offset-ion", "x"]).code, 2);
    assert_eq!(run(&["raw", "info"]).code, 2);
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn an_unreadable_file_keeps_the_document_with_its_error_and_exit_1() {
    let (d, f) = mzml_file("rawbad");
    let bad = d.join("b.raw");
    std::fs::write(&bad, b"\x00\x01garbage").unwrap();
    let o = run(&["raw", "info", &f, &bad.display().to_string()]);
    assert_eq!(o.code, 1, "{}", o.stderr);
    let v = json(&o);
    let files = v.get("files").unwrap().arr();
    assert!(files[0].get("samples").is_some());
    assert_eq!(
        files[1]
            .get("error")
            .unwrap()
            .get("error_key")
            .unwrap()
            .str(),
        Some("err.file.notThisFormat")
    );
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn raw_cache_builds_then_reuses_and_check_writes_nothing() {
    let (d, f) = mzml_file("rawcache");
    assert_eq!(run(&["raw", "cache", &f]).code, 2);
    let c = dir("rawcache_c").display().to_string();
    let status = |o: &crate::Outcome| {
        json(o).get("files").unwrap().arr()[0]
            .get("status")
            .unwrap()
            .str()
            .unwrap()
            .to_string()
    };
    let check = run(&["raw", "cache", &f, "--check", "--cache", &c]);
    // a verdict: the entry the next call would build is exit 1, with the document on stdout
    assert_eq!(
        (check.code, status(&check)),
        (1, "missing".to_string()),
        "{}",
        check.stderr
    );
    assert_eq!(std::fs::read_dir(&c).unwrap().count(), 0);
    let built = run(&["raw", "cache", &f, "--cache", &c]);
    assert_eq!(
        (built.code, status(&built)),
        (0, "built".to_string()),
        "{}",
        built.stderr
    );
    let v = json(&built);
    let e = &v.get("files").unwrap().arr()[0];
    assert!(e.get("bytes").unwrap().f64().unwrap() > 0.0);
    assert!(!e.get("sections").unwrap().arr().is_empty());
    assert_eq!(v.get("schema").unwrap().str(), Some("raw-cache/1"));
    assert_eq!(status(&run(&["raw", "cache", &f, "--cache", &c])), "reused");
    let again = run(&["raw", "cache", &f, "--check", "--cache", &c]);
    assert_eq!((again.code, status(&again)), (0, "reused".to_string()));
    // `raw info` prints the same bytes with and without the cache, and its plan sees the entry
    let cold = run(&["raw", "info", &f]);
    let warm = run(&["raw", "info", &f, "--cache", &c]);
    assert_eq!(cold.stdout, warm.stdout);
    let plan = json(&run(&["raw", "info", &f, "--dry-run", "--cache", &c]));
    assert_eq!(
        plan.get("files").unwrap().arr()[0]
            .get("cache")
            .unwrap()
            .str(),
        Some("reused")
    );
    let _ = std::fs::remove_dir_all(d);
    let _ = std::fs::remove_dir_all(c);
}

#[test]
fn a_measured_offset_says_which_flag_applies_it() {
    let (d, f) = mzml_file("rawoffhint");
    let o = run(&["raw", "info", &f, "--offset-ion", "241.3@3"]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert!(
        o.stderr.contains("--mz-offset auto:241.3@3"),
        "{}",
        o.stderr
    );
    // not measured: nothing to apply
    assert!(!run(&["raw", "info", &f]).stderr.contains("--mz-offset"));
    let _ = std::fs::remove_dir_all(d);
}
