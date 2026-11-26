//! `raw xic --ions`: the opt-in ion scale in the CLI document is the engine route's bytes, the choice is logged, and a method value without the opt-in is refused.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use super::super::extract::tests::{mzml_file, page_routes, run};
use mzlab_core::json::{parse, Value};
use mzlab_core::routes::Query;

const BASE: [&str; 8] = [
    "raw", "xic", "FILE", "--mz", "241.3", "--rt", "2.5,3.5", "--da",
];

fn args(f: &str, extra: &[&str]) -> Vec<String> {
    let mut v: Vec<String> = BASE.iter().map(|s| s.to_string()).collect();
    v[2] = f.to_string();
    v.push("0.3".into());
    v.extend(extra.iter().map(|s| s.to_string()));
    v
}

fn go(f: &str, extra: &[&str]) -> crate::Outcome {
    let a = args(f, extra);
    run(&a.iter().map(String::as_str).collect::<Vec<_>>())
}

#[test]
fn the_ion_scale_is_the_route_bytes_and_the_choice_is_logged() {
    let (d, f) = mzml_file("xicions");
    let o = go(&f, &["--ions", "--scan-type", "sim", "--set", "dwell_ms=5"]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let out = String::from_utf8(o.stdout.clone()).unwrap();
    let route = page_routes(&f)
        .raw_xic_route(&Query::parse(
            "mz=241.3&rt=2.5,3.5&da=0.3&ions=1&scan_type=sim&dwell_ms=5",
        ))
        .unwrap()
        .unwrap();
    let members = &route[1..route.find(",\"rows\":[").unwrap()];
    assert!(out.contains(members), "{route}\n{out}");
    let row = &route[route.find("\"rows\":[").unwrap() + 9..route.len() - 2];
    assert!(out.contains(row), "{row}\n{out}");
    let v = parse(out.as_bytes()).unwrap();
    let q = v.get("query").unwrap().get("ions").unwrap();
    assert_eq!(q.get("scan_type").unwrap().str(), Some("sim"));
    assert_eq!(q.get("dwell_ms").unwrap().f64(), Some(5.0));
    let c = v.get("provenance").unwrap().get("choices").unwrap().arr();
    let c = c
        .iter()
        .find(|c| {
            c.get("what")
                .unwrap()
                .str()
                .unwrap()
                .starts_with("ion_scale")
        })
        .unwrap();
    assert_eq!(c.get("value").unwrap().f64(), Some(0.005));
    assert!(c.get("why").unwrap().str().unwrap().contains("method"));
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn without_the_opt_in_nothing_is_added_and_a_method_value_is_refused() {
    let (d, f) = mzml_file("xicionsoff");
    let off = json(&go(&f, &[]));
    assert!(off.get("query").unwrap().get("ions").is_none());
    assert!(off.get("rows").unwrap().arr()[0].get("ions_net").is_none());
    for extra in [&["--set", "fill_ms=3"][..], &["--scan-type", "ems"]] {
        let o = go(&f, extra);
        assert_eq!(o.code, 2);
        assert!(String::from_utf8_lossy(&o.stdout).contains("err.cli.rawxic.needsIons"));
    }
    // the file names no scan type and the method gives no value: the cps columns stand and the choice says why
    let v = json(&go(&f, &["--ions"]));
    assert_eq!(v.get("rows").unwrap().arr()[0].get("ions_net"), None);
    let s = v.get("files").unwrap().arr()[0]
        .get("samples")
        .unwrap()
        .arr()[0]
        .get("ion_scale")
        .unwrap();
    assert_eq!(s, &Value::Null);
    let _ = std::fs::remove_dir_all(d);
}

fn json(o: &crate::Outcome) -> Value {
    parse(&o.stdout).unwrap()
}
