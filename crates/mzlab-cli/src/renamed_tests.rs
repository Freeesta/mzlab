//! Tests of `renamed.rs`.

use super::*;

fn a(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

#[test]
fn every_target_is_a_command_and_no_old_name_is_one() {
    for (old, now) in RENAMED {
        assert!(
            crate::cmd::find(&a(&[old])).is_none(),
            "{old} is a command again"
        );
        assert!(
            crate::cmd::group_of(&a(&[old])).is_none(),
            "{old} is a group"
        );
        if !now.is_empty() {
            let words: Vec<String> = now.split(' ').map(String::from).collect();
            let hit = crate::cmd::find(&words).is_some() || crate::cmd::group_of(&words).is_some();
            assert!(hit, "{old} -> {now}: no such command");
        }
    }
    for (old, now) in RENAMED {
        assert!(
            !now.is_empty() == REMOVED.iter().all(|(o, _)| o != old),
            "{old}"
        );
    }
}

#[test]
fn an_old_name_is_exit_2_with_the_new_one_in_message_and_hint() {
    let o = crate::execute(&a(&["channels", "x.mzML"]));
    assert_eq!(o.code, 2);
    assert!(
        o.stderr.contains("renamed to “file channels”") || o.stderr.contains("file channels"),
        "{}",
        o.stderr
    );
    assert!(
        o.stderr.contains("hint: mzlab-cli file channels --help"),
        "{}",
        o.stderr
    );
    let j = crate::execute(&a(&["manual-table", "--json"]));
    let v: serde_json::Value = serde_json::from_slice(&j.stdout).unwrap();
    assert_eq!(j.code, 2);
    assert_eq!(v["schema"], "error/1");
    assert_eq!(v["error_key"], "err.cli.command.renamed");
    assert_eq!(v["params"]["now"], "peak table");
    assert_eq!(v["hint"], "mzlab-cli peak table --help");
}

#[test]
fn a_removed_command_says_so() {
    let o = crate::execute(&a(&["explain-missing", "t.mzfeat", "e.csv", "--json"]));
    assert_eq!(o.code, 2);
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["error_key"], "err.cli.command.removed");
    assert!(v["message"].as_str().unwrap().contains("explain-missing"));
}

#[test]
fn unknown_words_stay_unknown() {
    let o = crate::execute(&a(&["no-such-command"]));
    assert_eq!(o.code, 2);
    assert!(o.stderr.contains("unknown command no-such-command"));
}
