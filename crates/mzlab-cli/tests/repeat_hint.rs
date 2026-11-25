//! The hint of a loop of calls through the real binary: the tenth `peak auto-lr` from one parent says on stderr which one call replaces the loop;
//! stdout, the exit code and the first nine calls are untouched, and `MZLAB_NO_HINTS` silences it.

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
            && tenth.1.contains("(note.cli.repeat)"),
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
