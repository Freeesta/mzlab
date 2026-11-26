//! `trace xic --ions`: the ion scale of every trace is the bytes of the engine route `/api/xicions` (the site's and the app's), cut by the same `--rt`
//! window as the points; without `--ions` nothing changes; the choice is in the evidence and on stderr.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use mzlab_core::json::parse;
use mzlab_core::routes::Query;

use crate::extract::tests::{mzml_file, page_routes, run};
use crate::extract::tokens;

const SIM: [&str; 5] = ["--ions", "--scan-type", "sim", "--set", "dwell_ms=5"];

fn go(f: &str, mz: &str, extra: &[&str]) -> crate::Outcome {
    let mut a = vec!["trace", "xic", f, "--mz", mz, "--da", "0.4"];
    a.extend_from_slice(extra);
    run(&a)
}

fn out(o: &crate::Outcome) -> String {
    assert_eq!(o.code, 0, "{}", o.stderr);
    String::from_utf8(o.stdout.clone()).unwrap()
}

#[test]
fn the_ions_of_a_trace_are_the_route_bytes_next_to_the_points() {
    let (d, f) = mzml_file("tx_ions");
    let doc = out(&go(&f, "241.3", &SIM));
    let route = page_routes(&f)
        .xic_ions_route(&Query::parse(
            "mz=241.3&tol=0.4&ions=1&scan_type=sim&dwell_ms=5",
        ))
        .unwrap()
        .unwrap();
    let ions = tokens(&route, "ions").unwrap();
    assert!(!ions.is_empty());
    assert!(
        doc.contains(&format!("\"ions\":[{}]", ions.join(","))),
        "{route}\n{doc}"
    );
    // the factor and the log of the choice are the route's text, in the evidence
    let scale = &route
        [route.find("\"ion_scale\":").unwrap() + 12..route.find(",\"ion_scale_choice\"").unwrap()];
    assert!(
        doc.contains(&format!("\"ion_scale\":{scale}")),
        "{scale}\n{doc}"
    );
    let v = parse(doc.as_bytes()).unwrap();
    assert_eq!(v.get("schema").unwrap().str(), Some("trace/1"));
    assert_eq!(v.get("ions").unwrap().arr().len(), ions.len());
    let ev = v.get("evidence").unwrap();
    assert_eq!(
        ev.get("ion_scale")
            .unwrap()
            .get("ions_per_cps")
            .unwrap()
            .f64(),
        Some(0.005)
    );
    assert_eq!(
        ev.get("ion_scale_choice")
            .unwrap()
            .get("scan_type")
            .unwrap()
            .str(),
        Some("sim")
    );
    // the cps points are the same bytes as without the opt-in
    let plain = out(&go(&f, "241.3", &[]));
    assert_eq!(
        crate::extract::tests::doc_points(&doc),
        crate::extract::tests::doc_points(&plain)
    );
    assert!(
        !plain.contains("\"ions\"") && !plain.contains("ion_scale"),
        "{plain}"
    );
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn the_window_cuts_the_ions_with_the_points() {
    let (d, f) = mzml_file("tx_ions_rt");
    let all = parse(out(&go(&f, "241.3", &SIM)).as_bytes()).unwrap();
    let mut a: Vec<&str> = SIM.to_vec();
    a.extend(["--rt", "2.5,3.5"]);
    let cut = parse(out(&go(&f, "241.3", &a)).as_bytes()).unwrap();
    let n = cut.get("n_points").unwrap().f64().unwrap() as usize;
    assert!(n > 0 && n < all.get("n_points").unwrap().f64().unwrap() as usize);
    assert_eq!(cut.get("ions").unwrap().arr().len(), n);
    // the kept ions are the ones of the kept points
    let first = all
        .get("points")
        .unwrap()
        .arr()
        .iter()
        .position(|p| p.arr()[0].f64() == cut.get("points").unwrap().arr()[0].arr()[0].f64())
        .unwrap();
    assert_eq!(
        cut.get("ions").unwrap().arr()[0].f64(),
        all.get("ions").unwrap().arr()[first].f64()
    );
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn csv_has_the_ions_column_and_the_wide_csv_one_per_ion() {
    let (d, f) = mzml_file("tx_ions_csv");
    let mut a: Vec<&str> = SIM.to_vec();
    a.extend(["--format", "csv"]);
    let one = out(&go(&f, "241.3", &a));
    let mut lines = one.lines();
    assert_eq!(lines.next(), Some("rt_min,intensity,ions"));
    assert!(lines.all(|l| l.split(',').count() == 3), "{one}");
    let wide = out(&go(&f, "241.3,300", &a));
    assert_eq!(
        wide.lines().next(),
        Some("rt_min,xic:241.3,ions:241.3,xic:300,ions:300")
    );
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn a_list_answers_every_ion_and_logs_the_factor() {
    let (d, f) = mzml_file("tx_ions_set");
    let o = go(&f, "241.3,300", &SIM);
    let set = parse(out(&o).as_bytes()).unwrap();
    let route = page_routes(&f)
        .xic_ions_route(&Query::parse(
            "mz=241.3,300&tol=0.4,0.4&ions=1&scan_type=sim&dwell_ms=5",
        ))
        .unwrap()
        .unwrap();
    for (t, want) in set
        .get("traces")
        .unwrap()
        .arr()
        .iter()
        .zip(route.split(",{\"mz\":"))
    {
        assert_eq!(
            t.get("ions").unwrap().arr().len(),
            tokens(want, "ions").unwrap().len()
        );
    }
    assert!(
        o.stderr
            .contains("note: ion scale of the sim scans: 0.005 ions per cps (method: dwell_ms)"),
        "{}",
        o.stderr
    );
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn without_a_factor_the_traces_stay_in_cps_and_the_choice_says_why() {
    let (d, f) = mzml_file("tx_ions_none");
    let o = go(&f, "241.3", &["--ions"]);
    let doc = out(&o);
    let v = parse(doc.as_bytes()).unwrap();
    assert!(v.get("ions").is_none());
    let ev = v.get("evidence").unwrap();
    assert!(
        ev.get("ion_scale").is_some()
            && ev
                .get("ion_scale_choice")
                .unwrap()
                .get("evidence")
                .unwrap()
                .str()
                .unwrap()
                .starts_with("none")
    );
    assert!(o.stderr.contains("note: no ion scale"), "{}", o.stderr);
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn usage_errors_and_the_plan() {
    let (d, f) = mzml_file("tx_ions_usage");
    for extra in [&["--set", "fill_ms=3"][..], &["--scan-type", "ems"]] {
        let o = go(&f, "241.3", extra);
        assert_eq!(o.code, 2);
        assert!(
            String::from_utf8_lossy(&o.stdout).contains("err.cli.tracexic.needsIons"),
            "{}",
            o.stderr
        );
    }
    for extra in [
        &["--ions", "--scan-type", "x"][..],
        &["--ions", "--set", "gain=3"],
        &["--ions", "--set", "fill_ms=0"],
    ] {
        assert_eq!(go(&f, "241.3", extra).code, 2, "{extra:?}");
    }
    let mut a: Vec<&str> = SIM.to_vec();
    a.push("--dry-run");
    let plan = parse(out(&go(&f, "241.3", &a)).as_bytes()).unwrap();
    let i = plan.get("ions").unwrap();
    assert_eq!(i.get("scan_type").unwrap().str(), Some("sim"));
    assert_eq!(i.get("dwell_ms").unwrap().f64(), Some(5.0));
    assert!(parse(out(&go(&f, "241.3", &["--dry-run"])).as_bytes())
        .unwrap()
        .get("ions")
        .is_none());
    let _ = std::fs::remove_dir_all(d);
}
