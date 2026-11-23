use super::run;
use mzlab_core::ionstat::{floor_json, measure, FloorAsk, FloorParams, OneIon};

fn args(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

// 200 points every 0.02 min, all zero but one ion every 40 points and a peak of 2, 3, 2 ions at 2.0 min (2222 cps per ion)
fn trace() -> (Vec<f64>, Vec<f64>) {
    let rt: Vec<f64> = (0..200).map(|i| f64::from(i) * 0.02).collect();
    let mut y: Vec<f64> = (0..200).map(|i| if i % 40 == 7 { 2222.0 } else { 0.0 }).collect();
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
    assert!(text.contains("\"status\":\"fail\""), "three ions over one: S/N 3 < 5: {text}");
    assert!(o.stderr.starts_with("note: noise floor: 2222 cps from one_ion"), "{}", o.stderr);
    // the names of --set, the same numbers as the core parameters
    let o = run(args(&[
        p, "--rt", "1.95,2.05", "--set", "from=0.5", "--set", "to=3.5", "--set", "one_ion=1000", "--set", "min_sn=3",
        "--set", "sd_weight=0",
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
    assert_eq!(String::from_utf8(o.stdout).unwrap(), format!("{}\n", floor_json(&core)));
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
    assert!(text.contains("\"one_ion_source\":\"off\"") && text.contains("\"status\":\"warn\""), "{text}");
    let _ = std::fs::remove_file(f);
}
