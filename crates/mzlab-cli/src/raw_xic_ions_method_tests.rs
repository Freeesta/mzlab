//! `raw xic --ions` reads the dwell of the file's own method: the value fills what `--set` left out, a `--set` value wins, differing dwells and a file
//! with no method give the data estimate, every case is a line of `provenance.choices`, and the bytes are those of the route given the same values.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use super::super::extract::tests::{dir, page_routes, run};
use mzlab_core::json::{parse, Value};
use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter};
use mzlab_core::routes::Query;

/// 200 SIM scans (one ion is 200 cps = 5 ms) with three ions at m/z 250.3 around 2.7 min; `method` is the `method/1` text in the `<run>`.
fn sim_file(tag: &str, method: Option<&str>) -> (std::path::PathBuf, String) {
    let mut h = Header {
        run: "r".into(),
        ..Default::default()
    };
    if let Some(m) = method {
        h.run_params
            .push((mzlab_core::method::USERPARAM.into(), m.into()));
    }
    let mut w = MzmlWriter::new(Vec::new(), &h).unwrap();
    w.open_spectra(200).unwrap();
    for i in 0..200 {
        let ions = if (85..90).contains(&i) { 3.0 } else { 0.0 };
        w.spectrum(&Spectrum {
            index: i,
            id: format!("scan={}", i + 1),
            level: 1,
            rt: 1.0 + i as f64 * 0.02,
            polarity: 1,
            filter: "+ SIM ms [250.000-251.000]".into(),
            mz: vec![150.3, 250.3],
            intensity: vec![200.0, (ions * 200.0) as f32],
            ..Default::default()
        })
        .unwrap();
    }
    w.close_spectra().unwrap();
    let d = dir(tag);
    let f = d.join("a.mzML");
    std::fs::write(&f, w.finish().unwrap()).unwrap();
    (d, f.to_string_lossy().into_owned())
}

fn method(dwells: &[f64]) -> String {
    let exps: Vec<String> = dwells
        .iter()
        .enumerate()
        .map(|(i, d)| {
            format!(r#"{{"index":{i},"type":"MRM","transitions":[{{"q1":250.3,"q3":100,"dwell_ms":{d}}}]}}"#)
        })
        .collect();
    format!(
        r#"{{"schema":"method/1","ms":{{"periods":[{{"experiments":[{}]}}]}}}}"#,
        exps.join(",")
    )
}

const Q: &str = "mz=250.3&rt=2.5,3.0&da=0.3";

fn go(f: &str, extra: &[&str]) -> Value {
    let mut a = vec![
        "raw", "xic", f, "--mz", "250.3", "--rt", "2.5,3.0", "--da", "0.3", "--ions",
    ];
    a.extend_from_slice(extra);
    let o = run(&a);
    assert_eq!(o.code, 0, "{}", o.stderr);
    parse(&o.stdout).unwrap()
}

fn scale(v: &Value) -> &Value {
    v.get("files").unwrap().arr()[0]
        .get("samples")
        .unwrap()
        .arr()[0]
        .get("ion_scale")
        .unwrap()
}

fn choice<'a>(v: &'a Value, what: &str) -> &'a Value {
    v.get("provenance")
        .unwrap()
        .get("choices")
        .unwrap()
        .arr()
        .iter()
        .find(|c| c.get("what").unwrap().str().unwrap().starts_with(what))
        .unwrap_or_else(|| panic!("no choice {what}"))
}

#[test]
fn the_dwell_of_the_method_is_read_logged_and_is_the_routes_bytes() {
    let (d, f) = sim_file("ionmeth", Some(&method(&[5.0, 5.0])));
    let o = run(&[
        "raw", "xic", &f, "--mz", "250.3", "--rt", "2.5,3.0", "--da", "0.3", "--ions",
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let out = String::from_utf8(o.stdout.clone()).unwrap();
    let v = parse(out.as_bytes()).unwrap();
    let s = scale(&v);
    assert_eq!(s.get("source").unwrap().str(), Some("method"));
    assert_eq!(s.get("field").unwrap().str(), Some("dwell_ms"));
    assert_eq!(s.get("ions_per_cps").unwrap().f64(), Some(0.005));
    let c = choice(&v, "ion_method");
    assert!(c.get("what").unwrap().str().unwrap().ends_with("dwell_ms"));
    assert_eq!(c.get("value").unwrap().f64(), Some(5.0));
    assert!(c
        .get("why")
        .unwrap()
        .str()
        .unwrap()
        .contains("ms.periods[0].experiments[0]"));
    // the route given the values the shared function names answers the same bytes
    let facts = mzlab_core::ionfacts::read_json(&method(&[5.0, 5.0]), "").unwrap();
    let q = parse(facts.as_bytes()).unwrap();
    let query = format!("{Q}&ions=1&{}", q.get("query").unwrap().str().unwrap());
    let route = page_routes(&f)
        .raw_xic_route(&Query::parse(&query))
        .unwrap()
        .unwrap();
    let members = &route[1..route.find(",\"rows\":[").unwrap()];
    assert!(out.contains(members), "{route}\n{out}");
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn a_set_value_wins_and_the_choice_says_so() {
    let (d, f) = sim_file("ionmethset", Some(&method(&[5.0])));
    let v = go(&f, &["--set", "dwell_ms=10"]);
    assert_eq!(scale(&v).get("ions_per_cps").unwrap().f64(), Some(0.01));
    let c = choice(&v, "ion_method");
    assert_eq!(c.get("value").unwrap().f64(), Some(10.0));
    assert!(c.get("why").unwrap().str().unwrap().contains("--set wins"));
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn differing_dwells_and_a_method_without_one_give_the_data_estimate_and_say_why() {
    let (d, f) = sim_file("ionmethdiff", Some(&method(&[5.0, 8.0])));
    let v = go(&f, &[]);
    assert_eq!(scale(&v).get("source").unwrap().str(), Some("data"));
    let c = choice(&v, "ion_method");
    assert_eq!(c.get("value"), Some(&Value::Null));
    assert!(c.get("why").unwrap().str().unwrap().contains("differs"));
    let (d2, g) = sim_file("ionmethnone", None); // the mzML header gives a method/1 with no dwell
    let v = go(&g, &[]);
    assert_eq!(scale(&v).get("source").unwrap().str(), Some("data"));
    assert!(choice(&v, "ion_method")
        .get("why")
        .unwrap()
        .str()
        .unwrap()
        .contains("names no value"));
    let _ = (std::fs::remove_dir_all(d), std::fs::remove_dir_all(d2));
}

#[test]
fn without_the_opt_in_the_method_is_not_read() {
    let (d, f) = sim_file("ionmethoff", Some(&method(&[5.0])));
    let o = run(&[
        "raw", "xic", &f, "--mz", "250.3", "--rt", "2.5,3.0", "--da", "0.3",
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let v = parse(&o.stdout).unwrap();
    assert!(v
        .get("provenance")
        .unwrap()
        .get("choices")
        .unwrap()
        .arr()
        .iter()
        .all(|c| !c
            .get("what")
            .unwrap()
            .str()
            .unwrap()
            .starts_with("ion_method")));
    assert!(scale_absent(&v));
    let _ = std::fs::remove_dir_all(d);
}

fn scale_absent(v: &Value) -> bool {
    v.get("files").unwrap().arr()[0]
        .get("samples")
        .unwrap()
        .arr()[0]
        .get("ion_scale")
        .is_none()
}
