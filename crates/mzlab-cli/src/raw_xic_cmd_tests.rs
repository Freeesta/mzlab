//! `raw xic` on a synthetic mzML: the rows and members are the engine route's bytes, the summary and the cap are the CLI's, the plan reads no
//! peak, and a badly asked or unreadable input has its exit code.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use super::super::extract::tests::{mzml_file, page_routes, run};
use mzlab_core::json::{parse, Value};
use mzlab_core::routes::Query;

fn json(o: &crate::Outcome) -> Value {
    parse(&o.stdout).unwrap_or_else(|e| panic!("{e:?}: {}", String::from_utf8_lossy(&o.stdout)))
}

#[test]
fn a_peak_and_an_empty_mass_get_their_verdicts_with_the_rule_and_the_summary() {
    let (d, f) = mzml_file("xicverdict");
    let o = run(&["raw", "xic", &f, "--mz", "241.3,300.7", "--rt", "2.5,3.5"]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let v = json(&o);
    assert_eq!(v.get("schema").unwrap().str(), Some("raw-xic/1"));
    let rows = v.get("rows").unwrap().arr();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].get("verdict").unwrap().str(), Some("peak"));
    assert_eq!(rows[0].get("file").unwrap().str(), Some("a.mzML"));
    assert!(
        rows[0]
            .get("rule")
            .unwrap()
            .str()
            .unwrap()
            .contains("pipeline S/N")
            && rows[0]
                .get("rule")
                .unwrap()
                .str()
                .unwrap()
                .ends_with("-> peak")
    );
    assert_eq!(rows[1].get("verdict").unwrap().str(), Some("nothing"));
    assert_eq!(rows[1].get("ul_counts"), Some(&Value::Null));
    // no ion quantum in this fixture: no counts, so no upper limit and no level for it (the values are pinned on the route's own test)
    assert_eq!(rows[1].get("ul_alpha"), Some(&Value::Null));
    assert_eq!(
        rows[0].get("flags").unwrap().arr()[0].str(),
        Some("no_quantum")
    );
    let s = v.get("summary").unwrap().arr();
    assert_eq!(s[0].get("peak").unwrap().f64(), Some(1.0));
    // the fixture holds no ion counts: the pipeline S/N decides and there is no floor to rank the files by
    assert_eq!(s[0].get("max_sn_floor"), Some(&Value::Null));
    assert!(s[0].get("median_area").unwrap().f64().unwrap() > 0.0);
    assert_eq!(s[1].get("nothing").unwrap().f64(), Some(1.0));
    let p = v.get("provenance").unwrap();
    assert_eq!(
        p.get("params")
            .unwrap()
            .get("set")
            .unwrap()
            .get("min_sn")
            .unwrap()
            .f64(),
        Some(3.0)
    );
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn the_members_are_the_bytes_of_the_route_the_site_answers() {
    let (d, f) = mzml_file("xicroute");
    let o = run(&[
        "raw", "xic", &f, "--mz", "241.3", "--rt", "2.5,3.5", "--da", "0.3",
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let out = String::from_utf8(o.stdout.clone()).unwrap();
    let route = page_routes(&f)
        .raw_xic_route(&Query::parse("mz=241.3&rt=2.5,3.5&da=0.3"))
        .unwrap()
        .unwrap();
    let members = &route[1..route.find(",\"rows\":[").unwrap()];
    assert!(out.contains(members), "{route}\n{out}");
    let row = &route[route.find("\"rows\":[").unwrap() + 9..route.len() - 2];
    assert!(out.contains(&row[1..]), "{row}\n{out}");
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn the_limit_cuts_the_rows_and_says_so_while_the_summary_stays_whole() {
    let (d, f) = mzml_file("xiclimit");
    let o = run(&[
        "raw",
        "xic",
        &f,
        "--mz",
        "241.3,257.3,300.7",
        "--rt",
        "2.5,3.5",
        "--limit",
        "2",
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let v = json(&o);
    assert_eq!(v.get("rows").unwrap().arr().len(), 2);
    assert_eq!(v.get("n_total").unwrap().f64(), Some(3.0));
    assert_eq!(v.get("truncated"), Some(&Value::Bool(true)));
    assert_eq!(v.get("summary").unwrap().arr().len(), 3);
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn the_dry_run_plans_without_reading_and_bad_asks_have_their_exits() {
    let (d, f) = mzml_file("xicplan");
    let o = run(&[
        "raw",
        "xic",
        &f,
        "--mz",
        "241.3",
        "--rt",
        "2.5,3.5",
        "--dry-run",
        "--set",
        "min_sn=5",
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let v = json(&o);
    assert_eq!(v.get("schema").unwrap().str(), Some("raw-plan/1"));
    assert_eq!(v.get("command").unwrap().str(), Some("raw xic"));
    assert_eq!(
        v.get("query")
            .unwrap()
            .get("set")
            .unwrap()
            .get("min_sn")
            .unwrap()
            .f64(),
        Some(5.0)
    );
    assert_eq!(
        run(&[
            "raw",
            "xic",
            &format!("{f}.absent"),
            "--mz",
            "1",
            "--rt",
            "1,2"
        ])
        .code,
        4
    );
    for bad in [
        vec!["--rt", "1,2"],
        vec!["--mz", "241.3"],
        vec!["--mz", "241.3", "--rt", "3,2"],
        vec!["--mz", "241.3", "--rt", "1,2", "--da", "0"],
        vec!["--mz", "241.3", "--rt", "1,2", "--set", "bogus=1"],
        vec!["--mz", "241.3", "--rt", "1,2", "--set", "alpha=0.9"],
        vec!["--mz", "241.3", "--rt", "1,2", "--limit", "0"],
        vec!["--mz", "241.3", "--rt", "1,2", "--polarity", "x"],
    ] {
        let mut a = vec!["raw", "xic", f.as_str()];
        a.extend(bad.iter().copied());
        assert_eq!(run(&a).code, 2, "{bad:?}");
    }
    assert_eq!(run(&["raw", "xic", "--mz", "1", "--rt", "1,2"]).code, 2);
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn an_unreadable_file_keeps_the_document_with_its_error_and_exit_1() {
    let (d, f) = mzml_file("xicbad");
    let bad = d.join("b.raw");
    std::fs::write(&bad, b"\x00\x01garbage").unwrap();
    let o = run(&[
        "raw",
        "xic",
        &f,
        &bad.display().to_string(),
        "--mz",
        "241.3",
        "--rt",
        "2.5,3.5",
    ]);
    assert_eq!(o.code, 1, "{}", o.stderr);
    let v = json(&o);
    let files = v.get("files").unwrap().arr();
    assert!(files[0].get("samples").is_some());
    assert!(files[1].get("error").is_some());
    assert_eq!(v.get("rows").unwrap().arr().len(), 1);
    let _ = std::fs::remove_dir_all(d);
}

/// Every usage error of this command is an `err.cli.rawxic.*` key (the text lives in `lang/en`), pinned with its exit code 2 and the `error/1` document.
#[test]
fn usage_errors_carry_their_keys() {
    let (d, f) = mzml_file("xickeys");
    let base = ["raw", "xic", f.as_str()];
    let many = (0..=mzlab_core::rawxic::MAX_MASSES)
        .map(|i| format!("{}", 100.0 + i as f64 * 0.01))
        .collect::<Vec<_>>()
        .join(",");
    for (rest, key) in [
        (vec!["--rt", "1,2"], "err.cli.rawxic.needsMz"),
        (vec!["--mz", "241.3"], "err.cli.rawxic.needsRt"),
        (
            vec!["--mz", "241.3", "--rt", "1,2", "--set", "bogus=1"],
            "err.cli.rawxic.set",
        ),
        (
            vec!["--mz", "241.3", "--rt", "1,2", "--limit", "0"],
            "err.cli.rawxic.limit",
        ),
        (
            vec!["--mz", "241.3", "--rt", "1,2", "--polarity", "x"],
            "err.cli.rawxic.polarity",
        ),
        (vec!["--mz", "241.3", "--rt", "2,1"], "err.raw.xic.param"),
        (
            vec!["--mz", many.as_str(), "--rt", "1,2"],
            "err.raw.xic.many",
        ),
    ] {
        let mut a = base.to_vec();
        a.extend(rest.iter().copied());
        let o = run(&a);
        assert_eq!(o.code, 2, "{key}");
        assert!(o.stderr.contains(key), "{key}: {}", o.stderr);
        assert_eq!(json(&o).get("error_key").unwrap().str(), Some(key));
    }
    let o = run(&["raw", "xic", "--mz", "1", "--rt", "1,2"]);
    assert_eq!(o.code, 2);
    assert_eq!(
        json(&o).get("error_key").unwrap().str(),
        Some("err.cli.rawxic.needsFile")
    );
    let _ = std::fs::remove_dir_all(d);
}

/// A row the summary cannot read, or a row count other than the masses', fails the file loud with `err.raw.xic.rows`: exit 1 (data), a
/// per-file error that does not end the whole run, and never the panic-only exit 7.
#[test]
fn inconsistent_rows_fail_the_file_loud_with_a_data_error() {
    let ok = vec![r#"{"verdict":"peak"}"#.to_string()];
    assert_eq!(super::check_rows(&ok, 1, "a.mzML").unwrap().len(), 1);
    for (rows, masses, what) in [
        (vec!["{not json".to_string()], 1, "unreadable row"),
        (ok.clone(), 2, "fewer rows than masses"),
        (vec![], 1, "no rows"),
    ] {
        let e = super::check_rows(&rows, masses, "a.mzML").unwrap_err();
        assert_eq!(e.key, "err.raw.xic.rows", "{what}");
        assert_eq!(crate::exit::code_of(&e), crate::exit::DATA, "{what}");
        assert!(!super::whole_run(&e), "{what}");
    }
}
