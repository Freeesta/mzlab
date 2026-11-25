//! The hint of a loop of calls through the real binary: the tenth `peak auto-lr`, and the tenth `flow run` of a Dig plan, from one parent say once on stderr
//! which one call replaces the loop, also when the calls run in parallel; stdout, the exit code and the first nine calls are untouched, and `MZLAB_NO_HINTS`
//! silences it.

use std::process::Command;

fn trace(dir: &std::path::Path) -> String {
    let rows: String = (0..60)
        .map(|i| {
            format!(
                "{},{}\n",
                1.0 + 0.01 * f64::from(i),
                20.0 + 1e4 * (-0.5 * ((f64::from(i) - 30.0) / 4.0).powi(2)).exp()
            )
        })
        .collect();
    let f = dir.join("t.csv");
    std::fs::write(&f, rows).unwrap();
    f.display().to_string()
}

fn call(state: &std::path::Path, file: &str, silent: bool) -> (Vec<u8>, String, i32) {
    let mut c = Command::new(env!("CARGO_BIN_EXE_mzlab-cli"));
    c.args(["peak", "auto-lr", file])
        .env("MZLAB_STATE_DIR", state)
        .env_remove("MZLAB_NO_HINTS");
    if silent {
        c.env("MZLAB_NO_HINTS", "1");
    }
    let o = c.output().unwrap();
    (
        o.stdout,
        String::from_utf8_lossy(&o.stderr).into_owned(),
        o.status.code().unwrap_or(-1),
    )
}

#[test]
fn the_tenth_call_of_a_loop_says_the_one_call_and_changes_nothing_else() {
    let d = std::env::temp_dir().join(format!("mzlab_repeat_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let (file, state) = (trace(&d), d.join("state"));
    let first = call(&state, &file, false);
    assert_eq!(first.2, 0, "{}", first.1);
    for k in 2..=9 {
        let o = call(&state, &file, false);
        assert!(!o.1.contains("note.cli.repeat"), "call {k}: {}", o.1);
        assert_eq!(o.0, first.0);
    }
    let tenth = call(&state, &file, false);
    assert_eq!(tenth.2, 0);
    assert_eq!(
        tenth.0, first.0,
        "stdout is the same bytes with and without the hint"
    );
    assert!(
        tenth
            .1
            .contains("quant targets FILE... --targets TARGETS.csv")
            && tenth.1.contains("(note.cli.repeat.targets)"),
        "{}",
        tenth.1
    );
    assert!(
        !call(&state, &file, false).1.contains("note.cli.repeat"),
        "said once, not at every call"
    );
    // silenced: nothing is said, however many calls
    let d2 = d.join("state2");
    let said: Vec<String> = (0..12).map(|_| call(&d2, &file, true).1).collect();
    assert!(
        said.iter().all(|s| !s.contains("note.cli.repeat")),
        "{said:?}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// `flow run` of a plan whose steps include `tp.dig_lr` is the other looped command: the tenth run from one parent names the one run of the series.
#[test]
fn the_tenth_flow_run_of_a_dig_plan_says_the_one_run_of_the_series() {
    let d = std::env::temp_dir().join(format!("mzlab_repeat_dig_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let plan = d.join("dig.json");
    std::fs::write(&plan, r#"{"steps":[{"id":"d","step":"tp.dig_lr@1"}]}"#).unwrap();
    let run = |state: &std::path::Path| {
        let o = Command::new(env!("CARGO_BIN_EXE_mzlab-cli"))
            .args(["flow", "run"])
            .arg(&plan)
            .env("MZLAB_STATE_DIR", state)
            .env_remove("MZLAB_NO_HINTS")
            .output()
            .unwrap();
        (
            o.stdout,
            String::from_utf8_lossy(&o.stderr).into_owned(),
            o.status.code(),
        )
    };
    let state = d.join("state");
    let all: Vec<_> = (1..=11).map(|_| run(&state)).collect();
    for (k, o) in all.iter().enumerate() {
        assert_eq!(
            o.1.contains("(note.cli.repeat.dig)"),
            k == 9,
            "call {}: {}",
            k + 1,
            o.1
        );
        assert_eq!(
            (&o.0, o.2),
            (&all[0].0, all[0].2),
            "stdout and exit code are the same"
        );
    }
    assert!(all[9].1.contains("tp.dig_lr"), "{}", all[9].1);
    let _ = std::fs::remove_dir_all(&d);
}

/// `xargs -P`: twelve calls started together by one parent; none of the counts is lost and exactly one of them says the hint.
#[test]
fn calls_started_together_say_the_hint_exactly_once() {
    let d = std::env::temp_dir().join(format!("mzlab_repeat_par_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let (file, state) = (trace(&d), d.join("state"));
    let children: Vec<_> = (0..12)
        .map(|_| {
            Command::new(env!("CARGO_BIN_EXE_mzlab-cli"))
                .args(["peak", "auto-lr", &file])
                .env("MZLAB_STATE_DIR", &state)
                .env_remove("MZLAB_NO_HINTS")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .unwrap()
        })
        .collect();
    let said = children
        .into_iter()
        .map(|c| c.wait_with_output().unwrap())
        .filter(|o| String::from_utf8_lossy(&o.stderr).contains("(note.cli.repeat.targets)"))
        .count();
    assert_eq!(said, 1);
    let _ = std::fs::remove_dir_all(&d);
}
