//! The help of every command is the text the program printed before the command table existed (`tests/data/help/help_snapshot.txt`, taken from main), and the table is consistent.

use crate::execute;

const SNAPSHOT: &str = include_str!("../../tests/data/help/help_snapshot.txt");

#[test]
fn every_help_and_usage_error_is_unchanged() {
    let mut n = 0;
    for chunk in SNAPSHOT.split("=== mzlab-cli ").skip(1) {
        let (line, _) = chunk.split_once('\n').unwrap();
        let args: Vec<String> = line.split_whitespace().map(String::from).collect();
        let o = execute(&args);
        let got = format!(
            "{line}\nexit {}\n--- stdout\n{}--- stderr\n{}\n",
            o.code,
            String::from_utf8_lossy(&o.stdout),
            o.stderr
        );
        assert_eq!(got, chunk, "mzlab-cli {line}");
        n += 1;
    }
    assert!(n >= 60, "the snapshot lists {n} invocations");
}

#[test]
fn names_are_unique_and_aliases_point_to_a_command() {
    let mut seen = std::collections::HashSet::new();
    for e in super::entries() {
        assert!(seen.insert(e.name), "{} twice", e.name);
        for a in e.aliases {
            assert_eq!(super::canonical(a), e.name);
            assert!(!seen.contains(a), "alias {a} is also a name");
        }
    }
}
