//! `raw isotopes` on a synthetic mzML of known ion counts: the members and the result are the engine route's bytes, the plan reads no peak, the formula
//! and the carbon range answer as the study says, and a badly asked or unreadable input has its exit code.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use super::super::extract::tests::{dir, page_routes, run};
use mzlab_core::json::{parse, Value};
use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter};
use mzlab_core::routes::Query;

fn json(o: &crate::Outcome) -> Value {
    parse(&o.stdout).unwrap_or_else(|e| panic!("{e:?}: {}", String::from_utf8_lossy(&o.stdout)))
}

/// 200 scans every 0.02 min from 1.0 min; from 2.0 to 2.4 min `m0`, `m1` and `m2` ions (200 cps each) per scan at m/z 181.07, 182.07 and 183.07.
fn file(tag: &str, m0: f32, m1: f32, m2: f32) -> (std::path::PathBuf, String) {
    let d = dir(tag);
    let mut w = MzmlWriter::new(
        Vec::new(),
        &Header {
            run: "r".into(),
            ..Default::default()
        },
    )
    .unwrap();
    w.open_spectra(200).unwrap();
    for i in 0..200 {
        let on = if (50..71).contains(&i) { 200.0 } else { 0.0 };
        let s = Spectrum {
            index: i,
            id: format!("scan={}", i + 1),
            level: 1,
            rt: 1.0 + i as f64 * 0.02,
            polarity: 1,
            mz: vec![181.07, 182.07, 183.07, 300.0],
            intensity: vec![on * m0, on * m1, on * m2, 200.0],
            ..Default::default()
        };
        w.spectrum(&s).unwrap();
    }
    w.close_spectra().unwrap();
    let f = d.join("a.mzML");
    std::fs::write(&f, w.finish().unwrap()).unwrap();
    (d, f.to_string_lossy().into_owned())
}

const WIN: &str = "2.0,2.4";

