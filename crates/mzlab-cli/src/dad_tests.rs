use super::*;
use mzlab_core::json::parse;

fn example() -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../mzlab/web/esempi/FullScan_t10.mzML")
        .to_string_lossy()
        .into_owned()
}

fn dad(a: &[&str]) -> Outcome {
    let mut all = vec!["dad".to_string()];
    all.extend(a.iter().map(|s| s.to_string()));
    crate::execute(&all)
}

fn engine_dad() -> Arc<DadData> {
    dad_of(Path::new(&example()), None).unwrap()
}

#[test]
fn spectrum_and_trace_are_the_engine_answers_byte_for_byte() {
    let f = example();
    let file = File::open(&f).unwrap();
    let size = file.metadata().unwrap().len();
    let want =
        mzlab_core::itemroutes::pda_spectrum(&mut Mzml::new(file, size), 1.0, 2.0).unwrap() + "\n";
    let o = dad(&["spectrum", &f, "--rt", "1,2"]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert_eq!(String::from_utf8(o.stdout).unwrap(), want);
    let o = dad(&["trace", &f, "--nm", "250,260"]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert!(String::from_utf8(o.stdout)
        .unwrap()
        .starts_with("{\"rt\":["));
}

#[test]
fn a_file_minus_itself_is_zero_and_reports_no_warning() {
    let f = example();
    let o = dad(&["subtract", &f, &f]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let t = String::from_utf8(o.stdout).unwrap();
    let v = parse(t.trim().as_bytes()).unwrap();
    let sub = v.get("sub").unwrap();
    assert_eq!(sub.get("warnings").unwrap().arr().len(), 0, "{t}");
    assert_eq!(sub.get("scale").unwrap().str(), Some("none"));
    // the spectrum of the difference is all zeros and is the engine's `pda_subtract_data`
    let o = dad(&["spectrum", &f, "--rt", "1,2", "--minus", &f]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let d = engine_dad();
    let want = mzlab_core::itemroutes::pda_subtract_data(
        &d,
        &d,
        &Default::default(),
        mzlab_core::itemroutes::PdaView::Spectrum(1.0, 2.0),
    )
    .unwrap()
        + "\n";
    assert_eq!(String::from_utf8(o.stdout).unwrap(), want);
}

#[test]
fn bad_use_is_exit_2_and_a_missing_file_is_4_with_error_json() {
    let f = example();
    for a in [
        vec!["subtract", &f],
        vec!["spectrum", &f],
        vec!["nope", &f],
        vec!["spectrum", &f, "--bogus"],
        vec!["subtract", &f, &f, "--scale", "x"],
    ] {
        assert_eq!(dad(&a).code, 2, "{a:?}");
    }
    let o = dad(&["spectrum", "/nonexistent/x.mzML", "--rt", "1", "--json"]);
    assert_eq!(o.code, 4);
    assert!(String::from_utf8(o.stdout)
        .unwrap()
        .contains("\"schema\":\"error/1\""));
    assert_eq!(dad(&["--help"]).code, 0);
}

#[test]
fn ms_at_applies_the_signed_offset_and_delay_reports_its_evidence() {
    let f = example();
    let o = dad(&["ms-at", &f, "--rt", "1.5", "--offset", "-0.25"]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let t = String::from_utf8(o.stdout).unwrap();
    let v = parse(t.trim().as_bytes()).unwrap();
    assert_eq!(v.get("rt_ms").and_then(|x| x.f64()), Some(1.25), "{t}");
    assert_eq!(v.get("offset_min").and_then(|x| x.f64()), Some(-0.25));
}

#[test]
fn ms_at_takes_the_nearest_scan_and_never_answers_an_empty_spectrum() {
    let f = example();
    // an instant that is not a scan time used to give `mz: []` with exit 0
    let o = dad(&["ms-at", &f, "--rt", "1.5003"]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let t = String::from_utf8(o.stdout).unwrap();
    let v = parse(t.trim().as_bytes()).unwrap();
    assert!(
        !v.get("spectrum")
            .unwrap()
            .get("mz")
            .unwrap()
            .arr()
            .is_empty(),
        "{t}"
    );
    let (scan, gap) = (
        v.get("scan_rt").and_then(|x| x.f64()).unwrap(),
        v.get("gap_min").and_then(|x| x.f64()).unwrap(),
    );
    assert!((scan - 1.5003).abs() - gap < 1e-6 && gap < 0.5, "{t}");
    assert!(v.get("warnings").unwrap().arr().is_empty(), "{t}");
    assert!(v.get("ms_range").is_some());
}

#[test]
fn ms_at_outside_the_scans_or_without_the_level_is_exit_1_with_the_evidence() {
    let f = example();
    for (args, key) in [
        (
            vec!["ms-at", &f, "--rt", "900", "--json"],
            "err.dad.msRange",
        ),
        (
            vec!["ms-at", &f, "--rt", "1.5", "--level", "3", "--json"],
            "err.dad.msNoLevel",
        ),
    ] {
        let o = dad(&args);
        assert_eq!(o.code, 1, "{args:?} {}", o.stderr);
        let t = String::from_utf8(o.stdout).unwrap();
        assert!(
            t.contains("\"schema\":\"error/1\"") && t.contains(key),
            "{t}"
        );
    }
}

#[test]
fn delay_with_no_matching_peak_is_exit_1_with_the_delay_and_correlation_found() {
    // the example's UV peaks match none of the TIC's: the engine used to answer `peaks: 0` with exit 0
    let o = dad(&["delay", &f_example(), "--json"]);
    assert_eq!(o.code, 1, "{}", o.stderr);
    let t = String::from_utf8(o.stdout).unwrap();
    let v = parse(t.trim().as_bytes()).unwrap();
    assert_eq!(
        v.get("error_key").and_then(|x| x.str()),
        Some("err.dad.delayNoPeaks"),
        "{t}"
    );
    let p = v.get("params").unwrap();
    assert!(p.get("delay").is_some() && p.get("corr").is_some(), "{t}");
}

fn f_example() -> String {
    example()
}

#[test]
fn shift_auto_is_the_engine_estimate_with_its_evidence_and_a_bad_value_is_exit_2() {
    let f = example();
    let o = dad(&[
        "spectrum", &f, "--rt", "1,2", "--minus", &f, "--shift", "auto",
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert!(o.stderr.contains("dad: shift auto = "), "{}", o.stderr);
    let d = engine_dad();
    let (opts, view) =
        mzlab_core::itemroutes::pda_sub_args("spectrum", 1.0, 2.0, "none", 0.0, true).unwrap();
    let want = mzlab_core::itemroutes::pda_subtract_data(&d, &d, &opts, view).unwrap() + "\n";
    let got = String::from_utf8(o.stdout).unwrap();
    assert_eq!(got, want);
    let v = parse(got.trim().as_bytes()).unwrap();
    let al = v.get("sub").unwrap().get("align").unwrap();
    assert_eq!(al.get("method").unwrap().str(), Some("twc_xcorr"));
    assert_eq!(al.get("shift_min").unwrap().f64(), Some(0.0), "{got}");
    // a number stays a shift by hand: no align member
    let o = dad(&["subtract", &f, &f, "--shift", "0.05"]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert!(!String::from_utf8(o.stdout).unwrap().contains("\"align\""));
    assert_eq!(dad(&["subtract", &f, &f, "--shift", "soon"]).code, 2);
}

#[test]
fn trace_csv_is_the_same_points_as_the_json_and_feeds_the_peak_commands() {
    let f = example();
    let j = dad(&["trace", &f, "--nm", "250,260"]);
    assert_eq!(j.code, 0, "{}", j.stderr);
    let jt = String::from_utf8(j.stdout).unwrap();
    let v = parse(jt.trim().as_bytes()).unwrap();
    let (rt, y) = (v.get("rt").unwrap().arr(), v.get("y").unwrap().arr());
    let o = dad(&["trace", &f, "--nm", "250,260", "--format", "csv"]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let t = String::from_utf8(o.stdout).unwrap();
    let mut lines = t.lines();
    assert_eq!(lines.next(), Some("rt_min,intensity"));
    let rows: Vec<&str> = lines.collect();
    assert_eq!(rows.len(), rt.len());
    assert!(!rows.is_empty() && rt.len() == y.len());
    // the tokens are the engine's own: the CSV and the JSON hold the same text
    let want = crate::extract::points_csv(
        "rt_min",
        "intensity",
        &crate::extract::pairs(
            &crate::extract::tokens(&jt, "rt").unwrap(),
            &crate::extract::tokens(&jt, "y").unwrap(),
            None,
        ),
    );
    assert_eq!(t, want);
    // a difference prints its rows the same way; --format is for trace only; a bad value is usage
    let d = dad(&["trace", &f, "--nm", "250", "--minus", &f, "--format", "csv"]);
    assert_eq!(d.code, 0, "{}", d.stderr);
    assert!(String::from_utf8(d.stdout)
        .unwrap()
        .starts_with("rt_min,intensity\n"));
    for a in [
        vec!["spectrum", &f, "--rt", "1", "--format", "csv"],
        vec!["trace", &f, "--nm", "250", "--format", "xml"],
    ] {
        assert_eq!(dad(&a).code, 2, "{a:?}");
    }
}

#[test]
fn peaks_give_lambda_max_and_ions_and_say_which_offset_was_used() {
    let f = example();
    let o = dad(&["peaks", &f, "--top", "2"]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert!(o.stderr.contains("dad: offset auto"), "{}", o.stderr);
    let t = String::from_utf8(o.stdout).unwrap();
    let v = parse(t.trim().as_bytes()).unwrap();
    let peaks = v.get("peaks").unwrap().arr();
    assert!(!peaks.is_empty(), "{t}");
    for p in peaks {
        for k in [
            "rt_uv",
            "height",
            "lambda_max_nm",
            "rt_ms",
            "scan_rt",
            "gap_min",
        ] {
            assert!(p.get(k).and_then(|x| x.f64()).is_some(), "{k}: {t}");
        }
        let ions = p.get("ions").unwrap().arr();
        assert!(!ions.is_empty() && ions.len() <= 2);
        let y: Vec<f64> = ions.iter().filter_map(|i| i.get("y")?.f64()).collect();
        assert!(y.windows(2).all(|w| w[0] >= w[1]), "strongest first: {t}");
        assert_eq!(ions[0].get("rel").unwrap().f64(), Some(100.0));
    }
    assert_eq!(
        v.get("trace").unwrap().get("kind").unwrap().str(),
        Some("twc")
    );
    // the CSV has a row per ion with the answer's own numbers; a given offset is `source: given` and is logged
    let c = dad(&[
        "peaks", &f, "--top", "2", "--offset", "0.1", "--format", "csv",
    ]);
    assert_eq!(c.code, 0, "{}", c.stderr);
    assert!(c.stderr.contains("(given)"), "{}", c.stderr);
    let csv = String::from_utf8(c.stdout).unwrap();
    let mut rows = csv.lines();
    assert_eq!(
        rows.next(),
        Some("peak,rt_uv_min,height_mau,lambda_max_nm,rt_ms_min,mz,intensity,rel_pct")
    );
    assert_eq!(rows.count(), peaks.len() * 2);
}

#[test]
fn peaks_usage_errors_are_exit_2() {
    let f = example();
    for a in [
        vec!["peaks", &f, "--top", "0"],
        vec!["peaks", &f, "--top", "many"],
        vec!["peaks", &f, "--offset", "soon"],
        vec!["ms-at", &f, "--rt", "1", "--offset", "auto"],
        vec!["peaks"],
    ] {
        assert_eq!(dad(&a).code, 2, "{a:?}");
    }
    assert_eq!(dad(&["peaks", "/nonexistent/x.mzML"]).code, 4);
}

#[test]
fn a_peak_outside_the_ms_scans_is_kept_without_ions_and_says_why() {
    let f = example();
    let o = dad(&["peaks", &f, "--offset", "500"]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let t = String::from_utf8(o.stdout).unwrap();
    let v = parse(t.trim().as_bytes()).unwrap();
    let peaks = v.get("peaks").unwrap().arr();
    assert!(!peaks.is_empty());
    for p in peaks {
        assert_eq!(p.get("ions").unwrap().arr().len(), 0, "{t}");
        assert_eq!(
            p.get("warnings").unwrap().arr()[0].str(),
            Some("ms_out_of_range")
        );
        assert!(p.get("scan_rt").unwrap().f64().is_none());
    }
}
