//! `spectrum get --bin`: a fragment the profile splits between 106.7 and 107.0 is ONE nominal bin, the offset is taken off before the cut and both the
//! rule and the offset are logged.

use mzlab_core::json::{parse, Value};

use crate::extract::tests::{dir, run};

/// Profile points of 0.1 Da on 100.02 + 0.1 i (no point on a bin edge), a baseline of 5 and a fragment of six points from 106.62 to 107.12 (the
/// profile of one ion between two integers), in 4 scans of one minute.
fn split_fragment(tag: &str) -> (std::path::PathBuf, String) {
    use mzlab_core::mzmlwrite::{Header, MzmlWriter};
    let d = dir(tag);
    let mut w = MzmlWriter::new(Vec::new(), &Header::default()).unwrap();
    w.open_spectra(4).unwrap();
    for k in 0..4 {
        let (mz, y): (Vec<f64>, Vec<f32>) = (0..100).map(|i| (grid(i), intensity(i))).unzip();
        let s = mzlab_core::mzml::Spectrum {
            index: k,
            id: format!("scan={}", k + 1),
            level: 1,
            rt: k as f64,
            polarity: 1,
            profile: true,
            filter: "ITMS + p ESI Full ms [100.00-110.00]".into(),
            mz,
            intensity: y,
            ..Default::default()
        };
        w.spectrum(&s).unwrap();
    }
    w.close_spectra().unwrap();
    let f = d.join("split.mzML");
    std::fs::write(&f, w.finish().unwrap()).unwrap();
    (d, f.to_string_lossy().into_owned())
}

fn grid(i: usize) -> f64 {
    100.02 + 0.1 * i as f64
}

fn intensity(i: usize) -> f32 {
    // the six points of the fragment: 106.62 .. 107.12
    let peak = [400.0, 800.0, 1000.0, 1000.0, 800.0, 400.0];
    5.0 + i
        .checked_sub(66)
        .and_then(|k| peak.get(k))
        .copied()
        .unwrap_or(0.0)
}

fn doc(args: &[&str]) -> Value {
    let o = run(args);
    assert_eq!(o.code, 0, "{}", o.stderr);
    parse(&o.stdout).unwrap()
}

fn column(v: &Value, key: &str) -> Vec<f64> {
    v.get("bins")
        .unwrap()
        .arr()
        .iter()
        .map(|b| b.get(key).and_then(Value::f64).unwrap())
        .collect()
}

