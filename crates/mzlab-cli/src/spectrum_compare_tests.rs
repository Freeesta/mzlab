//! `spectrum compare`: a known cosine on synthetic spectra, the collision-energy and precursor warnings, the exit codes and the usage errors.

use mzlab_core::json::{parse, Value};

use crate::extract::tests::{dir, run};

/// One scan: its time, precursor, collision energy and the `(m/z, counts)` peaks (centroids at whole masses).
type Scan = (f64, f64, f64, Vec<(f64, f32)>);

fn mzml(tag: &str, name: &str, scans: &[Scan]) -> (std::path::PathBuf, String) {
    use mzlab_core::mzmlwrite::{Header, MzmlWriter, SpectrumExtra};
    let d = dir(tag);
    let mut w = MzmlWriter::new(Vec::new(), &Header::default()).unwrap();
    w.open_spectra(scans.len()).unwrap();
    for (k, (rt, prec, ce, peaks)) in scans.iter().enumerate() {
        let s = mzlab_core::mzml::Spectrum {
            index: k,
            id: format!("scan={}", k + 1),
            level: 2,
            rt: *rt,
            polarity: 1,
            profile: false,
            filter: format!("ITMS + c ESI d Full ms2 {prec}@cid{ce} [50.00-300.00]"),
            precursor: Some(*prec),
            mz: peaks.iter().map(|p| p.0).collect(),
            intensity: peaks.iter().map(|p| p.1).collect(),
            ..Default::default()
        };
        let extra = SpectrumExtra {
            ce: Some(*ce),
            ..Default::default()
        };
        w.spectrum_with(&s, &extra).unwrap();
    }
    w.close_spectra().unwrap();
    let f = d.join(name);
    std::fs::write(&f, w.finish().unwrap()).unwrap();
    (d, f.to_string_lossy().into_owned())
}

fn sample(prec: f64, ce: f64, peaks: &[(f64, f32)]) -> Vec<Scan> {
    vec![(1.0, prec, ce, peaks.to_vec())]
}

fn doc(args: &[&str]) -> (Value, String) {
    let o = run(args);
    assert_eq!(o.code, 0, "{}", o.stderr);
    (parse(&o.stdout).unwrap(), o.stderr)
}

fn keys(v: &Value) -> Vec<String> {
    v.get("warnings")
        .unwrap()
        .arr()
        .iter()
        .map(|w| w.get("key").and_then(Value::str).unwrap().to_string())
        .collect()
}

#[test]
fn two_spectra_of_a_known_cosine_list_the_matched_and_the_unmatched_ions() {
    let (d1, a) = mzml(
        "cmp_a",
        "a.mzML",
        &sample(253.1, 35.0, &[(100.0, 4.0), (101.0, 9.0), (102.0, 16.0)]),
    );
    let (d2, b) = mzml(
        "cmp_b",
        "b.mzML",
        &sample(253.1, 35.0, &[(100.0, 4.0), (101.0, 9.0), (103.0, 25.0)]),
    );
    let (v, err) = doc(&["spectrum", "compare", &a, &b, "--a-rt", "1", "--b-rt", "1"]);
    assert_eq!(v.get("schema").unwrap().str(), Some("spectrum-compare/1"));
    assert_eq!(
        v.get("cosine").and_then(Value::f64),
        Some(mzlab_core::json::round(13.0 / (29.0f64 * 38.0).sqrt(), 6))
    );
    assert_eq!(v.get("n_matched").and_then(Value::f64), Some(2.0));
    let m = &v.get("matched").unwrap().arr()[1];
    assert_eq!(
        (
            m.get("bin").and_then(Value::f64),
            m.get("a").and_then(Value::f64),
            m.get("b").and_then(Value::f64)
        ),
        (Some(101.0), Some(9.0), Some(9.0))
    );
    assert_eq!(
        v.get("only_a").unwrap().arr()[0]
            .get("bin")
            .and_then(Value::f64),
        Some(102.0)
    );
    assert_eq!(
        v.get("only_b").unwrap().arr()[0]
            .get("bin")
            .and_then(Value::f64),
        Some(103.0)
    );
    let side = v.get("a").unwrap();
    assert_eq!(
        (
            side.get("ce").and_then(Value::f64),
            side.get("precursor").and_then(Value::f64)
        ),
        (Some(35.0), Some(253.1))
    );
    assert!(keys(&v).is_empty());
    assert!(err.contains("note: cosine 0.39"), "{err}");
    for d in [d1, d2] {
        let _ = std::fs::remove_dir_all(d);
    }
}

