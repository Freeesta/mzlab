use super::run;
use mzlab_core::ionstat::{floor_json, measure, FloorAsk, FloorParams, OneIon};

fn args(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

// 200 points every 0.02 min, all zero but one ion every 40 points and a peak of 2, 3, 2 ions at 2.0 min (2222 cps per ion)
fn trace() -> (Vec<f64>, Vec<f64>) {
    let rt: Vec<f64> = (0..200).map(|i| f64::from(i) * 0.02).collect();
    let mut y: Vec<f64> = (0..200)
        .map(|i| if i % 40 == 7 { 2222.0 } else { 0.0 })
        .collect();
    y[99] = 4444.0;
    y[100] = 6666.0;
    y[101] = 4444.0;
    (rt, y)
}

// one file per test: the tests run in parallel and each removes its own
fn write(tag: &str, rt: &[f64], y: &[f64]) -> std::path::PathBuf {
    let f = std::env::temp_dir().join(format!("mzlab_peak_noise_{tag}_{}.csv", std::process::id()));
    let body: Vec<String> = rt.iter().zip(y).map(|(a, b)| format!("{a},{b}")).collect();
    std::fs::write(&f, format!("rt,y\n{}", body.join("\n"))).unwrap();
    f
}

#[test]
fn the_output_is_the_core_json_byte_for_byte_and_the_log_goes_to_stderr() {
    let (rt, y) = trace();
    let f = write("json", &rt, &y);
    let p = f.to_str().unwrap();
    let o = run(args(&[p, "--rt", "1.95,2.05"]));
    assert_eq!(o.code, 0, "{}", o.stderr);
    let ask = FloorAsk {
        peak: Some((1.95, 2.05)),
        ..FloorAsk::default()
    };
    let core = measure(&rt, &y, ask, FloorParams::default()).unwrap();
    let text = String::from_utf8(o.stdout).unwrap();
    assert_eq!(text, format!("{}\n", floor_json(&core)));
    assert!(text.contains("\"estimator\":\"one_ion\""), "{text}");
    assert!(
        text.contains("\"status\":\"fail\""),
        "three ions over one: S/N 3 < 5: {text}"
    );
    assert!(
        o.stderr
            .starts_with("note: noise floor: 2222 cps from one_ion"),
        "{}",
        o.stderr
    );
    // the names of --set, the same numbers as the core parameters
    let o = run(args(&[
        p,
        "--rt",
        "1.95,2.05",
        "--set",
        "from=0.5",
        "--set",
        "to=3.5",
        "--set",
        "one_ion=1000",
        "--set",
        "min_sn=3",
        "--set",
        "sd_weight=0",
    ]));
    let ask = FloorAsk {
        from: Some(0.5),
        to: Some(3.5),
        peak: Some((1.95, 2.05)),
    };
    let p2 = FloorParams {
        one_ion: OneIon::Cps(1000.0),
        sd_weight: 0.0,
        min_sn: 3.0,
    };
    let core = measure(&rt, &y, ask, p2).unwrap();
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert_eq!(
        String::from_utf8(o.stdout).unwrap(),
        format!("{}\n", floor_json(&core))
    );
    let _ = std::fs::remove_file(f);
}

#[test]
fn usage_data_and_input_errors_have_their_codes() {
    let (rt, y) = trace();
    let f = write("codes", &rt, &y);
    let p = f.to_str().unwrap();
    assert_eq!(run(args(&[p, "--set", "one_ion=-3"])).code, 2);
    assert_eq!(run(args(&[p, "--set", "noise=3"])).code, 2);
    assert_eq!(run(args(&[p, "--rt", "2"])).code, 2);
    assert_eq!(run(args(&[p, "--rt", "3,2"])).code, 2);
    assert_eq!(run(args(&[])).code, 2);
    let all = run(args(&[p, "--rt", "-1,10"]));
    assert_eq!(all.code, 1, "{}", all.stderr);
    assert_eq!(all.error.unwrap().key, "err.noise.noBackground");
    assert_eq!(run(args(&["/nonexistent/t.csv"])).code, 4);
    let off = run(args(&[p, "--rt", "1.95,2.05", "--set", "one_ion=0"]));
    let text = String::from_utf8(off.stdout).unwrap();
    assert!(
        text.contains("\"one_ion_source\":\"off\"") && text.contains("\"status\":\"warn\""),
        "{text}"
    );
    let _ = std::fs::remove_file(f);
}

// the trace above is made of 2222-cps ions: a fill time of 1000 / 2222 ms is the same factor the data estimate gives
#[test]
fn ions_adds_the_ions_member_and_the_log_and_the_cps_members_keep_their_bytes() {
    use mzlab_core::ionscale::{Facts, ScanKind};
    use mzlab_core::ionstat_ions::{floor_json_ions, measure_ions, IonsAsk};
    let (rt, y) = trace();
    let f = write("ions", &rt, &y);
    let p = f.to_str().unwrap();
    let plain = String::from_utf8(run(args(&[p, "--rt", "1.95,2.05"])).stdout).unwrap();
    let o = run(args(&[
        p,
        "--rt",
        "1.95,2.05",
        "--ions",
        "--scan-type",
        "ems",
        "--set",
        "fill_ms=0.45",
    ]));
    assert_eq!(o.code, 0, "{}", o.stderr);
    let text = String::from_utf8(o.stdout).unwrap();
    let ask = FloorAsk {
        peak: Some((1.95, 2.05)),
        ..FloorAsk::default()
    };
    let facts = Facts {
        fill_ms: Some(0.45),
        ..Facts::default()
    };
    let (r, i) = measure_ions(
        &rt,
        &y,
        ask,
        FloorParams::default(),
        Some(IonsAsk {
            kind: Some(ScanKind::Ems),
            facts,
        }),
    )
    .unwrap();
    assert_eq!(text, format!("{}\n", floor_json_ions(&r, i.as_ref())));
    assert!(text.starts_with(&plain[..plain.len() - 2]), "{text}");
    assert!(
        text.contains("\"ion_scale\":{\"scan_type\":\"ems\""),
        "{text}"
    );
    assert!(text.contains("\"field\":\"fill_ms\""), "{text}");
    assert_eq!(o.stderr.matches("note: ").count(), 2, "{}", o.stderr);
    assert!(
        o.stderr.contains("note: ion scale: 0.00045 ions per cps"),
        "{}",
        o.stderr
    );
    // no method value and no scan type: the data estimate of the trace (2222 cps per ion) is used and said so
    let o = run(args(&[p, "--rt", "1.95,2.05", "--ions"]));
    assert_eq!(o.code, 0, "{}", o.stderr);
    let text = String::from_utf8(o.stdout).unwrap();
    assert!(text.contains("\"source\":\"data\""), "{text}");
    assert!(o.stderr.contains("data estimate"), "{}", o.stderr);
    std::fs::remove_file(f).ok();
}

#[test]
fn a_method_value_or_scan_type_without_ions_is_usage_and_a_bad_value_too() {
    let (rt, y) = trace();
    let f = write("ionsbad", &rt, &y);
    let p = f.to_str().unwrap();
    for extra in [&["--set", "fill_ms=0.5"][..], &["--scan-type", "ems"][..]] {
        let mut a = vec![p];
        a.extend_from_slice(extra);
        let o = run(args(&a));
        assert_eq!(o.code, 2, "{extra:?}");
        assert!(o.stderr.contains("needs --ions"), "{}", o.stderr);
    }
    for extra in [
        &["--ions", "--set", "fill_ms=0"][..],
        &["--ions", "--scan-type", "tof"][..],
    ] {
        let mut a = vec![p];
        a.extend_from_slice(extra);
        assert_eq!(run(args(&a)).code, 2, "{extra:?}");
    }
    std::fs::remove_file(f).ok();
}
