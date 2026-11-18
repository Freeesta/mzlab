//! `mzlab-cli manual-table --set traces=PATH`: traces the caller supplies (a chromatogram, an XIC) give the five extra columns, the same bytes as the engine
//! on the same text, and a request with both files and traces is refused.

use crate::execute;

fn trace(panel: &str, label: &str) -> String {
    let (mut t, mut y) = (Vec::new(), Vec::new());
    for i in 0..300 {
        let x = i as f64 * 0.01;
        t.push(format!("{x:.3}"));
        y.push(format!(
            "{:.1}",
            10.0 + 900.0 * (-0.5 * ((x - 1.5) / 0.1).powi(2)).exp()
        ));
    }
    format!(
        "{{\"panel\":\"{panel}\",\"label\":\"{label}\",\"rt\":[{}],\"y\":[{}]}}",
        t.join(","),
        y.join(",")
    )
}

fn run(args: &[&str]) -> crate::Outcome {
    let mut a = vec!["manual-table".to_string()];
    a.extend(args.iter().map(|s| s.to_string()));
    execute(&a)
}

#[test]
fn traces_give_the_columns_of_the_engine_byte_for_byte() {
    let d = std::env::temp_dir().join(format!("mzlab_cli_traces_{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let files = format!(
        "[{{\"name\":\"a.mzML\",\"conc\":5,\"conc_unit\":\"ppm\",\"transitions\":[{}]}}]",
        trace("XIC", "m/z 100.1")
    );
    let windows = "[{\"file\":0,\"tr\":0,\"a\":1.2,\"b\":1.8,\"corr\":\"drift\"}]";
    let (tp, wp) = (d.join("traces.json"), d.join("windows.json"));
    std::fs::write(&tp, &files).unwrap();
    std::fs::write(&wp, windows).unwrap();
    let o = run(&[
        "--set",
        &format!("traces={}", tp.display()),
        "--set",
        &format!("windows={}", wp.display()),
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let want = mzlab_core::manint::table_text(&files, windows, false).unwrap();
    assert_eq!(String::from_utf8(o.stdout).unwrap(), want);
    assert!(want
        .lines()
        .next()
        .unwrap()
        .ends_with(mzlab_core::manint::EXT_HEAD.trim_start_matches(',')));
    let bad = run(&[
        "x.mzML",
        "--set",
        &format!("traces={}", tp.display()),
        "--set",
        &format!("windows={}", wp.display()),
    ]);
    assert_ne!(bad.code, 0);
}
