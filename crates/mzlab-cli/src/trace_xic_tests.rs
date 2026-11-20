//! `trace xic` with a list of ions, `--smooth` and `--dry-run`: every trace of a list is byte for byte what the same ion gives alone, which is the
//! page's `api/xic`; the wide CSV and the plan follow the schemas of `docs/schemi/`.

use mzlab_core::json::parse;

use crate::extract::tests::{doc_points, mzml_file, page_routes, route_points, run};

fn text(o: crate::Outcome) -> String {
    assert_eq!(o.code, 0, "{}", o.stderr);
    String::from_utf8(o.stdout).unwrap()
}

#[test]
fn a_list_is_the_traces_of_each_ion_in_order() {
    let (d, f) = mzml_file("xic_list");
    let set = text(run(&[
        "trace",
        "xic",
        &f,
        "--mz",
        "241.3,300,500.5",
        "--da",
        "0.4",
    ]));
    let v = parse(set.trim().as_bytes()).unwrap();
    assert_eq!(v.get("schema").unwrap().str(), Some("trace-set/1"));
    assert_eq!(v.get("n_traces").unwrap().f64(), Some(3.0));
    let mut r = page_routes(&f);
    for mz in ["241.3", "300", "500.5"] {
        let one = text(run(&["trace", "xic", &f, "--mz", mz, "--da", "0.4"]));
        // the member of the list is the document of the ion alone
        assert!(set.contains(one.trim()), "{mz}: {set}");
        assert_eq!(
            doc_points(&one),
            route_points(&r.xic(mz.parse().unwrap(), 0.4, 1).unwrap(), "rt"),
            "{mz}"
        );
    }
    let ids: Vec<_> = v
        .get("traces")
        .unwrap()
        .arr()
        .iter()
        .map(|t| t.get("id").unwrap().str().unwrap().to_string())
        .collect();
    assert_eq!(ids, ["xic:241.3", "xic:300", "xic:500.5"]);
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn the_wide_csv_has_one_column_per_ion_on_the_shared_scans() {
    let (d, f) = mzml_file("xic_wide");
    let csv = text(run(&[
        "trace",
        "xic",
        &f,
        "--mz",
        "241.3,300",
        "--ppm",
        "300",
        "--format",
        "csv",
    ]));
    let mut lines = csv.lines();
    assert_eq!(lines.next(), Some("rt_min,xic:241.3,xic:300"));
    let rows: Vec<Vec<&str>> = lines.map(|l| l.split(',').collect()).collect();
    assert_eq!(rows.len(), 120);
    let mut r = page_routes(&f);
    for (col, mz) in [(1, 241.3), (2, 300.0)] {
        let want = route_points(&r.xic(mz, mz * 300.0 * 1e-6, 1).unwrap(), "rt");
        let got: Vec<(String, String)> = rows
            .iter()
            .map(|c| (c[0].to_string(), c[col].to_string()))
            .collect();
        assert_eq!(got, want, "{mz}");
    }
    // one ion keeps the two-column CSV the `peak` commands read
    let one = text(run(&[
        "trace", "xic", &f, "--mz", "241.3", "--ppm", "300", "--format", "csv",
    ]));
    assert_eq!(one.lines().next(), Some("rt_min,intensity"));
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn tol_with_a_unit_is_ppm_or_da() {
    let (d, f) = mzml_file("xic_tol");
    let by = |a: &[&str]| {
        let mut v = vec!["trace", "xic", &f, "--mz", "241.3"];
        v.extend_from_slice(a);
        text(run(&v))
    };
    assert_eq!(by(&["--tol", "300ppm"]), by(&["--ppm", "300"]));
    assert_eq!(by(&["--tol", "0.35da"]), by(&["--da", "0.35"]));
    assert_eq!(by(&["--tol", "0.35DA"]), by(&["--da", "0.35"]));
    for bad in [
        vec!["--tol", "5"],
        vec!["--tol", "ppm"],
        vec!["--tol", "0ppm"],
        vec!["--tol", "5ppm", "--da", "0.1"],
        vec!["--ppm", "5", "--da", "0.1"],
        vec!["--mz", "300,300", "--da", "0.1"],
        vec!["--da", "0.1", "--smooth", "4"],
        vec!["--da", "0.1", "--mz", "241.3,x"],
        vec!["--da", "0.1", "--mz", "241.3,"],
    ] {
        let mut v = vec!["trace", "xic", &f, "--mz", "241.3"];
        v.extend_from_slice(&bad);
        assert_eq!(run(&v).code, 2, "{bad:?}");
    }
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn smoothing_is_the_engine_filter_and_is_named_in_the_evidence() {
    let (d, f) = mzml_file("xic_smooth");
    let plain = text(run(&["trace", "xic", &f, "--mz", "241.3", "--da", "0.35"]));
    let raw: Vec<f64> = doc_points(&plain)
        .iter()
        .map(|p| p.1.parse().unwrap())
        .collect();
    for k in [3usize, 5] {
        let doc = text(run(&[
            "trace",
            "xic",
            &f,
            "--mz",
            "241.3",
            "--da",
            "0.35",
            "--smooth",
            &k.to_string(),
        ]));
        let want = mzlab_core::peak1d::smooth(&raw, k);
        let got: Vec<f64> = doc_points(&doc)
            .iter()
            .map(|p| p.1.parse().unwrap())
            .collect();
        assert_eq!(got.len(), want.len());
        for (g, w) in got.iter().zip(&want) {
            assert!((g - w).abs() <= 0.051, "{g} {w}");
        }
        assert!(doc.contains(&format!("\"smooth\":{k}")), "{doc}");
        assert_ne!(got, raw);
        // the retention times are the route's own
        let x = |d: &str| -> Vec<String> { doc_points(d).into_iter().map(|p| p.0).collect() };
        assert_eq!(x(&doc), x(&plain));
    }
    assert!(!plain.contains("smooth"), "{plain}");
    // the window is cut after smoothing: the points inside are those of the whole smoothed trace
    let whole = text(run(&[
        "trace", "xic", &f, "--mz", "241.3", "--da", "0.35", "--smooth", "3",
    ]));
    let cut = text(run(&[
        "trace", "xic", &f, "--mz", "241.3", "--da", "0.35", "--smooth", "3", "--rt", "1,3",
    ]));
    let inside: Vec<_> = doc_points(&whole)
        .into_iter()
        .filter(|p| (1.0..=3.0).contains(&p.0.parse::<f64>().unwrap()))
        .collect();
    assert_eq!(doc_points(&cut), inside);
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn a_dry_run_names_the_extraction_and_writes_nothing() {
    let (d, f) = mzml_file("xic_dry");
    let out = d.join("never.json");
    let plan = text(run(&[
        "trace",
        "xic",
        &f,
        "--mz",
        "241.3,300",
        "--ppm",
        "10",
        "--smooth",
        "5",
        "--rt",
        "1,3",
        "--polarity",
        "+",
        "--out",
        &out.to_string_lossy(),
        "--dry-run",
    ]));
    assert_eq!(plan, "", "the plan goes where the data would go");
    assert!(out.exists());
    let plan = std::fs::read_to_string(&out).unwrap();
    let v = parse(plan.trim().as_bytes()).unwrap();
    assert_eq!(v.get("schema").unwrap().str(), Some("xicplan/1"));
    assert_eq!(v.get("smooth").unwrap().f64(), Some(5.0));
    assert_eq!(v.get("polarity").unwrap().str(), Some("+"));
    assert_eq!(v.get("n_traces").unwrap().f64(), Some(2.0));
    let t = &v.get("traces").unwrap().arr()[1];
    assert_eq!(t.get("id").unwrap().str(), Some("xic:300"));
    assert!((t.get("tolerance_da").unwrap().f64().unwrap() - 0.003).abs() < 1e-12);
    // to the standard output when no --out, with no index built (a text that is no mzML is still planned)
    let junk = d.join("junk.mzML");
    std::fs::write(&junk, b"not an mzML").unwrap();
    let shown = text(run(&[
        "trace",
        "xic",
        &junk.to_string_lossy(),
        "--mz",
        "100",
        "--da",
        "0.2",
        "--dry-run",
    ]));
    assert!(shown.starts_with("{\"schema\":\"xicplan/1\""), "{shown}");
    assert!(shown.contains("\"tolerance_ppm\":null"));
    // a missing file is still exit 4, like the run
    let gone = d.join("gone.mzML");
    let o = run(&[
        "trace",
        "xic",
        &gone.to_string_lossy(),
        "--mz",
        "100",
        "--da",
        "0.2",
        "--dry-run",
    ]);
    assert_eq!(o.code, 4, "{}", o.stderr);
    let _ = std::fs::remove_dir_all(d);
}
