//! `mzlab-cli llm replay` on the synthetic recordings in `tests/data/llm`: the turn follows the conversation, `--strict` sees a request that was
//! not recorded, and a missing turn is an error with the exit code of the CLI.

use std::path::PathBuf;

use crate::execute;

fn file(name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/data/llm")
        .join(name)
        .to_string_lossy()
        .into_owned()
}

fn replay(extra: &[&str], conv: &str) -> crate::Outcome {
    let mut a: Vec<String> = [
        "llm",
        "replay",
        &file("tool_turn.jsonl"),
        "--conv",
        &file(conv),
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    a.extend(extra.iter().map(|s| s.to_string()));
    execute(&a)
}

#[test]
fn the_turn_follows_the_conversation() {
    let o = replay(&[], "conv_turn0.json");
    assert_eq!(o.code, 0, "{}", o.stderr);
    let t = String::from_utf8(o.stdout).unwrap();
    let doc: serde_json::Value = serde_json::from_str(&t).unwrap();
    assert!(t.starts_with("{\"schema\":\"replay/1\",\"turn\":0"), "{t}");
    let ev = doc["events"].as_array().unwrap();
    assert_eq!(ev.len(), 7);
    assert_eq!(ev[0]["text"], "Let me check the plan.");
    assert_eq!(ev[6]["reason"], "tool");
    let t1 = String::from_utf8(replay(&[], "conv_turn1.json").stdout).unwrap();
    assert!(t1.contains("The plan is valid.") && t1.contains("\"turn\":1"));
    // the same call twice gives the same bytes
    assert_eq!(replay(&[], "conv_turn0.json").stdout, t.into_bytes());
    assert!(
        String::from_utf8(replay(&["--turn", "1"], "conv_turn0.json").stdout)
            .unwrap()
            .contains("The plan is valid.")
    );
}

#[test]
fn strict_wants_a_recorded_request_and_errors_take_the_codes_of_the_contract() {
    // a recording in the wrong format is integrity (3), one with no turn left is data (1), no arguments is usage (2)
    let o = replay(&["--strict"], "conv_turn0.json");
    assert_eq!(o.code, 3);
    assert!(o.stderr.contains("err.llm.replay.format"), "{}", o.stderr);
    let o = replay(&["--turn", "9"], "conv_turn0.json");
    assert_eq!(o.code, 1);
    assert!(
        o.stderr.contains("err.llm.replay.exhausted"),
        "{}",
        o.stderr
    );
    assert_eq!(execute(&["llm".to_string(), "replay".to_string()]).code, 2);
}
