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
    std::fs::write(&other, r#"{"steps":[{"id":"d","step":"table.stats@1"}]}"#).unwrap();
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

/// A loop of one-mass `trace xic` / `raw xic` calls is counted; a list or a sweep is already the one call and never is.
#[test]
fn a_looped_single_mass_xic_has_a_rule_and_a_list_or_sweep_has_none() {
    let cmd = |a: &[&str]| rule_of(&args(a)).map(|r| r.command);
    assert_eq!(
        cmd(&["trace", "xic", "a.mzML", "--mz", "364.4", "--da", "0.3"]),
        Some("trace xic")
    );
    assert_eq!(
        cmd(&["trace", "xic", "a.mzML", "--mz=364.4", "--da", "0.3"]),
        Some("trace xic")
    );
    assert_eq!(
        cmd(&["raw", "xic", "a.mzML", "--mz", "364.4", "--rt", "1,2"]),
        Some("raw xic")
    );
    for a in [
        &[
            "trace",
            "xic",
            "a.mzML",
            "--mz",
            "364.4,194.2",
            "--da",
            "0.3",
        ][..],
        &["trace", "xic", "a.mzML", "--sweep", "100:500:0.5"],
        &["raw", "xic", "a.mzML", "--mz", "364.4,194.2", "--rt", "1,2"],
    ] {
        assert_eq!(cmd(a), None, "{a:?}");
    }
    let (t, r) = (line(&RULES[2], 10), line(&RULES[3], 10));
    assert!(
        t.contains("--sweep LO:HI:STEP") && t.ends_with("(note.cli.repeat.xic)"),
        "{t}"
    );
    assert!(
        r.contains("raw xic FILE... --mz M1,M2,...") && r.ends_with("(note.cli.repeat.rawxic)"),
        "{r}"
    );
}

/// Each plan family of `flow run` has a rule by its steps; a plan with none of them, or an unreadable plan, has none; `tp.dig_lr` wins over the others.
#[test]
fn a_flow_plan_has_the_rule_of_its_family() {
    let d = dir("fam");
    std::fs::create_dir_all(&d).unwrap();
    let of = |name: &str, steps: &[&str]| {
        let body: Vec<String> = steps
            .iter()
            .enumerate()
            .map(|(i, s)| format!(r#"{{"id":"s{i}","step":"{s}@1"}}"#))
            .collect();
        let f = d.join(name);
        std::fs::write(&f, format!(r#"{{"steps":[{}]}}"#, body.join(","))).unwrap();
        rule_of(&args(&[
            "flow",
            "run",
            &f.display().to_string(),
            "--out",
            "R",
        ]))
        .map(|r| r.command)
    };
    assert_eq!(
        of("a.json", &["quant.targets"]),
        Some("flow run quant.targets")
    );
    assert_eq!(
        of("b.json", &["index.mzidx", "chrom.transitions"]),
        Some("flow run chrom.transitions")
    );
    assert_eq!(
        of(
            "c.json",
            &["index.mzidx", "features.detect", "features.align"]
        ),
        Some("flow run features")
    );
    assert_eq!(of("d.json", &["convert.mzml"]), Some("flow run convert"));
    assert_eq!(
        of("e.json", &["import.raw", "spectra.centroid"]),
        Some("flow run convert")
    );
    assert_eq!(
        of("f.json", &["features.detect", "tp.dig_lr"]),
        Some("tp.dig_lr")
    );
    assert_eq!(
        of("g.json", &["table.stats"]),
        None,
        "a step that is not looped"
    );
    assert_eq!(
        of("h.json", &["features.detectx"]),
        None,
        "the step id is whole"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// The lines of the new families name the one call and end with their key; every call they name exists.
#[test]
fn the_family_lines_name_a_call_that_exists() {
    let by = |c: &str| RULES.iter().find(|r| r.command == c).unwrap();
    for (rule, names, key) in [
        (
            "flow run chrom.transitions",
            "peak table FILE... --auto",
            "note.cli.repeat.peaks",
        ),
        (
            "flow run features",
            "flow run PLAN.json --in FOLDER",
            "note.cli.repeat.features",
        ),
        (
            "flow run convert",
            "file convert INPUT... --out FOLDER",
            "note.cli.repeat.convert",
        ),
        (
            "flow run quant.targets",
            "quant targets FILE... --targets",
            "note.cli.repeat.targets",
        ),
    ] {
        let l = line(by(rule), 10);
        assert!(
            l.contains(names) && l.contains("10 times") && l.ends_with(&format!("({key})")),
            "{l}"
        );
    }
    for c in [
        ["peak", "table"],
        ["file", "convert"],
        ["features", "--help"],
        ["flow", "run"],
    ] {
        let help = if c[1] == "--help" {
            vec![c[0], "--help"]
        } else {
            vec![c[0], c[1], "--help"]
        };
        assert_eq!(crate::execute(&args(&help)).code, 0, "{c:?}");
    }
    // the words of each family lead to the call its line names
    for (rule, call) in [
        ("flow run chrom.transitions", "peak table"),
        ("flow run features", "features (command)"),
        ("flow run convert", "file convert"),
    ] {
        let out = crate::execute(&args(&["commands", "--find", by(rule).find]));
        let t = String::from_utf8_lossy(&out.stdout).into_owned();
        assert!(t.contains(call), "{rule}: {t}");
    }
    let mut states: Vec<&str> = RULES.iter().map(|r| r.command).collect();
    states.sort_unstable();
    states.dedup();
    assert_eq!(
        states.len(),
        RULES.len(),
        "one count per rule: the commands differ"
    );
}

/// The parent of a test process is tagged by its id and start moment (`/proc` on Unix; the Windows module has its own test).
#[cfg(unix)]
#[test]
fn the_parent_is_tagged_by_its_id_and_start() {
    let tag = parent_tag().unwrap();
    let (id, start) = tag.split_once('-').unwrap();
    assert!(
        id.parse::<u32>().unwrap() > 0 && start.parse::<u64>().unwrap() > 0,
        "{tag}"
    );
    assert_eq!(parent_tag().as_deref(), Some(tag.as_str()), "stable");
}
