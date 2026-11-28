//! `mzlab-cli peak table --ions`: the ion columns of the table for traces of a known factor, one per scan type (ems, epi, q1, sim, other), for the
//! factor a `trace xic --ions` trace brings, and for the SIM `.wiff` of `tools/wiff_sim.py`; the bytes without `--ions` stay, the usage errors exit 2.

use std::path::{Path, PathBuf};

use crate::execute;

/// A trace of counts (every point a whole number of 20 cps), a peak at 2 min.
fn counts() -> String {
    let (mut t, mut y) = (Vec::new(), Vec::new());
    let mut s: u64 = 11;
    for i in 0..400 {
        let x = i as f64 * 0.01;
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let bg = ((s >> 33) % 3) as f64;
        let peak = (40.0 * (-((x - 2.0) / 0.1).powi(2)).exp()).round();
        t.push(format!("{x:.2}"));
        y.push(format!("{}", (bg + peak) * 20.0));
    }
    format!(
        "{{\"q1\":100.1,\"rt\":[{}],\"y\":[{}]}}",
        t.join(","),
        y.join(",")
    )
}

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_cli_ions_{tag}_{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn run(d: &Path, extra: &[&str]) -> crate::Outcome {
    let (tp, wp) = (d.join("traces.json"), d.join("windows.json"));
    std::fs::write(
        &tp,
        format!("[{{\"name\":\"a.mzML\",\"transitions\":[{}]}}]", counts()),
    )
    .unwrap();
    std::fs::write(&wp, "[{\"file\":0,\"tr\":0,\"a\":1.7,\"b\":2.3}]").unwrap();
    let mut a: Vec<String> = ["peak", "table", "--traces"].map(String::from).into();
    a.push(tp.display().to_string());
    a.push("--windows".into());
    a.push(wp.display().to_string());
    a.extend(extra.iter().map(|s| s.to_string()));
    execute(&a)
}

fn cells(o: &crate::Outcome) -> (Vec<String>, Vec<String>) {
    assert_eq!(o.code, 0, "{}", o.stderr);
    let t = String::from_utf8(o.stdout.clone()).unwrap();
    let h = t
        .lines()
        .next()
        .unwrap()
        .split(',')
        .map(String::from)
        .collect();
    (
        h,
        t.lines()
            .nth(1)
            .unwrap()
            .split(',')
            .map(String::from)
            .collect(),
    )
}

fn at(c: &(Vec<String>, Vec<String>), name: &str) -> String {
    c.1[c.0.iter().position(|h| h == name).unwrap()].clone()
}

#[test]
fn each_scan_type_gives_its_factor_in_the_columns_and_the_log() {
    let d = dir("kinds");
    let cases: [(&[&str], &str, &str, &str); 5] = [
        (
            &["--scan-type", "ems", "--set", "fill_ms=100"],
            "0.1",
            "method",
            "ems",
        ),
        (
            &["--scan-type", "epi", "--set", "fill_ms=250"],
            "0.25",
            "method",
            "epi",
        ),
        (
            &[
                "--scan-type",
                "q1",
                "--set",
                "scan_s=2,step_da=0.1,mass_lo=100,mass_hi=300",
            ],
            "0.001",
            "method",
            "q1",
        ),
        (
            &["--scan-type", "sim", "--set", "dwell_ms=50"],
            "0.05",
            "method",
            "sim",
        ),
        (&["--scan-type", "other"], "0.05", "data", "other"),
    ];
    for (flags, factor, source, kind) in cases {
        let mut a = vec!["--ions"];
        a.extend(flags);
        let o = run(&d, &a);
        let c = cells(&o);
        assert_eq!(at(&c, "ions_per_cps"), factor, "{kind}");
        assert_eq!(at(&c, "ion_source"), source, "{kind}");
        assert_eq!(at(&c, "ion_scan_type"), kind);
        assert!(
            !at(&c, "area_ions").is_empty() && !at(&c, "sn_ions").is_empty(),
            "{kind}"
        );
        // the choice is logged, one line per sample
        assert!(
            o.stderr.contains(&format!("ion scale: a: {kind} scans")),
            "{}",
            o.stderr
        );
    }
}

#[test]
fn the_cps_columns_are_the_bytes_without_ions() {
    let d = dir("bytes");
    let plain = String::from_utf8(run(&d, &[]).stdout).unwrap();
    let with = String::from_utf8(
        run(
            &d,
            &["--ions", "--scan-type", "ems", "--set", "fill_ms=100"],
        )
        .stdout,
    )
    .unwrap();
    for (a, b) in plain.lines().zip(with.lines()) {
        assert!(b.starts_with(a));
    }
    assert!(with
        .lines()
        .next()
        .unwrap()
        .ends_with(mzlab_core::manint::IONS_HEAD.trim_start_matches(',')));
    // the engine writes the same text (the site and the app call it)
    let files = format!("[{{\"name\":\"a.mzML\",\"transitions\":[{}]}}]", counts());
    let doc = "{\"samples\":[{\"scan_type\":\"ems\",\"scan_from\":\"scan type given\",\"fill_ms\":100,\"traces\":[]}]}";
    let want = mzlab_core::manint::table_text_ions(
        &files,
        "[{\"file\":0,\"tr\":0,\"a\":1.7,\"b\":2.3}]",
        false,
        doc,
    )
    .unwrap();
    assert_eq!(with, want);
}

#[test]
fn a_method_value_or_a_scan_type_without_ions_is_a_usage_error() {
    let d = dir("usage");
    for extra in [&["--scan-type", "ems"][..], &["--set", "fill_ms=100"][..]] {
        let o = run(&d, extra);
        assert_eq!(o.code, 2);
        assert!(
            String::from_utf8_lossy(&o.stdout).contains("err.cli.peaktable.needsIons")
                || o.stderr.contains("err.cli.peaktable.needsIons")
        );
    }
    assert_eq!(run(&d, &["--ions", "--scan-type", "qqq"]).code, 2);
    assert_eq!(run(&d, &["--ions", "--set", "bogus=1"]).code, 2);
}

#[test]
fn the_factor_a_trace_xic_trace_brings_is_used_and_says_trace() {
    let d = dir("evidence");
    let set = format!(
        "{{\"schema\":\"trace-set/1\",\"file\":\"a.mzML\",\"traces\":[{{\"id\":\"xic\",\"points\":[{}],\"evidence\":{{\"mz\":100.1,\"ion_scale\":{{\"scan_type\":\"epi\",\"ions_per_cps\":0.2}}}}}}]}}",
        (0..300).map(|i| format!("[{:.2},{}]", i as f64 * 0.01, 10.0 + 900.0 * (-0.5 * ((i as f64 * 0.01 - 1.5) / 0.1).powi(2)).exp())).collect::<Vec<_>>().join(",")
    );
    let (tp, wp) = (d.join("set.json"), d.join("w.json"));
    std::fs::write(&tp, set).unwrap();
    std::fs::write(&wp, "[{\"file\":0,\"tr\":0,\"a\":1.2,\"b\":1.8}]").unwrap();
    let a: Vec<String> = [
        "peak",
        "table",
        "--traces",
        &tp.display().to_string(),
        "--windows",
        &wp.display().to_string(),
        "--ions",
        "--json",
    ]
    .map(String::from)
    .into();
    let o = execute(&a);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let j = String::from_utf8(o.stdout).unwrap();
    assert!(
        j.contains("\"ion_source\":\"trace\"")
            && j.contains("\"ions_per_cps\":\"0.2\"")
            && j.contains("\"ion_scan_type\":\"epi\""),
        "{j}"
    );
    assert!(
        j.contains("\"ion_scale\":[{\"sample\":\"a\",\"scan_type\":\"epi\""),
        "{j}"
    );
    assert!(
        o.stderr
            .contains("epi scans (the ion_scale of the traces (trace xic --ions))"),
        "{}",
        o.stderr
    );
}

#[test]
fn the_sim_wiff_reads_its_dwell_scan_type_as_sim() {
    let d = dir("wiff");
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/wiff_sim.py");
    let ok = std::process::Command::new("python3")
        .arg(script)
        .arg(&d)
        .output()
        .ok()
        .filter(|o| o.status.success());
    if ok.is_none() {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 not available"
        );
        eprintln!("SKIP peak table --ions: python3 not available");
        return;
    }
    let wiff = d.join("sim3.wiff");
    let wp = d.join("windows.json");
    std::fs::write(&wp, "[{\"file\":3,\"tr\":0,\"a\":1.97,\"b\":2.27}]").unwrap();
    let a: Vec<String> = [
        "peak",
        "table",
        &wiff.display().to_string(),
        "--windows",
        &wp.display().to_string(),
        "--ions",
        "--set",
        "dwell_ms=40",
    ]
    .map(String::from)
    .into();
    let o = execute(&a);
    let c = cells(&o);
    assert_eq!(at(&c, "ion_scan_type"), "sim");
    assert_eq!(at(&c, "ions_per_cps"), "0.04");
    assert_eq!(at(&c, "ion_source"), "method");
    assert!(o.stderr.contains("SIM / MRM chromatograms"), "{}", o.stderr);
}
