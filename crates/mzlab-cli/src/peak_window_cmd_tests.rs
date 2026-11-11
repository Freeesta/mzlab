use super::run;
use mzlab_core::intwin::move_end_json;

fn args(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

/// A trace of 400 points 0.01 min apart (the intensities do not matter to the window).
fn trace_file(name: &str) -> (Vec<f64>, String) {
    let rt: Vec<f64> = (0..400).map(|i| f64::from(i) * 0.01).collect();
    let f = std::env::temp_dir().join(format!("{name}_{}", std::process::id()));
    let body: Vec<String> = rt.iter().map(|a| format!("{a},1")).collect();
    std::fs::write(&f, body.join("\n")).unwrap();
    (rt, f.to_str().unwrap().to_string())
}

fn out(o: crate::Outcome) -> String {
    assert_eq!(o.code, 0, "{}", o.stderr);
    String::from_utf8(o.stdout).unwrap()
}

#[test]
fn a_drag_and_a_step_are_the_text_of_the_page() {
    let (rt, p) = trace_file("mzlab_peak_window_trace.csv");
    let drag = out(run(args(&[
        &p, "--end", "1.7", "--other", "2.3", "--at", "1.655",
    ])));
    let want = move_end_json(&rt, 1.7, 2.3, Some(1.655), 0, None, None).unwrap();
    assert_eq!(drag, format!("{want}\n"));
    assert!(
        drag.starts_with("{\"t\":1.66") || drag.starts_with("{\"t\":1.65"),
        "{drag}"
    );
    let key = out(run(args(&[
        &p, "--end", "2.3", "--other", "1.7", "--step", "-5", "--min", "1.0",
    ])));
    let want = move_end_json(&rt, 2.3, 1.7, None, -5, Some(1.0), None).unwrap();
    assert_eq!(key, format!("{want}\n"));
    let stop = out(run(args(&[
        &p, "--end", "2.3", "--other", "1.7", "--at", "3.5", "--max", "2.5",
    ])));
    assert_eq!(
        stop,
        format!(
            "{}\n",
            move_end_json(&rt, 2.3, 1.7, Some(3.5), 0, None, Some(2.5)).unwrap()
        )
    );
    assert!(stop.contains("\"i\":250"), "{stop}");
    let _ = std::fs::remove_file(&p);
}

#[test]
fn it_needs_one_of_at_or_step() {
    let (_, p) = trace_file("mzlab_peak_window_bad.csv");
    for bad in [
        &[&p[..], "--end", "1", "--other", "2"][..],
        &[
            &p[..],
            "--end",
            "1",
            "--other",
            "2",
            "--at",
            "1",
            "--step",
            "1",
        ][..],
        &[&p[..], "--end", "1", "--other", "2", "--step", "1.5"][..],
        &[&p[..], "--end", "x", "--other", "2", "--at", "1"][..],
        &[&p[..], "--end", "0", "--other", "0", "--at", "1"][..],
    ] {
        assert_ne!(run(args(bad)).code, 0, "{bad:?}");
    }
    assert!(out(run(args(&["--help"]))).contains("peak-window"));
    let _ = std::fs::remove_file(&p);
}
