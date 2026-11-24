//! `mzlab-cli peak table --traces PATH`: traces the caller supplies (a chromatogram, an XIC) give the five extra columns, the same bytes as the engine
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
    let mut a = vec!["peak".to_string(), "table".to_string()];
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
        "--traces",
        &tp.display().to_string(),
        "--windows",
        &wp.display().to_string(),
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
        "--traces",
        &tp.display().to_string(),
        "--windows",
        &wp.display().to_string(),
    ]);
    assert_ne!(bad.code, 0);
}

/// The help names the optional columns exactly as the real header ends: the lists come from the same constants.
fn header_and_help_agree(want: &str) {
    let header = want.lines().next().unwrap();
    let ext = format!(
        "{}{}{}{}",
        mzlab_core::manint::EXT_HEAD,
        mzlab_core::manint::SN_HEAD,
        mzlab_core::manint::FIG_HEAD,
        mzlab_core::manint::QUALITY_HEAD
    );
    assert_eq!(
        header,
        format!("{}{ext}", mzlab_core::manint::HEAD.trim_end())
    );
    let h = crate::manual_table_cmd::help();
    for e in [
        mzlab_core::manint::SN_HEAD,
        mzlab_core::manint::FIG_HEAD,
        mzlab_core::manint::QUALITY_HEAD,
    ] {
        assert!(
            header.contains(e.trim_start_matches(',')) && h.contains(e.trim_start_matches(','))
        );
    }
}

#[test]
fn columns_and_json_are_the_engine_table_in_two_shapes() {
    let d = std::env::temp_dir().join(format!("mzlab_cli_cols_{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let files = format!(
        "[{{\"name\":\"a.mzML\",\"transitions\":[{}]}}]",
        trace("XIC", "m/z 100.1")
    );
    let windows = "[{\"file\":0,\"tr\":0,\"a\":1.2,\"b\":1.8,\"noise\":[0.1,0.9]}]";
    let (tp, wp) = (d.join("traces.json"), d.join("windows.json"));
    std::fs::write(&tp, &files).unwrap();
    std::fs::write(&wp, windows).unwrap();
    let base = [
        "--traces".to_string(),
        tp.display().to_string(),
        "--windows".to_string(),
        wp.display().to_string(),
        "--columns".to_string(),
        "sn,fig,quality".to_string(),
    ];
    let go = |extra: &[&str]| {
        let mut a: Vec<&str> = base.iter().map(String::as_str).collect();
        a.extend(extra);
        run(&a)
    };
    let o = go(&[]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let doc = format!("{{\"windows\":{windows},\"columns\":[\"sn\",\"fig\",\"quality\"]}}");
    let want = mzlab_core::manint::table_text(&files, &doc, false).unwrap();
    assert_eq!(String::from_utf8(o.stdout).unwrap(), want);
    header_and_help_agree(&want);
    // the JSON keeps the header's names as keys and the CSV's text in each cell
    let j = go(&["--json"]);
    assert_eq!(j.code, 0, "{}", j.stderr);
    let v: serde_json::Value = serde_json::from_slice(&j.stdout).unwrap();
    let cols: Vec<&str> = v["columns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c.as_str().unwrap())
        .collect();
    assert_eq!(
        cols,
        want.lines().next().unwrap().split(',').collect::<Vec<_>>()
    );
    let row = want.lines().nth(1).unwrap().split(',').collect::<Vec<_>>();
    assert_eq!(
        v["rows"][0]["tailing"],
        row[cols.iter().position(|c| *c == "tailing").unwrap()]
    );
    // an unknown group is refused with the engine's message, and --json goes with auto
    let bad = run(&[
        "--traces",
        &tp.display().to_string(),
        "--windows",
        &wp.display().to_string(),
        "--columns",
        "nope",
    ]);
    assert_eq!(bad.code, 2);
    // --json with --auto is accepted: the answer is JSON already
    let auto = go(&["--auto", "--json"]);
    assert_eq!(auto.code, 0, "{}", auto.stderr);
    assert!(String::from_utf8(auto.stdout)
        .unwrap()
        .starts_with("{\"tol\":"));
}

/// An empty `--columns ""` is a usage error, like an empty `--split ""`, not an empty list that silently adds nothing.
#[test]
fn an_empty_columns_list_is_refused() {
    let o = run(&["--traces", "x.json", "--windows", "w.json", "--columns", ""]);
    assert_eq!(o.code, 2, "{}", o.stderr);
    assert!(o.stderr.contains("--columns needs a list"), "{}", o.stderr);
}

/// `--columns state` adds the value-state word, the same bytes as the engine, named in the help; a window the user gave is `forced`, the engine's own
/// peaks (`--auto` windows fed back) are `detected`.
#[test]
fn the_state_column_is_the_engine_word_and_the_help_names_it() {
    let d = std::env::temp_dir().join(format!("mzlab_cli_state_{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let files = format!(
        "[{{\"name\":\"a.mzML\",\"transitions\":[{}]}}]",
        trace("XIC", "m/z 100.1")
    );
    let windows = "[{\"file\":0,\"tr\":0,\"a\":1.2,\"b\":1.8}]";
    let (tp, wp) = (d.join("traces.json"), d.join("windows.json"));
    std::fs::write(&tp, &files).unwrap();
    std::fs::write(&wp, windows).unwrap();
    let (t, w) = (tp.display().to_string(), wp.display().to_string());
    let o = run(&["--traces", &t, "--windows", &w, "--columns", "state"]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let doc = format!("{{\"windows\":{windows},\"columns\":[\"state\"]}}");
    let want = mzlab_core::manint::table_text(&files, &doc, false).unwrap();
    let out = String::from_utf8(o.stdout).unwrap();
    assert_eq!(out, want);
    let (head, row) = (want.lines().next().unwrap(), want.lines().nth(1).unwrap());
    assert!(head.ends_with(mzlab_core::manint::STATE_HEAD), "{head}");
    assert!(row.ends_with(",forced"), "{row}");
    assert!(crate::manual_table_cmd::help().contains("state"));
    let _ = std::fs::remove_dir_all(&d);
}
