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

/// The golden of `mzlab-core` (tests/data/click_golden.*): `peak edges --set click=1` prints its exact bytes for every case; a refused one exits 1 (the one
/// pinned code of a refused window: the class is in `refused[]`), a kept or forced one exits 0.
#[test]
fn click_mode_matches_the_golden_bytes_and_exit_codes() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../mzlab-core/tests/data");
    let golden = std::fs::read_to_string(format!("{dir}/click_golden.txt")).unwrap();
    let file = format!("{dir}/click_golden.csv");
    let mut lines = golden.lines();
    let mut cases = 0;
    while let (Some(head), Some(want)) = (lines.next(), lines.next()) {
        let f: Vec<&str> = head.split(' ').collect();
        let at = f[0].trim_start_matches("at=");
        let mut a = vec![file.as_str(), "--at", at, "--set", "click=1"];
        if f[1] == "mrm=1" {
            a.push("--mrm");
        }
        if f[2] == "force=1" {
            a.extend(["--set", "force=1"]);
        }
        let o = run(args(&a));
        assert_eq!(
            String::from_utf8(o.stdout).unwrap(),
            format!("{want}\n"),
            "{head}"
        );
        assert_eq!(
            o.code,
            if f[3] == "ok=true" { 0 } else { 1 },
            "{head}: {}",
            o.stderr
        );
        cases += 1;
    }
    assert_eq!(cases, 5);
}

/// A refusal ends with exit 1 whatever its class and names the class in `refused[]`: the exit table is one for all commands (exit.rs), so the class is data, not a
/// new code. No peak (a flat trace, a trace too short), too few points and S/N too low, each on its own.
#[test]
fn a_refusal_exits_1_and_names_its_class() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../mzlab-core/tests/data");
    let big = format!("{dir}/click_golden.csv");
    let tmp = std::env::temp_dir().join("mzlab_click_classes");
    std::fs::create_dir_all(&tmp).unwrap();
    let flat = tmp.join("flat.csv");
    std::fs::write(
        &flat,
        (0..60)
            .map(|i| format!("{},{}\n", i as f64 * 0.025, 100 + i % 3))
            .collect::<String>(),
    )
    .unwrap();
    let tiny = tmp.join("tiny.csv");
    std::fs::write(&tiny, "0,1\n0.1,2\n0.2,1\n").unwrap();
    let cases: [(&str, &str, Vec<&str>); 4] = [
        (flat.to_str().unwrap(), "no_peak", vec![]),
        (tiny.to_str().unwrap(), "no_peak", vec![]),
        (&big, "points", vec!["--set", "min_points=40"]),
        (&big, "sn", vec!["--set", "min_sn=1000"]),
    ];
    for (file, class, extra) in cases {
        let mut a = vec![
            file,
            "--at",
            if file == big { "1.02" } else { "0.6" },
            "--set",
            "click=1",
        ];
        a.extend(extra);
        let o = run(args(&a));
        let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
        assert_eq!(o.code, 1, "{class}: {}", o.stderr);
        assert_eq!(
            (v["ok"].as_bool(), v["refused"][0].as_str()),
            (Some(false), Some(class)),
            "{v}"
        );
        assert!(o.stderr.contains("refused:"), "{}", o.stderr);
    }
    let _ = std::fs::remove_dir_all(tmp);
}