#[test]
fn different_collision_energies_give_a_warning_on_both_channels() {
    let (d1, a) = mzml("cmp_ce_a", "a.mzML", &sample(253.1, 35.0, &[(100.0, 4.0)]));
    let (d2, b) = mzml("cmp_ce_b", "b.mzML", &sample(253.1, 20.0, &[(100.0, 4.0)]));
    let (v, err) = doc(&["spectrum", "compare", &a, &b, "--a-rt", "1", "--b-rt", "1"]);
    assert_eq!(keys(&v), ["warn.spectrum.ceDiffer"]);
    assert!(
        err.contains("warning:") && err.contains("warn.spectrum.ceDiffer"),
        "{err}"
    );
    // the tolerance is a --set and the default is in the document
    let (v, _) = doc(&[
        "spectrum",
        "compare",
        &a,
        &b,
        "--a-rt",
        "1",
        "--b-rt",
        "1",
        "--set",
        "ce_tol=20",
    ]);
    assert!(keys(&v).is_empty());
    assert_eq!(
        v.get("params").unwrap().get("ce_tol").and_then(Value::f64),
        Some(20.0)
    );
    for d in [d1, d2] {
        let _ = std::fs::remove_dir_all(d);
    }
}

#[test]
fn the_tolerance_set_pairs_neighbouring_bins() {
    let (d, f) = mzml(
        "cmp_tol",
        "s.mzML",
        &[
            (1.0, 253.1, 35.0, vec![(100.0, 4.0)]),
            (2.0, 253.1, 35.0, vec![(101.0, 4.0)]),
        ],
    );
    let (v, _) = doc(&["spectrum", "compare", &f, "--a-scan", "1", "--b-scan", "2"]);
    assert_eq!(v.get("cosine").and_then(Value::f64), Some(0.0));
    let (v, _) = doc(&[
        "spectrum", "compare", &f, "--a-scan", "1", "--b-scan", "2", "--set", "tol=1",
    ]);
    assert_eq!(v.get("cosine").and_then(Value::f64), Some(1.0));
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn a_window_with_two_precursors_needs_the_precursor_and_says_which() {
    let scans = vec![
        (1.0, 253.1, 35.0, vec![(100.0, 4.0)]),
        (1.1, 300.2, 35.0, vec![(100.0, 9.0), (150.0, 1.0)]),
    ];
    let (d, f) = mzml("cmp_prec", "s.mzML", &scans);
    let o = run(&[
        "spectrum", "compare", &f, "--a-rt", "0.9,1.2", "--b-scan", "1",
    ]);
    assert_eq!(o.code, 5, "{}", o.stderr);
    assert!(
        o.stderr.contains("253.1") && o.stderr.contains("300.2"),
        "{}",
        o.stderr
    );
    let (v, _) = doc(&[
        "spectrum",
        "compare",
        &f,
        "--a-rt",
        "0.9,1.2",
        "--a-precursor",
        "300.2",
        "--b-scan",
        "2",
    ]);
    assert_eq!(v.get("cosine").and_then(Value::f64), Some(1.0));
    assert_eq!(
        v.get("a").unwrap().get("precursor").and_then(Value::f64),
        Some(300.2)
    );
    // a time with a precursor takes the nearest scan of that precursor
    let (v, _) = doc(&[
        "spectrum",
        "compare",
        &f,
        "--a-rt",
        "1",
        "--a-precursor",
        "300.2",
        "--b-scan",
        "2",
    ]);
    assert_eq!(
        v.get("a").unwrap().get("n_bins").and_then(Value::f64),
        Some(2.0)
    );
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn a_shifted_side_matches_the_unshifted_one_once_its_offset_is_taken_off() {
    // ions at 100.3.. (bin 100); the second side is read 0.4 Da high, so its ions fall into the next bins
    let ions = [(100.3, 4.0), (101.3, 9.0), (102.3, 16.0)];
    let shifted: Vec<(f64, f32)> = ions.iter().map(|(m, y)| (m + 0.4, *y)).collect();
    let (d1, a) = mzml("cmp_off_a", "a.mzML", &sample(253.1, 35.0, &ions));
    let (d2, b) = mzml("cmp_off_b", "b.mzML", &sample(253.1, 35.0, &shifted));
    let cosine = |extra: &[&str]| {
        let mut args = vec!["spectrum", "compare", &a, &b, "--a-rt", "1", "--b-rt", "1"];
        args.extend_from_slice(extra);
        doc(&args)
    };
    let cos = |v: &Value| v.get("cosine").and_then(Value::f64).unwrap();
    let (plain, plain_err) = cosine(&[]);
    let (fixed, err) = cosine(&["--b-mz-offset", "0.4"]);
    // the unshifted pair: the same ions on both sides
    let (same, _) = doc(&["spectrum", "compare", &a, &a, "--a-rt", "1", "--b-rt", "1"]);
    assert_eq!(cos(&same), 1.0);
    assert!(cos(&plain) < 0.9, "{}", cos(&plain));
    assert_eq!(cos(&fixed), cos(&same));
    // each side's block carries the offset used; the side that was not asked states 0
    let da = |v: &Value, side: &str| {
        v.get(side)
            .unwrap()
            .get("mz_offset_da")
            .and_then(Value::f64)
    };
    assert_eq!((da(&fixed, "a"), da(&fixed, "b")), (Some(0.0), Some(0.4)));
    assert!(fixed
        .get("b")
        .unwrap()
        .get("query")
        .unwrap()
        .str()
        .unwrap()
        .contains("mzoff=0.4"));
    assert!(
        err.contains("mz offset b (b.mzML): +0.4 Da (fixed)"),
        "{err}"
    );
    assert!(!err.contains("mz offset a"), "{err}");
    assert!(!plain_err.contains("mz offset"), "{plain_err}");
    // both flags together: each side is logged with its own offset
    let (_, both_err) = cosine(&["--a-mz-offset", "0.1", "--b-mz-offset", "0.4"]);
    assert!(
        both_err.contains("mz offset a (a.mzML): +0.1 Da (fixed)")
            && both_err.contains("mz offset b (b.mzML): +0.4 Da (fixed)"),
        "{both_err}"
    );
    for d in [d1, d2] {
        let _ = std::fs::remove_dir_all(d);
    }
}

#[test]
fn an_auto_offset_that_finds_no_ion_is_logged_on_its_side() {
    let (d, f) = mzml(
        "cmp_off_auto",
        "s.mzML",
        &sample(253.1, 35.0, &[(100.0, 4.0)]),
    );
    let (v, err) = doc(&[
        "spectrum",
        "compare",
        &f,
        "--a-rt",
        "1",
        "--b-rt",
        "1",
        "--a-mz-offset",
        "auto:300",
    ]);
    let a = v.get("a").unwrap();
    assert_eq!(a.get("mz_offset_da").and_then(Value::f64), Some(0.0));
    assert_eq!(
        a.get("mz_offset").unwrap().get("status").unwrap().str(),
        Some("not_found")
    );
    assert!(
        err.contains("(auto on 300, not found)") && err.contains("warn.mz.offset.notFound"),
        "{err}"
    );
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn usage_errors_exit_2_and_a_missing_scan_exits_5() {
    let (d, f) = mzml("cmp_use", "s.mzML", &sample(253.1, 35.0, &[(100.0, 4.0)]));
    assert_eq!(run(&["spectrum", "compare", &f, "--a-rt", "1"]).code, 2);
    assert_eq!(
        run(&["spectrum", "compare", &f, "--a-rt", "1", "--a-scan", "1", "--b-rt", "1"]).code,
        2
    );
    assert_eq!(
        run(&["spectrum", "compare", &f, "--a-rt", "1", "--b-rt", "1", "--set", "nope=1"]).code,
        2
    );
    assert_eq!(
        run(&["spectrum", "compare", &f, "--a-rt", "1", "--b-rt", "1", "--set", "tol=-1"]).code,
        2
    );
    // an offset that is not a number or auto:MZ[@RT] is a usage error, on either side
    for flag in ["--a-mz-offset", "--b-mz-offset"] {
        for bad in ["x", "auto:", "auto:-5"] {
            let o = run(&[
                "spectrum", "compare", &f, "--a-rt", "1", "--b-rt", "1", flag, bad,
            ]);
            assert_eq!(o.code, 2, "{flag} {bad}: {}", o.stderr);
            assert!(o.stderr.contains(flag), "{}", o.stderr);
        }
    }
    assert_eq!(
        run(&["spectrum", "compare", &f, "--a-scan", "99", "--b-rt", "1"]).code,
        5
    );
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn the_document_has_the_keys_of_its_schema() {
    let (d1, a) = mzml(
        "cmp_sch_a",
        "a.mzML",
        &sample(253.1, 35.0, &[(100.0, 4.0), (101.0, 9.0)]),
    );
    let (d2, b) = mzml(
        "cmp_sch_b",
        "b.mzML",
        &sample(253.1, 20.0, &[(100.0, 4.0), (103.0, 25.0)]),
    );
    let (v, _) = doc(&["spectrum", "compare", &a, &b, "--a-rt", "1", "--b-rt", "1"]);
    let schema = parse(include_bytes!(
        "../../../docs/schemi/spectrum-compare-1.json"
    ))
    .unwrap();
    let keys = |v: &Value| {
        v.entries()
            .iter()
            .map(|(k, _)| k.clone())
            .collect::<std::collections::BTreeSet<_>>()
    };
    let props = schema.get("properties").unwrap();
    assert_eq!(keys(&v), keys(props));
    assert_eq!(
        keys(v.get("params").unwrap()),
        keys(props.get("params").unwrap().get("properties").unwrap())
    );
    let defs = schema.get("$defs").unwrap();
    assert_eq!(
        keys(v.get("a").unwrap()),
        keys(defs.get("side").unwrap().get("properties").unwrap())
    );
    for (k, def) in [("only_a", "unmatched"), ("only_b", "unmatched")] {
        assert_eq!(
            keys(&v.get(k).unwrap().arr()[0]),
            keys(
                defs.get(def)
                    .unwrap()
                    .get("items")
                    .unwrap()
                    .get("properties")
                    .unwrap()
            )
        );
    }
    let item = props
        .get("matched")
        .unwrap()
        .get("items")
        .unwrap()
        .get("properties")
        .unwrap();
    assert_eq!(keys(&v.get("matched").unwrap().arr()[0]), keys(item));
    for d in [d1, d2] {
        let _ = std::fs::remove_dir_all(d);
    }
}
