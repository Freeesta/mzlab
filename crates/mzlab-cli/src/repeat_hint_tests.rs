use super::*;

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_hint_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn args(a: &[&str]) -> Vec<String> {
    a.iter().map(|s| s.to_string()).collect()
}

#[test]
fn the_hint_is_due_at_the_tenth_call_and_then_every_thousandth() {
    let due_at: Vec<u64> = (1..=3000).filter(|&n| due(n)).collect();
    assert_eq!(due_at, vec![10, 1000, 2000, 3000]);
}

#[test]
fn a_parent_is_counted_apart_and_a_long_pause_starts_again() {
    let d = dir("count");
    let rule = &RULES[0];
    let n: Vec<u64> = (0..12)
        .map(|i| tick(&d, 7, rule, 100 + i).unwrap())
        .collect();
    assert_eq!(n[9], FIRST);
    assert_eq!(
        tick(&d, 8, rule, 120),
        Some(1),
        "another parent has its own count"
    );
    assert_eq!(tick(&d, 7, &RULES[1], 120), Some(1), "another command too");
    assert_eq!(
        tick(&d, 7, rule, 111 + GAP_S + 1),
        Some(1),
        "a long pause starts again"
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
    assert!(l.ends_with("(note.cli.repeat)"), "{l}");
    assert!(line(&RULES[1], 10).contains("tp.dig_lr"));
}

/// Every command a rule points at exists, so the hint never sends an agent to a name that is not there.
#[test]
fn the_calls_a_hint_names_exist() {
    for r in RULES {
        let first = r
            .bulk
            .split_whitespace()
            .take(2)
            .collect::<Vec<_>>()
            .join(" ");
        if first == "quant targets" {
            assert!(crate::execute(&args(&["quant", "targets", "--help"])).code == 0);
        }
        let o = crate::execute(&args(&["commands", "--find", r.find, "--json"]));
        assert_eq!(o.code, 0, "{}", o.stderr);
    }
}
