//! `raw scan` on a synthetic mzML: the members, summary and rows are the engine route's bytes, the cap and `--full` are the CLI's, the plan reads no
//! peak, and a badly asked or unreadable input has its exit code.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use super::super::extract::tests::{mzml_file, page_routes, run};
use mzlab_core::json::{parse, Value};
use mzlab_core::routes::Query;

fn json(o: &crate::Outcome) -> Value {
    parse(&o.stdout).unwrap_or_else(|e| panic!("{e:?}: {}", String::from_utf8_lossy(&o.stdout)))
}

const WIN: &str = "2.9,3.1";

#[test]
fn the_peak_of_the_window_is_the_first_row_and_the_summary_says_what_was_read() {
    let (d, f) = mzml_file("scanrows");
    let o = run(&["raw", "scan", &f, "--rt", WIN]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let v = json(&o);
    assert_eq!(v.get("schema").unwrap().str(), Some("raw-scan/1"));
    let rows = v.get("rows").unwrap().arr();
    assert_eq!(rows[0].get("file").unwrap().str(), Some("a.mzML"));
    assert_eq!(rows[0].get("mz_center").unwrap().f64(), Some(241.0));
    assert!(rows[0].get("net").unwrap().f64().unwrap() > 1e5);
    // the fixture holds no ion counts: the means stand, and the counts and the verdict are null
    assert_eq!(rows[0].get("verdict"), Some(&Value::Null));
    assert!(v
        .get("warnings")
        .unwrap()
        .arr()
        .iter()
        .any(|w| w.get("key").unwrap().str() == Some("warn.raw.noQuantum")));
    let s = &v.get("summary").unwrap().arr()[0];
    assert_eq!(s.get("n_scans").unwrap().f64(), Some(5.0));
    assert!(s.get("n_side_scans").unwrap().f64().unwrap() > 0.0);
    let p = v.get("provenance").unwrap().get("params").unwrap();
    assert_eq!(p.get("bin").unwrap().str(), Some("nominal"));
    assert_eq!(p.get("top").unwrap().f64(), Some(20.0));
    assert_eq!(p.get("set").unwrap().get("min_z").unwrap().f64(), Some(3.0));
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn the_members_summary_and_rows_are_the_bytes_of_the_route_the_site_answers() {
    let (d, f) = mzml_file("scanroute");
    let o = run(&[
        "raw", "scan", &f, "--rt", WIN, "--bin", "0.1", "--top", "3", "--side", "0.2",
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let out = String::from_utf8(o.stdout.clone()).unwrap();
    let route = page_routes(&f)
        .raw_scan_route(&Query::parse("rt=2.9,3.1&bin=0.1&top=3&side_min=0.2"))
        .unwrap()
        .unwrap();
    let members = &route[1..route.find(",\"summary\":{").unwrap()];
    assert!(out.contains(members), "{route}\n{out}");
    let summary =
        &route[route.find("\"summary\":{").unwrap() + 11..route.find(",\"rows\":[").unwrap()];
    assert!(out.contains(&summary[1..]), "{summary}\n{out}");
    let rows = &route[route.find("\"rows\":[").unwrap() + 9..route.len() - 2];
    for r in rows.split("},{") {
        let r = r.trim_start_matches('{').trim_end_matches('}');
        assert!(out.contains(r), "{r}\n{out}");
    }
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn top_and_limit_cut_the_rows_and_full_writes_every_bin_to_a_csv() {
    let (d, f) = mzml_file("scanfull");
    let two = json(&run(&["raw", "scan", &f, "--rt", WIN, "--top", "2"]));
    assert_eq!(two.get("rows").unwrap().arr().len(), 2);
    let lim = json(&run(&[
        "raw", "scan", &f, "--rt", WIN, "--top", "0", "--limit", "3",
    ]));
    assert_eq!(lim.get("rows").unwrap().arr().len(), 3);
    assert_eq!(lim.get("truncated"), Some(&Value::Bool(true)));
    let n_total = lim.get("n_total").unwrap().f64().unwrap();
    assert!(n_total > 3.0, "{n_total}");
    let dir = d.join("csv");
    let o = run(&[
        "raw",
        "scan",
        &f,
        "--rt",
        WIN,
        "--full",
        &dir.display().to_string(),
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let v = json(&o);
    assert_eq!(v.get("rows").unwrap().arr().len(), 0);
    let full = &v.get("full").unwrap().arr()[0];
    assert_eq!(full.get("rows").unwrap().f64(), Some(n_total));
    let csv = std::fs::read_to_string(dir.join("raw-scan.csv")).unwrap();
    assert!(csv.starts_with("file,sample,bin,mz_center,mean,background,net,ratio,background_zero,counts_net,z_counts,verdict\n"));
    assert_eq!(csv.lines().count() as f64, n_total + 1.0);
    let sha = full.get("sha256").unwrap().str().unwrap();
    assert_eq!(sha, mzlab_core::budget::sha256_hex(csv.as_bytes()));
    // a folder that cannot be made is exit 6
    let blocked = d.join("a.mzML").join("csv");
    assert_eq!(
        run(&[
            "raw",
            "scan",
            &f,
            "--rt",
            WIN,
            "--full",
            &blocked.display().to_string()
        ])
        .code,
        6
    );
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn a_window_at_the_end_of_the_run_warns_that_it_has_one_side() {
    let (d, f) = mzml_file("scanside");
    let v = json(&run(&["raw", "scan", &f, "--rt", "6.1,6.6"]));
    let w = v.get("warnings").unwrap().arr();
    let one = w
        .iter()
        .find(|w| w.get("key").unwrap().str() == Some("warn.raw.oneSide"))
        .unwrap();
    assert_eq!(
        one.get("params").unwrap().get("side").unwrap().str(),
        Some("left")
    );
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn the_dry_run_plans_without_reading_and_bad_asks_have_their_exits() {
    let (d, f) = mzml_file("scanplan");
    let o = run(&[
        "raw",
        "scan",
        &f,
        "--rt",
        WIN,
        "--dry-run",
        "--set",
        "min_z=5",
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let v = json(&o);
    assert_eq!(v.get("schema").unwrap().str(), Some("raw-plan/1"));
    assert_eq!(v.get("command").unwrap().str(), Some("raw scan"));
    let q = v.get("query").unwrap();
    assert_eq!(q.get("set").unwrap().get("min_z").unwrap().f64(), Some(5.0));
    assert_eq!(
        run(&["raw", "scan", &format!("{f}.absent"), "--rt", WIN]).code,
        4
    );
    assert_eq!(
        run(&["raw", "scan", &f, "--rt", WIN, "--channel", "ms:none"]).code,
        5
    );
    for bad in [
        vec![],
        vec!["--rt", "3,2"],
        vec!["--rt", WIN, "--bin", "0.5"],
        vec!["--rt", WIN, "--set", "bogus=1"],
        vec!["--rt", WIN, "--set", "alpha=0.9"],
        vec!["--rt", WIN, "--top", "-1"],
        vec!["--rt", WIN, "--limit", "0"],
        vec!["--rt", WIN, "--polarity", "x"],
        vec!["--rt", WIN, "--mz-range", "300"],
        vec!["--rt", WIN, "--channel", "tic"],
    ] {
        let mut a = vec!["raw", "scan", f.as_str()];
        a.extend(bad.iter().copied());
        assert_eq!(run(&a).code, 2, "{bad:?}");
    }
    assert_eq!(run(&["raw", "scan", "--rt", WIN]).code, 2);
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn an_unreadable_file_keeps_the_document_with_its_error_and_exit_1() {
    let (d, f) = mzml_file("scanbad");
    let bad = d.join("b.raw");
    std::fs::write(&bad, b"\x00\x01garbage").unwrap();
    let o = run(&["raw", "scan", &f, &bad.display().to_string(), "--rt", WIN]);
    assert_eq!(o.code, 1, "{}", o.stderr);
    let v = json(&o);
    let files = v.get("files").unwrap().arr();
    assert!(files[0].get("samples").is_some());
    assert!(files[1].get("error").is_some());
    assert_eq!(v.get("summary").unwrap().arr().len(), 1);
    let _ = std::fs::remove_dir_all(d);
}
