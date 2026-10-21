//! `mzlab-cli tutor eval` on the synthetic corpus and recordings in `tests/data/tutor`: every recorded case passes, a bad recording fails with exit 1,
//! the live mode refuses to start without `--yes`, and a wrong command line is exit 2.

use std::path::PathBuf;

use crate::execute;

fn data(name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/data/tutor")
        .join(name)
        .to_string_lossy()
        .into_owned()
}

fn run(args: &[&str]) -> crate::Outcome {
    let a: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    execute(&a)
}

fn text(o: &crate::Outcome) -> String {
    String::from_utf8(o.stdout.clone()).unwrap()
}

#[test]
fn every_recorded_case_passes() {
    let (cases, blocks, dir) = (data("eval"), data("blocchi-mini.json"), data(""));
    let o = run(&[
        "tutor",
        "eval",
        &cases,
        "--blocchi",
        &blocks,
        "--replay",
        &dir,
    ]);
    assert_eq!(o.code, 0, "{}\n{}", o.stderr, text(&o));
    let t = text(&o);
    assert!(t.lines().last().unwrap().contains("\"failed\":0"), "{t}");
    assert!(t.lines().last().unwrap().contains("\"scored\":9"), "{t}");
    assert_eq!(t.lines().filter(|l| l.contains("\"pass\":true")).count(), 9);
}

#[test]
fn a_recording_with_an_invented_citation_fails() {
    let dir = std::env::temp_dir().join(format!("mzlab-tutor-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let key = "c27816e75926";
    let good = std::fs::read_to_string(data(&format!("{key}.jsonl"))).unwrap();
    std::fs::write(dir.join(format!("{key}.jsonl")), good.replace("[1]", "[7]")).unwrap();
    let o = run(&[
        "tutor",
        "eval",
        &data("eval"),
        "--blocchi",
        &data("blocchi-mini.json"),
        "--replay",
        dir.to_str().unwrap(),
    ]);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(o.code, 1, "{}", text(&o));
    assert!(text(&o).contains("\"grounded\":false"));
}

#[test]
fn recall_mode_reports_the_search() {
    let o = run(&[
        "tutor",
        "eval",
        &data("eval"),
        "--blocchi",
        &data("blocchi-mini.json"),
        "--solo-recupero",
    ]);
    assert_eq!(o.code, 0);
    assert!(text(&o).contains("\"lang\":\"it\""));
}

#[test]
fn the_live_mode_asks_for_yes_and_the_command_line_is_checked() {
    let live = run(&[
        "tutor",
        "eval",
        &data("eval"),
        "--blocchi",
        &data("blocchi-mini.json"),
        "--provider",
        "anthropic",
        "--key-file",
        "/nonexistent",
    ]);
    assert_eq!(live.code, 2);
    assert!(live.stderr.contains("--yes"), "{}", live.stderr);
    assert_eq!(run(&["tutor", "eval", &data("eval")]).code, 2);
    assert_eq!(
        run(&[
            "tutor",
            "eval",
            &data("eval"),
            "--blocchi",
            &data("blocchi-mini.json")
        ])
        .code,
        2
    );
    assert_eq!(run(&["tutor", "nope"]).code, 2);
    assert_eq!(run(&["tutor", "--help"]).code, 0);
}
