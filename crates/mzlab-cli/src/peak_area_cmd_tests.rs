use super::run;
use mzlab_core::baseline::{peak_area_json, Baseline};

fn args(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

/// A Gaussian (apex 1e5 at 2 min, sigma 0.05 min) on a background of 2000 counts, one point every 0.01 min.
fn trace_file(name: &str) -> (Vec<f64>, Vec<f64>, String) {
    let rt: Vec<f64> = (0..400).map(|i| f64::from(i) * 0.01).collect();
    let y: Vec<f64> = rt
        .iter()
        .map(|t| 1.0e5 * (-((t - 2.0) / 0.05).powi(2) / 2.0).exp() + 2000.0)
        .collect();
    let f = std::env::temp_dir().join(name);
    let body: Vec<String> = rt
        .iter()
        .zip(&y)
        .map(|(a, b)| format!("{a}\t{b}"))
        .collect();
    std::fs::write(&f, body.join("\n")).unwrap();
    (rt, y, f.to_str().unwrap().to_string())
}

#[test]
fn the_area_is_the_text_of_the_page() {
    let (rt, y, p) = trace_file("mzlab_peak_area_trace.tsv");
    let o = run(args(&[
        &p,
        "--from",
        "1.7",
        "--to",
        "2.3",
        "--set",
        "baseline=zero",
    ]));
    assert_eq!(o.code, 0);
    let want = peak_area_json(&rt, &y, 1.7, 2.3, Baseline::Zero).unwrap();
    assert_eq!(String::from_utf8(o.stdout).unwrap(), format!("{want}\n"));
    // from zero the background is inside the area: about 1e5 sigma sqrt(2 pi) + 2000 x 0.6 min, in counts x s
    let v: serde_json::Value = serde_json::from_str(&want).unwrap();
    let area = v["area"].as_f64().unwrap();
    let expect = (1.0e5 * 0.05 * (2.0 * std::f64::consts::PI).sqrt() + 2000.0 * 0.6) * 60.0;
    assert!((area - expect).abs() < 1e-3 * expect, "{area} {expect}");
    // a level through the middle of the peak: signed, the part under it reported
    let o = run(args(&[
        &p,
        "--from",
        "1.7",
        "--to",
        "2.3",
        "--set",
        "baseline=line",
        "--set",
        "base_y0=52000",
    ]));
    let line = Baseline::Line {
        t0: 0.0,
        y0: 52000.0,
        t1: 0.0,
        y1: 52000.0,
    };
    assert_eq!(
        String::from_utf8(o.stdout).unwrap(),
        format!("{}\n", peak_area_json(&rt, &y, 1.7, 2.3, line).unwrap())
    );
    let v: serde_json::Value =
        serde_json::from_str(&peak_area_json(&rt, &y, 1.7, 2.3, line).unwrap()).unwrap();
    assert!(v["below"].as_f64().unwrap() > 0.0 && v["area"].as_f64().unwrap() < 0.0);
}

#[test]
fn no_baseline_named_is_from_zero() {
    let (_, _, p) = trace_file("mzlab_peak_area_trace3.tsv");
    let plain = run(args(&[&p, "--from", "1.7", "--to", "2.3"]));
    let zero = run(args(&[
        &p,
        "--from",
        "1.7",
        "--to",
        "2.3",
        "--set",
        "baseline=zero",
    ]));
    assert_eq!((plain.code, &plain.stdout), (0, &zero.stdout));
    assert!(String::from_utf8(plain.stdout)
        .unwrap()
        .starts_with("{\"baseline\":\"zero\""));
}

#[test]
fn edges_and_bad_numbers_are_refused() {
    let (_, _, p) = trace_file("mzlab_peak_area_trace2.tsv");
    assert_ne!(
        run(args(&[
            &p,
            "--from",
            "1",
            "--to",
            "3",
            "--set",
            "baseline=edges"
        ]))
        .code,
        0
    );
    assert_ne!(
        run(args(&[
            &p,
            "--from",
            "1",
            "--to",
            "3",
            "--set",
            "baseline=line"
        ]))
        .code,
        0
    );
    assert_ne!(
        run(args(&[
            &p,
            "--from",
            "1",
            "--to",
            "3",
            "--set",
            "baseline=line",
            "--set",
            "base_y0=x"
        ]))
        .code,
        0
    );
    assert_ne!(
        run(args(&[&p, "--from", "1", "--to", "3", "--set", "hr_sn=3"])).code,
        0
    );
    assert_ne!(run(args(&[&p, "--set", "baseline=zero"])).code, 0);
    assert_eq!(run(args(&["--help"])).code, 0);
}
