//! `mzlab-cli peak table`: the `sample_id` / `file` columns tie every row to its sample on the SIM `.wiff` of `tools/wiff_sim.py` (a blank among the
//! samples), `--max-shift` keeps a followed window where it was (`too_far`), and a `.wiff` pair that grows while it is opened ends with exit 7.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::execute;

fn fixture(tag: &str) -> Option<PathBuf> {
    let d = std::env::temp_dir().join(format!("mzlab_cli_guard_{tag}_{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/wiff_sim.py");
    let ok = Command::new("python3")
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
        eprintln!("SKIP peak table guard: python3 not available");
        return None;
    }
    Some(d)
}

fn run(args: &[&str]) -> crate::Outcome {
    let mut a = vec!["peak".to_string(), "table".to_string()];
    a.extend(args.iter().map(|s| s.to_string()));
    execute(&a)
}

fn cells(o: &crate::Outcome) -> Vec<Vec<String>> {
    assert_eq!(o.code, 0, "{}", o.stderr);
    String::from_utf8(o.stdout.clone())
        .unwrap()
        .lines()
        .map(|l| l.split(',').map(str::to_string).collect())
        .collect()
}

/// The windows on samples 3 and 1 of `sim3` (the blank is sample 0): the rows say 1 and 3, whatever their order, in the CSV and in the JSON.
#[test]
fn every_row_says_which_sample_it_is_even_with_a_blank_before_it() {
    let Some(d) = fixture("ids") else { return };
    let w = d.join("w.json");
    std::fs::write(
        &w,
        r#"[{"file":3,"tr":0,"a":1.97,"b":2.27},{"file":1,"tr":0,"a":1.89,"b":2.19}]"#,
    )
    .unwrap();
    let (wiff, wp) = (
        d.join("sim3.wiff").display().to_string(),
        w.display().to_string(),
    );
    let r = cells(&run(&[&wiff, "--windows", &wp]));
    let (i, n) = (
        r[0].iter().position(|c| c == "sample_id").unwrap(),
        r[0].iter().position(|c| c == "file").unwrap(),
    );
    let got: Vec<(&str, &str, &str)> = r[1..]
        .iter()
        .map(|x| (x[1].as_str(), x[i].as_str(), x[n].as_str()))
        .collect();
    assert_eq!(
        got,
        [
            ("Sample_A", "1", "Sample_A.mzML"),
            ("Sample_C", "3", "Sample_C.mzML")
        ]
    );
    let j = String::from_utf8(run(&[&wiff, "--windows", &wp, "--json"]).stdout).unwrap();
    assert!(
        j.contains("\"sample_id\":\"3\",\"file\":\"Sample_C.mzML\""),
        "{j}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// A trace set where the second sample's peak lies 0.30 min from the source's: followed freely it moves the window, with `--max-shift 0.2` (or the rigid 0)
/// it stays and says `too_far` with the refused shift; `--shifts` prints the same decision; a negative limit is a usage error.
#[test]
fn max_shift_keeps_a_window_where_it_was_and_writes_the_shift_it_refused() {
    let d = std::env::temp_dir().join(format!("mzlab_cli_maxshift_{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let t: Vec<String> = (0..500)
        .map(|i| format!("{:.2}", i as f64 * 0.02))
        .collect();
    let y = |c: f64| -> String {
        (0..500)
            .map(|i| {
                format!(
                    "{:.1}",
                    50.0 + 1e5 * (-0.5 * ((i as f64 * 0.02 - c) / 0.05).powi(2)).exp()
                )
            })
            .collect::<Vec<_>>()
            .join(",")
    };
    let one = |n: &str, c: f64| {
        format!(
            "{{\"name\":\"{n}.mzML\",\"transitions\":[{{\"q1\":253.1,\"rt\":[{}],\"y\":[{}]}}]}}",
            t.join(","),
            y(c)
        )
    };
    let (tp, wp) = (d.join("t.json"), d.join("w.json"));
    std::fs::write(
        &tp,
        format!(
            "[{},{},{}]",
            one("s0", 5.0),
            one("s1", 5.3),
            one("s2", 5.02)
        ),
    )
    .unwrap();
    std::fs::write(
        &wp,
        r#"[{"file":0,"tr":0,"a":4.85,"b":5.15,"follow":true}]"#,
    )
    .unwrap();
    let (tp, wp) = (tp.display().to_string(), wp.display().to_string());
    let row = |extra: &[&str]| -> Vec<String> {
        let mut a = vec!["--traces", &tp, "--windows", &wp];
        a.extend(extra);
        let r = cells(&run(&a));
        let h = &r[0];
        let at = |n: &str| h.iter().position(|c| c == n).unwrap();
        let x = r[1..].iter().find(|x| x[1] == "s1").unwrap();
        ["origin", "shift_min", "start_min", "end_min"]
            .iter()
            .map(|n| x[at(n)].clone())
            .collect()
    };
    assert_eq!(row(&[]), ["shifted", "0.300", "5.150", "5.450"]);
    for m in ["0.2", "0"] {
        assert_eq!(
            row(&["--max-shift", m]),
            ["too_far", "0.300", "4.850", "5.150"],
            "{m}"
        );
    }
    assert_eq!(row(&["--max-shift", "0.5"])[0], "shifted");
    let s = run(&[
        "--traces",
        &tp,
        "--windows",
        &wp,
        "--shifts",
        "--max-shift",
        "0.2",
    ]);
    let s = String::from_utf8(s.stdout).unwrap();
    assert!(
        s.contains("\"origin\":\"too_far\"") && s.contains("\"found_shift\":0.29"),
        "{s}"
    );
    let bad = run(&["--traces", &tp, "--windows", &wp, "--max-shift", "-1"]);
    assert_eq!(bad.code, 2, "{}", bad.stderr);
    let _ = std::fs::remove_dir_all(&d);
}

/// A `.wiff.scan` that keeps growing while the pair is opened is refused with exit 7 and `err.file.wiffGrowing` (sizes and interval in its parameters),
/// by `peak table` and by `sample list`; once the writer is done the same pair opens.
#[test]
fn a_pair_that_grows_while_it_is_opened_ends_with_exit_7() {
    let Some(d) = fixture("grow") else { return };
    let live = d.join("live");
    std::fs::create_dir_all(&live).unwrap();
    std::fs::copy(d.join("sim3.wiff"), live.join("L.wiff")).unwrap();
    let data = std::fs::read(d.join("sim3.wiff.scan")).unwrap();
    let scan = live.join("L.wiff.scan");
    std::fs::write(&scan, b"").unwrap();
    let writer = {
        let scan = scan.clone();
        std::thread::spawn(move || {
            use std::io::Write;
            let mut f = std::fs::OpenOptions::new().append(true).open(scan).unwrap();
            for c in data.chunks(4000) {
                f.write_all(c).unwrap();
                f.flush().unwrap();
                std::thread::sleep(std::time::Duration::from_millis(40));
            }
        })
    };
    std::thread::sleep(std::time::Duration::from_millis(100));
    let w = d.join("w.json");
    std::fs::write(&w, r#"[{"file":1,"tr":0,"a":1.89,"b":2.19}]"#).unwrap();
    let (lw, wp) = (
        live.join("L.wiff").display().to_string(),
        w.display().to_string(),
    );
    let o = run(&[&lw, "--windows", &wp, "--json"]);
    assert_eq!(o.code, 7, "{}", o.stderr);
    let out = String::from_utf8(o.stdout).unwrap();
    assert!(
        out.contains("\"error_key\":\"err.file.wiffGrowing\"") && out.contains("\"code\":7"),
        "{out}"
    );
    assert!(o.stderr.contains("still being written"), "{}", o.stderr);
    let l = execute(&["sample".into(), "list".into(), lw.clone(), "--json".into()]);
    assert_eq!(l.code, 7, "{}", l.stderr);
    writer.join().unwrap();
    // finished: the size no longer changes, the pair opens
    let done = run(&[&lw, "--windows", &wp]);
    assert_eq!(done.code, 0, "{}", done.stderr);
    let _ = std::fs::remove_dir_all(&d);
}
