use super::*;
use mzlab_core::json::parse;

fn example() -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../mzlab/web/esempi/FullScan_t10.mzML")
        .to_string_lossy()
        .into_owned()
}

fn dad(a: &[&str]) -> Outcome {
    run(a.iter().map(|s| s.to_string()).collect())
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
fn bad_use_is_exit_2_and_a_data_error_is_1_with_errore_json() {
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
    assert_eq!(o.code, 1);
    assert!(String::from_utf8(o.stdout)
        .unwrap()
        .contains("\"schema\":\"errore/1\""));
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
    let o = dad(&["delay", &f]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert!(String::from_utf8(o.stdout)
        .unwrap()
        .starts_with("{\"delay\":"));
}
