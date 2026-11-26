//! `quant targets --sample LIST` (members `FILE@ID`, names by the label rule, the same bytes as a `flow run` over `NAME@ID` inputs) and the stderr hint of the commands that
//! read one sample of a file with several (`file items`, `trace background`, `dad ms-at|delay|peaks`). Synthetic `.wiff` of `tools/wiff_grande.py`; python3 is part of the test.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{execute, Outcome};

fn pair(tag: &str) -> (PathBuf, String) {
    let dir = std::env::temp_dir().join(format!("mzlab_cli_first_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let tools = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools");
    let made = Command::new("python3")
        .arg(tools.join("wiff_grande.py"))
        .arg(dir.join("in"))
        .args(["--samples", "3", "--scans", "40", "--points", "50"])
        .output()
        .expect("python3 is needed by this test");
    assert!(
        made.status.success(),
        "{}",
        String::from_utf8_lossy(&made.stderr)
    );
    std::fs::write(
        dir.join("in/targets.csv"),
        "id,mz,rt_min,rt_max\nT1,229.1,0.2,1.0\nAway,555.5,0.2,1.0\n",
    )
    .unwrap();
    (
        dir.clone(),
        dir.join("in/grande.wiff").display().to_string(),
    )
}

fn run(a: &[&str]) -> Outcome {
    execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

fn quant(dir: &Path, wiff: &str, more: &[&str]) -> Outcome {
    let t = dir.join("in/targets.csv").display().to_string();
    let mut a = vec!["quant", "targets", "--targets", &t, wiff];
    a.extend_from_slice(more);
    run(&a)
}

/// Sample 2 of the file as an mzML of its own.
fn one_sample(dir: &Path, wiff: &str) -> PathBuf {
    let made = run(&[
        "file",
        "convert",
        wiff,
        "--sample",
        "2",
        "--out",
        dir.to_str().unwrap(),
    ]);
    assert_eq!(made.code, 0, "{}", made.stderr);
    let mzml = std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .find(|p| p.extension().is_some_and(|e| e == "mzML"))
        .unwrap();
    let one = dir.join("one.mzML");
    std::fs::rename(&mzml, &one).unwrap();
    one
}

fn text(o: &Outcome) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

#[test]
fn sample_makes_one_file_of_the_table_per_chosen_sample_named_by_the_label_rule() {
    let (dir, wiff) = pair("table");
    let log = dir.join("q.log.json");
    let o = quant(
        &dir,
        &wiff,
        &["--sample", "1,3", "--log", log.to_str().unwrap()],
    );
    assert_eq!(o.code, 0, "{}", o.stderr);
    let files: Vec<String> = text(&o)
        .lines()
        .skip(1)
        .map(|l| l.split(',').next().unwrap().to_string())
        .collect();
    assert_eq!(
        files,
        [
            "grande_s1.wiff",
            "grande_s1.wiff",
            "grande_s3.wiff",
            "grande_s3.wiff"
        ]
    );
    // every automatic choice is said: the label of each sample and how it was made
    for n in [
        "Column grande_s1.wiff is sample 1",
        "Column grande_s3.wiff is sample 3",
    ] {
        assert!(
            o.stderr.contains(n) && o.stderr.contains("(note.features.column)"),
            "{}",
            o.stderr
        );
    }
    // the bytes of a sample are those of the same sample alone: the order and the other samples change nothing
    let one = quant(&dir, &wiff, &["--sample", "3"]);
    let rows = |o: &Outcome| {
        text(o)
            .lines()
            .skip(1)
            .map(String::from)
            .collect::<Vec<_>>()
    };
    assert_eq!(rows(&one), rows(&o)[2..]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_plan_and_the_json_answer_name_each_member_as_file_at_id() {
    let (dir, wiff) = pair("members");
    for more in [&["--dry-run"][..], &["--json"][..]] {
        let mut a = vec!["--sample", "2-3"];
        a.extend_from_slice(more);
        let o = quant(&dir, &wiff, &a);
        assert_eq!(o.code, 0, "{}", o.stderr);
        let v = mzlab_core::json::parse(text(&o).as_bytes()).unwrap();
        let m = v.get("members").unwrap().arr();
        let at = |k: usize, key: &str| m[k].get(key).unwrap().str().unwrap().to_string();
        assert_eq!(
            (at(0, "name"), at(0, "member")),
            ("grande_s2.wiff".into(), "grande.wiff@2".into())
        );
        assert_eq!(
            (at(1, "name"), at(1, "member"), at(1, "label")),
            (
                "grande_s3.wiff".into(),
                "grande.wiff@3".into(),
                "own".into()
            )
        );
        assert_eq!(m[1].get("sample_id").unwrap().f64(), Some(3.0));
    }
    // without --sample the answers keep their keys
    let plain = quant(&dir, &wiff, &["--dry-run"]);
    assert!(!text(&plain).contains("members"), "{}", text(&plain));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_bad_sample_list_is_exit_2_an_unknown_id_exit_5_and_no_multi_sample_input_is_exit_2() {
    let (dir, wiff) = pair("exits");
    let bad = quant(&dir, &wiff, &["--sample", "3-1"]);
    assert_eq!(bad.code, 2, "{}", bad.stderr);
    let unknown = quant(&dir, &wiff, &["--sample", "9"]);
    assert_eq!(unknown.code, 5, "{}", unknown.stderr);
    assert!(unknown.stderr.contains("sample list"), "{}", unknown.stderr);
    // an mzML holds one sample: the list has nothing to apply to, never silent
    let one = one_sample(&dir, &wiff);
    let single = quant(&dir, one.to_str().unwrap(), &["--sample", "1"]);
    assert_eq!(single.code, 2, "{}", single.stderr);
    assert!(
        single.stderr.contains("err.sample.single"),
        "{}",
        single.stderr
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn without_sample_quant_targets_names_the_flag_that_chooses() {
    let (dir, wiff) = pair("note");
    let o = quant(&dir, &wiff, &["--dry-run"]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert!(
        o.stderr.contains("(note.quant.firstSample)")
            && o.stderr.contains("quant targets")
            && o.stderr.contains("--sample LIST")
            && o.stderr.contains("sample list"),
        "{}",
        o.stderr
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_flow_over_name_at_id_inputs_gives_the_bytes_of_the_command() {
    let (dir, wiff) = pair("flow");
    let log = dir.join("q.log.json");
    let o = quant(
        &dir,
        &wiff,
        &["--sample", "1,3", "--log", log.to_str().unwrap()],
    );
    assert_eq!(o.code, 0, "{}", o.stderr);
    let plan = r#"{"format":"mzflow/2","inputs":[{"name":"grande.wiff@1","type":"raw"},{"name":"grande.wiff@3","type":"raw"},{"name":"targets.csv","type":"report"}],"steps":[
        {"id":"idx","step":"index.mzidx@1","in":{"source":["file:grande.wiff@1","file:grande.wiff@3"]}},
        {"id":"qt","step":"quant.targets@1","in":{"raw":["idx.index"],"targets":"file:targets.csv"}}]}"#;
    let w = dir.join("work");
    std::fs::create_dir_all(&w).unwrap();
    let (done, prov) =
        crate::flow::run_plan(plan.as_bytes(), &dir.join("in"), &w, 2, None).unwrap();
    assert!(
        done.skipped.is_empty() && !prov.contains("\"error\""),
        "{prov}"
    );
    assert_eq!(std::fs::read_to_string(w.join("qt.csv")).unwrap(), text(&o));
    assert_eq!(
        std::fs::read_to_string(w.join("qt.log.json")).unwrap(),
        std::fs::read_to_string(&log).unwrap()
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_commands_that_read_the_first_sample_say_how_to_reach_the_others_and_keep_their_exit_and_bytes(
) {
    let (dir, wiff) = pair("hints");
    let key = "(note.sample.firstOnly)";
    let items = run(&["file", "items", &wiff]);
    let bg = run(&["trace", "background", &wiff]);
    for (o, command) in [(&items, "file items"), (&bg, "trace background")] {
        assert_eq!(o.code, 0, "{}", o.stderr);
        let hint = o
            .stderr
            .lines()
            .find(|l| l.contains(key))
            .unwrap_or_else(|| panic!("{}", o.stderr));
        assert!(
            hint.starts_with("hint: ")
                && hint.contains(command)
                && hint.contains("holds 3 samples")
                && hint.contains("file convert")
                && hint.contains("--sample LIST")
                && hint.contains("sample list"),
            "{hint}"
        );
    }
    // the hint is on stderr only: stdout is a document, whatever it says
    assert!(
        text(&items).starts_with("{\"schema\":\"items/1\""),
        "{}",
        text(&items)
    );
    // dad takes --sample ID: the hint names it, and goes away once a sample is chosen
    let ms = run(&["dad", "ms-at", &wiff, "--rt", "0.5"]);
    assert_eq!(ms.code, 0, "{}", ms.stderr);
    assert!(
        ms.stderr.contains("(note.sample.firstChoose)") && ms.stderr.contains("--sample ID"),
        "{}",
        ms.stderr
    );
    let chosen = run(&["dad", "ms-at", &wiff, "--rt", "0.5", "--sample", "2"]);
    assert_eq!(chosen.code, 0, "{}", chosen.stderr);
    assert!(
        !chosen.stderr.contains("note.sample.first"),
        "{}",
        chosen.stderr
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_file_of_one_sample_gets_no_hint() {
    let (dir, wiff) = pair("single");
    let one = one_sample(&dir, &wiff);
    let o = run(&["file", "items", one.to_str().unwrap()]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert!(!o.stderr.contains("note.sample.first"), "{}", o.stderr);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_words_of_the_task_find_the_command_that_takes_sample_and_the_listing_that_gives_the_ids() {
    for (task, want) in [
        (
            "integrate targets in several samples of one wiff file",
            "quant targets",
        ),
        (
            "choose samples of a wiff for the target list",
            "quant targets",
        ),
        ("which samples does a wiff file hold", "sample list"),
    ] {
        let o = run(&["commands", "--find", task, "--json"]);
        assert_eq!(o.code, 0, "{}", o.stderr);
        let v = mzlab_core::json::parse(text(&o).as_bytes()).unwrap();
        let top: Vec<&str> = v
            .get("matches")
            .unwrap()
            .arr()
            .iter()
            .take(3)
            .filter_map(|m| m.get("name").and_then(|n| n.str()))
            .collect();
        assert!(top.contains(&want), "{task:?}: {want} not in {top:?}");
    }
}
