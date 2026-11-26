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
    let o = run(args(&[p, "--at", "6", "--set", "sn_min=1000"]));
    assert_eq!(
        String::from_utf8(o.stdout).unwrap().trim(),
        "{\"edges\":null}"
    );
    assert_ne!(run(args(&[p, "--at", "2", "--set", "bogus=1"])).code, 0);
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

#[test]
fn click_mode_gives_the_core_text_refuses_with_exit_1_and_force_keeps_it() {
    use mzlab_core::peakq_click::{lr_click, ClickGuard};
    use mzlab_core::peakq_lr::LrParams;
    let f = std::env::temp_dir().join("mzlab_peaks_click_trace.csv");
    std::fs::write(&f, format!("rt,y\n{}", gauss_csv())).unwrap();
    let p = f.to_str().unwrap();
    let ok = run(args(&[p, "--at", "2.3", "--set", "click=1"]));
    let (x, y): (Vec<f64>, Vec<f64>) = (0..400)
        .map(|i| {
            let t = i as f64 * 0.02;
            (
                t,
                100.0 + 5000.0 * (-((t - 2.25) / 0.06).powi(2) / 2.0).exp(),
            )
        })
        .unzip();
    let want = lr_click(
        &x,
        &y,
        2.3,
        false,
        &LrParams::default(),
        &ClickGuard::default(),
    )
    .to_json();
    assert_eq!(
        (ok.code, String::from_utf8(ok.stdout).unwrap()),
        (0, format!("{want}\n"))
    );
    assert!(
        ok.stderr.starts_with("note: click at 2.3 min: apex 2.24"),
        "{}",
        ok.stderr
    );
    // flat stretch: no apex, so the click is refused (exit 1, data still on stdout), and --set force=1 keeps the window
    let no = run(args(&[p, "--at", "5.5", "--set", "click=1"]));
    let v: serde_json::Value = serde_json::from_slice(&no.stdout).unwrap();
    assert_eq!(
        (no.code, v["ok"].as_bool(), v["refused"][0].as_str()),
        (1, Some(false), Some("no_peak"))
    );
    assert!(no.stderr.contains("--set force=1"), "{}", no.stderr);
    let forced = run(args(&[
        p, "--at", "5.5", "--set", "click=1", "--set", "force=1",
    ]));
    let v: serde_json::Value = serde_json::from_slice(&forced.stdout).unwrap();
    assert_eq!(
        (forced.code, v["ok"].as_bool(), v["forced"].as_bool()),
        (0, Some(true), Some(true))
    );
    assert_eq!(v["params"]["min_points"], 5);
    assert_ne!(run(args(&[p, "--at", "2", "--set", "click=x"])).code, 0);
    let _ = std::fs::remove_file(f);
}
