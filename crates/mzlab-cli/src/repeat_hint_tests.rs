use super::*;

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_hint_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn args(a: &[&str]) -> Vec<String> {
    a.iter().map(|s| s.to_string()).collect()
}

fn nows() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[test]
fn the_hint_is_said_once_at_the_tenth_call_and_a_parent_is_counted_apart() {
    let d = dir("count");
    let (rule, now) = (&RULES[0], nows());
    let ticks: Vec<Tick> = (0..14)
        .map(|_| tick(&d, "7-1", rule, now).unwrap())
        .collect();
    let said: Vec<u64> = ticks.iter().filter(|t| t.say).map(|t| t.n).collect();
    assert_eq!(said, vec![FIRST], "once, at the tenth call");
    assert_eq!(ticks[13].n, 14);
    assert_eq!(
        tick(&d, "8-1", rule, now).unwrap(),
        Tick { n: 1, say: false },
        "another parent has its own count"
    );
    assert_eq!(
        tick(&d, "7-2", rule, now).unwrap().n,
        1,
        "a parent id reused after a restart has its own count"
    );
    assert_eq!(
        tick(&d, "7-1", &RULES[1], now).unwrap().n,
        1,
        "another command too"
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_long_pause_starts_again_and_old_state_is_swept() {
    let d = dir("gap");
    let (rule, now) = (&RULES[0], nows());
    (0..12).for_each(|_| {
        tick(&d, "7-1", rule, now).unwrap();
    });
    // the files of an old parent: untouched for more than STALE_S
    let old = d.join("99-1-peak_auto-lr");
    std::fs::write(&old, ".").unwrap();
    let long = std::time::SystemTime::now() - std::time::Duration::from_secs(STALE_S + 60);
    std::fs::File::options()
        .write(true)
        .open(&old)
        .unwrap()
        .set_modified(long)
        .unwrap();
    // the same parent, a little over GAP_S later: a new count, and the hint can be said again
    let t = tick(&d, "7-1", rule, now + GAP_S + 5).unwrap();
    assert_eq!(t, Tick { n: 1, say: false });
    assert!(
        !old.exists(),
        "state untouched for a day is deleted when a count starts"
    );
    // the files are dated by the system clock: the next calls are made "now" again
    (1..FIRST).for_each(|_| {
        tick(&d, "7-1", rule, now).unwrap();
    });
    assert!(d.join("7-1-peak_auto-lr.said").exists());
    let _ = std::fs::remove_dir_all(&d);
}

/// `xargs -P`: calls from one parent at the same moment lose no count and exactly one of them says the hint.
#[test]
fn calls_in_parallel_are_all_counted_and_one_says_the_hint() {
    let d = dir("par");
    let now = nows();
    let said: usize = (0..8)
        .map(|_| {
            let d = d.clone();
            std::thread::spawn(move || {
                (0..6)
                    .filter(|_| tick(&d, "5-1", &RULES[0], now).unwrap().say)
                    .count()
            })
        })
        .collect::<Vec<_>>()
        .into_iter()
        .map(|h| h.join().unwrap())
        .sum();
    assert_eq!(said, 1);
    assert_eq!(
        tick(&d, "5-1", &RULES[0], now).unwrap().n,
        49,
        "48 calls counted, none lost"
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn only_a_looped_command_has_a_rule() {
    assert_eq!(
        rule_of(&args(&["peak", "auto-lr", "t.csv"])).map(|r| r.command),
        Some("peak auto-lr")
    );
    assert!(rule_of(&args(&["peak", "table", "a.mzML"])).is_none());
    assert!(rule_of(&args(&["flow", "run", "/nonexistent.json"])).is_none());
    let d = dir("plan");
    std::fs::create_dir_all(&d).unwrap();
    let (dig, other) = (d.join("dig.json"), d.join("other.json"));
    std::fs::write(&dig, r#"{"steps":[{"id":"d","step":"tp.dig_lr@1"}]}"#).unwrap();
    std::fs::write(
        &other,
        r#"{"steps":[{"id":"d","step":"features.detect@1"}]}"#,
    )
    .unwrap();
    let of = |p: &Path| {
        rule_of(&args(&[
            "flow",
            "run",
            &p.display().to_string(),
            "--out",
            "R",
        ]))
        .map(|r| r.command)
    };
    assert_eq!(of(&dig), Some("tp.dig_lr"));
    assert_eq!(of(&other), None);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_line_names_the_one_call_with_its_key_and_the_count() {
    let l = line(&RULES[0], 10);
    assert!(
        l.contains("10 times") && l.contains("quant targets FILE... --targets TARGETS.csv"),
        "{l}"
    );
    assert!(l.ends_with("(note.cli.repeat.targets)"), "{l}");
    let g = line(&RULES[1], 10);
    assert!(
        g.contains("tp.dig_lr") && g.ends_with("(note.cli.repeat.dig)"),
        "{g}"
    );
}

/// Every command a hint names exists, so the hint never sends an agent to a name that is not there.
#[test]
fn the_calls_a_hint_names_exist() {
    assert_eq!(
        crate::execute(&args(&["quant", "targets", "--help"])).code,
        0
    );
    for r in RULES {
        let o = crate::execute(&args(&["commands", "--find", r.find, "--json"]));
        assert_eq!(o.code, 0, "{}", o.stderr);
    }
    // the words of each rule lead to the call it names
    let found = |w: &str| crate::execute(&args(&["commands", "--find", w, "--json"])).stdout;
    let t = String::from_utf8_lossy(&found(RULES[0].find)).into_owned();
    assert!(t.contains("quant targets"), "{t}");
}
