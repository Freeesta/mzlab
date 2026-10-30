use super::run;

fn gauss_csv() -> String {
    (0..400)
        .map(|i| {
            let t = i as f64 * 0.02;
            format!(
                "{t},{}",
                100.0 + 5000.0 * (-((t - 2.25) / 0.06).powi(2) / 2.0).exp()
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn args(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

#[test]
fn edges_surround_the_peak_and_snmin_refuses_noise() {
    let f = std::env::temp_dir().join("mzlab_peaks_edges_trace.csv");
    std::fs::write(&f, format!("rt,y\n{}", gauss_csv())).unwrap();
    let p = f.to_str().unwrap();
    let o = run(args(&[p, "--at", "2.25"]));
    let s = String::from_utf8(o.stdout.clone()).unwrap();
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    let (a, b) = (
        v["edges"][0].as_f64().unwrap(),
        v["edges"][1].as_f64().unwrap(),
    );
    assert!(a < 2.25 && b > 2.25 && b - a < 1.2, "{s}");
    let o = run(args(&[p, "--at", "6", "--set", "snMin=1000"]));
    assert_eq!(
        String::from_utf8(o.stdout).unwrap().trim(),
        "{\"edges\":null}"
    );
    assert_ne!(run(args(&[p, "--at", "2", "--set", "bogus=1"])).code, 0);
    let _ = std::fs::remove_file(f);
}