#[test]
fn the_ratios_of_known_counts_and_the_verdicts_of_a_formula() {
    let (d, f) = file("isoratio", 100.0, 7.0, 1.0);
    let o = run(&[
        "raw",
        "isotopes",
        &f,
        "--mz",
        "181.07",
        "--rt",
        WIN,
        "--formula",
        "C6H12O6",
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let v = json(&o);
    assert_eq!(v.get("schema").unwrap().str(), Some("raw-isotopes/1"));
    let r = &v.get("results").unwrap().arr()[0];
    assert_eq!(r.get("file").unwrap().str(), Some("a.mzML"));
    let ions = r.get("ions").unwrap().arr();
    assert_eq!(ions[0].get("net").unwrap().f64(), Some(2100.0));
    assert_eq!(ions[1].get("net").unwrap().f64(), Some(147.0));
    let r1 = r
        .get("observed")
        .unwrap()
        .get("r1")
        .unwrap()
        .get("r")
        .unwrap()
        .f64()
        .unwrap();
    assert!((r1 - 0.07).abs() < 1e-9);
    let t = r.get("tests").unwrap();
    assert_eq!(
        t.get("r1").unwrap().get("verdict").unwrap().str(),
        Some("consistent"),
        "{t:?}"
    );
    // the adduct was not given: a logged choice, and a parameter with its default
    let p = v.get("provenance").unwrap();
    assert_eq!(
        p.get("params").unwrap().get("adduct").unwrap().str(),
        Some("[M+H]+")
    );
    assert!(p
        .get("choices")
        .unwrap()
        .arr()
        .iter()
        .any(|c| c.get("what").unwrap().str() == Some("adduct")));
    assert_eq!(
        p.get("params")
            .unwrap()
            .get("set")
            .unwrap()
            .get("z_bad")
            .unwrap()
            .f64(),
        Some(3.0)
    );
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn the_members_and_result_are_the_bytes_of_the_route_the_site_answers() {
    let (d, f) = file("isoroute", 100.0, 22.0, 1.0);
    let o = run(&[
        "raw",
        "isotopes",
        &f,
        "--mz",
        "181.07",
        "--rt",
        WIN,
        "--set",
        "min_counts=5",
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let out = String::from_utf8(o.stdout.clone()).unwrap();
    let route = page_routes(&f)
        .raw_isotopes_route(&Query::parse("mz=181.07&rt=2.0,2.4&min_counts=5"))
        .unwrap()
        .unwrap();
    let members = &route[1..route.find(",\"result\":{").unwrap()];
    assert!(out.contains(members), "{route}\n{out}");
    let result = &route[route.find("\"result\":{").unwrap() + 10..route.len() - 2];
    assert!(out.contains(result), "{result}\n{out}");
    let v = json(&o);
    // no formula: a carbon range around 20 and no verdict
    let r = &v.get("results").unwrap().arr()[0];
    assert_eq!(
        r.get("carbon_range").unwrap().get("best").unwrap().f64(),
        Some(20.0)
    );
    assert_eq!(r.get("tests"), Some(&Value::Null));
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn a_formula_far_from_the_mass_warns_and_the_dry_run_reads_no_peak() {
    let (d, f) = file("isowarn", 100.0, 7.0, 1.0);
    let v = json(&run(&[
        "raw",
        "isotopes",
        &f,
        "--mz",
        "181.07",
        "--rt",
        WIN,
        "--formula",
        "C12H22O11",
    ]));
    let w = v.get("warnings").unwrap().arr();
    assert!(
        w.iter()
            .any(|w| w.get("key").unwrap().str() == Some("warn.raw.isotopes.formulaMismatch")),
        "{w:?}"
    );
    let o = run(&[
        "raw",
        "isotopes",
        &f,
        "--mz",
        "181.07",
        "--rt",
        WIN,
        "--dry-run",
        "--set",
        "z_ok=1",
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let p = json(&o);
    assert_eq!(p.get("schema").unwrap().str(), Some("raw-plan/1"));
    assert_eq!(p.get("command").unwrap().str(), Some("raw isotopes"));
    assert_eq!(
        p.get("query")
            .unwrap()
            .get("set")
            .unwrap()
            .get("z_ok")
            .unwrap()
            .f64(),
        Some(1.0)
    );
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn bad_asks_have_their_exit_codes() {
    let (d, f) = file("isobad", 100.0, 7.0, 1.0);
    assert_eq!(
        run(&[
            "raw",
            "isotopes",
            &format!("{f}.absent"),
            "--mz",
            "181",
            "--rt",
            WIN
        ])
        .code,
        4
    );
    // an mzML holds one sample: naming one is a usage error (a file of several samples answers 5 for an unknown one)
    assert_eq!(
        run(&["raw", "isotopes", &f, "--mz", "181", "--rt", WIN, "--sample", "9"]).code,
        2
    );
    for bad in [
        vec![],
        vec!["--mz", "181"],
        vec!["--rt", WIN],
        vec!["--mz", "x", "--rt", WIN],
        vec!["--mz", "181", "--rt", "3,2"],
        vec!["--mz", "181", "--rt", WIN, "--bin", "0.1"],
        vec!["--mz", "181", "--rt", WIN, "--set", "bogus=1"],
        vec!["--mz", "181", "--rt", WIN, "--set", "min_r=3"],
        vec!["--mz", "181", "--rt", WIN, "--adduct", "[M+H]+"],
        vec![
            "--mz",
            "181",
            "--rt",
            WIN,
            "--formula",
            "C6H12O6",
            "--adduct",
            "[M+X]+",
        ],
        vec!["--mz", "181", "--rt", WIN, "--polarity", "x"],
    ] {
        let mut a = vec!["raw", "isotopes", f.as_str()];
        a.extend(bad.iter().copied());
        assert_eq!(run(&a).code, 2, "{bad:?}");
    }
    assert_eq!(
        run(&["raw", "isotopes", "--mz", "181", "--rt", WIN]).code,
        2
    );
    let bad = d.join("b.raw");
    std::fs::write(&bad, b"\x00\x01garbage").unwrap();
    let o = run(&[
        "raw",
        "isotopes",
        &f,
        &bad.display().to_string(),
        "--mz",
        "181.07",
        "--rt",
        WIN,
    ]);
    assert_eq!(o.code, 1, "{}", o.stderr);
    let v = json(&o);
    assert!(v.get("files").unwrap().arr()[1].get("error").is_some());
    assert_eq!(v.get("results").unwrap().arr().len(), 1);
    let _ = std::fs::remove_dir_all(d);
}
