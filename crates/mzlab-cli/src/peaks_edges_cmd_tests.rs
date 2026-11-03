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

#[test]
fn snake_case_and_camel_case_set_the_same_field() {
    let f = std::env::temp_dir().join("mzlab_peak_edges_alias.csv");
    std::fs::write(&f, format!("rt,y\n{}", gauss_csv())).unwrap();
    let p = f.to_str().unwrap();
    for (old, new, v) in [
        ("areaNoise", "area_noise", "3"),
        ("edgeFrac", "edge_frac", "0.2"),
        ("snMin", "sn_min", "1000"),
        ("minHalf", "min_half", "0.5"),
    ] {
        for at in ["2.25", "6"] {
            let a = run(args(&[p, "--at", at, "--set", &format!("{old}={v}")]));
            let b = run(args(&[p, "--at", at, "--set", &format!("{new}={v}")]));
            assert_eq!((a.code, a.stdout), (b.code, b.stdout), "{old}");
        }
    }
    let _ = std::fs::remove_file(f);
}

#[test]
fn the_command_name_has_an_alias() {
    let f = std::env::temp_dir().join("mzlab_peak_edges_cmd.csv");
    std::fs::write(&f, format!("rt,y\n{}", gauss_csv())).unwrap();
    let p = f.to_str().unwrap().to_string();
    for extra in [
        vec!["--at", "2.25"],
        vec!["--at", "6", "--set", "snMin=1000"],
        vec!["--help"],
        vec![],
    ] {
        let go = |name: &str| {
            let mut a = vec![name.to_string(), p.clone()];
            a.extend(extra.iter().map(|s| s.to_string()));
            crate::execute(&a)
        };
        let (a, b) = (go("peaks-edges"), go("peak-edges"));
        assert_eq!((a.code, a.stdout), (b.code, b.stdout), "{extra:?}");
    }
    let _ = std::fs::remove_file(f);
}

#[test]
fn dialog_numbers_at_their_defaults_are_the_engine_defaults() {
    use mzlab_core::peakq_lr::LrParams;
    let d = LrParams::default();
    assert_eq!(
        LrParams::from_dialog(&[d.win, d.edge_frac, d.sn_min, d.multi, d.tail]),
        Some(d)
    );
    assert_eq!(LrParams::from_dialog(&[1.0, 2.0]), None);
}