#[test]
fn a_fragment_split_between_two_integers_is_one_nominal_bin() {
    let (d, f) = split_fragment("bins_one");
    let v = doc(&["spectrum", "get", &f, "--rt", "1", "--bin", "nominal"]);
    assert_eq!(v.get("schema").unwrap().str(), Some("spectrum-bins/1"));
    assert_eq!(v.get("rule").unwrap().str(), Some("nominal"));
    assert_eq!(v.get("mz_offset_da").and_then(Value::f64), Some(0.0));
    let (bin, counts, mz, points) = (
        column(&v, "bin"),
        column(&v, "counts"),
        column(&v, "mz"),
        column(&v, "points"),
    );
    assert_eq!(bin[0], 100.0);
    let at = bin.iter().position(|b| *b == 107.0).unwrap();
    // 106.62 .. 107.12 are all in [106.5, 107.5): ten points, the fragment once
    assert_eq!(points[at], 10.0);
    assert_eq!(counts[at], 4450.0);
    let want = (65..75)
        .map(|i| grid(i) * f64::from(intensity(i)))
        .sum::<f64>()
        / 4450.0;
    assert!(
        (mz[at] - want).abs() < 1e-3,
        "{} vs {want} {bin:?} {mz:?} {counts:?} {points:?}",
        mz[at]
    );
    assert!(
        !bin.contains(&106.0) || counts[bin.iter().position(|b| *b == 106.0).unwrap()] < 100.0,
        "{bin:?} {counts:?}"
    );
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn the_offset_is_taken_off_before_the_cut_and_logged() {
    let (d, f) = split_fragment("bins_off");
    let o = run(&[
        "spectrum",
        "get",
        &f,
        "--rt",
        "1",
        "--bin",
        "nominal",
        "--mz-offset",
        "0.24",
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let v = parse(&o.stdout).unwrap();
    assert_eq!(v.get("mz_offset_da").and_then(Value::f64), Some(0.24));
    // the same fragment is now cut: its first two points (106.62, 106.72 minus 0.24) are below 106.5
    let (bin, counts) = (column(&v, "bin"), column(&v, "counts"));
    let k106 = bin.iter().position(|b| *b == 106.0).unwrap();
    assert_eq!(counts[k106], 1200.0 + 5.0 * 10.0, "{bin:?} {counts:?}");
    assert!(
        o.stderr.contains("mz offset split.mzML: +0.24 Da (fixed)"),
        "{}",
        o.stderr
    );
    assert!(
        o.stderr
            .contains("note: bins nominal: half-open [bin - 0.5, bin + 0.5)"),
        "{}",
        o.stderr
    );
    // without the flag the offset is still stated, as 0
    let o = run(&[
        "spectrum", "get", &f, "--rt", "1", "--bin", "nominal", "--format", "csv",
    ]);
    assert!(
        o.stderr.contains("mz offset split.mzML: +0 Da (no offset)"),
        "{}",
        o.stderr
    );
    let csv = String::from_utf8(o.stdout).unwrap();
    assert!(csv.starts_with("bin,mz,counts,points\n100,"), "{csv}");
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn tenths_cut_around_multiples_of_a_tenth() {
    let (d, f) = split_fragment("bins_tenth");
    let v = doc(&["spectrum", "get", &f, "--rt", "1", "--bin", "0.1"]);
    let (bin, counts, points) = (
        column(&v, "bin"),
        column(&v, "counts"),
        column(&v, "points"),
    );
    // one grid point per tenth: 106.62 is in [106.55, 106.65), the bin 106.6
    let k = bin.iter().position(|b| (*b - 106.6).abs() < 1e-9).unwrap();
    assert_eq!((points[k], counts[k]), (1.0, 405.0));
    assert_eq!(v.get("width_da").and_then(Value::f64), Some(0.1));
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn an_estimate_that_finds_no_ion_is_logged_and_the_offset_is_zero() {
    let (d, f) = split_fragment("bins_auto");
    let o = run(&[
        "spectrum",
        "get",
        &f,
        "--rt",
        "1",
        "--bin",
        "nominal",
        "--mz-offset",
        "auto:300",
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert!(
        o.stderr.contains("(auto on 300, not found)")
            && o.stderr.contains("warn.mz.offset.notFound"),
        "{}",
        o.stderr
    );
    let v = parse(&o.stdout).unwrap();
    assert_eq!(
        v.get("mz_offset").unwrap().get("status").unwrap().str(),
        Some("not_found")
    );
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn bad_requests_are_usage_errors() {
    let (d, f) = split_fragment("bins_bad");
    for args in [
        vec!["--bin", "1"],
        vec!["--mz-offset", "0.2"],
        vec!["--bin", "nominal", "--mz-offset", "abc"],
        vec!["--bin", "nominal", "--mz-offset", "0.9"],
        vec!["--bin", "nominal", "--minus", &f],
    ] {
        let mut a = vec!["spectrum", "get", &f, "--rt", "1"];
        a.extend(args.iter().copied());
        a.push("--json");
        let o = run(&a);
        assert_eq!(o.code, 2, "{args:?}: {}", o.stderr);
        assert!(
            String::from_utf8(o.stdout).unwrap().contains("error/1"),
            "{args:?}"
        );
    }
    let o = run(&[
        "spectrum", "get", &f, "--rt", "0,2", "--bin", "nominal", "--sum", "--json",
    ]);
    assert_eq!(o.code, 2, "{}", o.stderr);
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn the_document_has_the_keys_of_its_schema() {
    let (d, f) = split_fragment("bins_schema");
    let v = doc(&[
        "spectrum",
        "get",
        &f,
        "--rt",
        "1",
        "--bin",
        "nominal",
        "--mz-offset",
        "0.24",
    ]);
    let schema = parse(include_bytes!("../../../docs/schemi/spectrum-bins-1.json")).unwrap();
    let keys = |v: &Value| {
        v.entries()
            .iter()
            .map(|(k, _)| k.clone())
            .collect::<std::collections::BTreeSet<_>>()
    };
    let props = schema.get("properties").unwrap();
    assert_eq!(keys(&v), keys(props));
    let item = props
        .get("bins")
        .unwrap()
        .get("items")
        .unwrap()
        .get("properties")
        .unwrap();
    assert_eq!(keys(&v.get("bins").unwrap().arr()[0]), keys(item));
    let ev = v.get("evidence").unwrap();
    for k in ["route", "query", "scan", "answer"] {
        assert!(ev.get(k).is_some(), "{k}");
    }
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn a_high_resolution_file_is_refused_with_its_key() {
    use mzlab_core::mzmlwrite::{Header, MzmlWriter};
    let d = dir("bins_hr");
    let mut w = MzmlWriter::new(Vec::new(), &Header::default()).unwrap();
    w.open_spectra(3).unwrap();
    for k in 0..3 {
        let s = mzlab_core::mzml::Spectrum {
            index: k,
            id: format!("scan={}", k + 1),
            level: 1,
            rt: k as f64,
            polarity: 1,
            profile: false,
            filter: "FTMS + c ESI Full ms [100.0000-1000.0000]".into(),
            mz: vec![106.07127, 107.04961, 301.14123, 302.14418],
            intensity: vec![900.0, 100.0, 5000.0, 600.0],
            ..Default::default()
        };
        w.spectrum(&s).unwrap();
    }
    w.close_spectra().unwrap();
    let f = d.join("hr.mzML");
    std::fs::write(&f, w.finish().unwrap()).unwrap();
    let f = f.to_string_lossy().into_owned();
    let o = run(&[
        "spectrum", "get", &f, "--rt", "0,2", "--bin", "nominal", "--json",
    ]);
    assert_eq!(o.code, 1, "{}", o.stderr);
    assert!(String::from_utf8(o.stdout)
        .unwrap()
        .contains("err.spectrum.rebin.hr"));
    // without --bin the same file answers as before
    assert_eq!(run(&["spectrum", "get", &f, "--rt", "0,2"]).code, 0);
    let _ = std::fs::remove_dir_all(d);
}
