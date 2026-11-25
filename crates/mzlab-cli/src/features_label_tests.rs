//! The label of the columns of a series with several samples in one file (`features --sample`, `flow run` inputs `NAME@ID`, `table compare`): the sample
//! name, the file only as a tie-break, the rule logged, the duplicate named with the command that fixes it. Synthetic `.wiff` of `tools/wiff_grande.py`;
//! python3 is part of the test: without it the test fails, it never skips.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{execute, Outcome};

fn pair(tag: &str) -> (PathBuf, String) {
    let dir = std::env::temp_dir().join(format!("mzlab_cli_label_{tag}_{}", std::process::id()));
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
    (
        dir.clone(),
        dir.join("in/grande.wiff").display().to_string(),
    )
}

fn run(a: &[&str]) -> Outcome {
    execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

fn header(o: &Outcome) -> Vec<String> {
    let text = String::from_utf8_lossy(&o.stdout);
    text.lines()
        .next()
        .unwrap_or("")
        .split(',')
        .map(String::from)
        .collect()
}

fn areas(h: &[String]) -> Vec<&str> {
    h.iter().filter_map(|c| c.strip_suffix(":area")).collect()
}

#[test]
fn the_columns_of_a_multi_sample_file_are_the_sample_names_and_the_choice_is_logged() {
    let (dir, wiff) = pair("columns");
    let o = run(&["features", &wiff, "--sample", "1,3"]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert_eq!(
        areas(&header(&o)),
        ["datafile:grande_s1", "datafile:grande_s3"]
    );
    for l in [
        "Column grande_s1.wiff is sample 1",
        "Column grande_s3.wiff is sample 3",
    ] {
        assert!(
            o.stderr.contains(l) && o.stderr.contains("(note.features.column)"),
            "{}",
            o.stderr
        );
    }
    // the plan and the report say the column apart from the identity of the member
    let plan = run(&["features", &wiff, "--sample", "1,3", "--dry-run"]);
    let text = String::from_utf8_lossy(&plan.stdout).into_owned();
    assert!(
        text.contains(r#""member":"grande.wiff@1","column":"grande_s1.wiff""#),
        "{text}"
    );
    let report = dir.join("report.json");
    let o = run(&[
        "features",
        &wiff,
        "--sample",
        "2",
        "--report",
        report.to_str().unwrap(),
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let text = std::fs::read_to_string(&report).unwrap();
    assert!(
        text.contains(r#""member":"grande.wiff@2","column":"grande_s2.wiff""#),
        "{text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn table_compare_takes_the_sample_columns_of_the_command() {
    let (dir, wiff) = pair("compare");
    let o = run(&["features", &wiff, "--sample", "1,2,3"]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let table = dir.join("t.csv");
    std::fs::write(&table, &o.stdout).unwrap();
    let t = table.to_str().unwrap();
    let c = run(&[
        "table",
        "compare",
        t,
        "--ref",
        "grande_s1*",
        "--blank",
        "grande_s2*",
        "--dry-run",
    ]);
    assert_eq!(c.code, 0, "{}", c.stderr);
    let text = String::from_utf8_lossy(&c.stdout).into_owned();
    assert!(
        text.contains(r#""name":"grande_s1""#) && text.contains(r#""name":"grande_s3""#),
        "{text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn names_that_collide_across_files_take_the_file_and_the_id_and_the_rule_says_so() {
    let (dir, _) = pair("tie");
    let tools = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools");
    let mut files = Vec::new();
    for (name, names) in [("x", "Alpha,Beta,Gamma"), ("y", "Alpha,Delta,Gamma")] {
        let made = Command::new("python3")
            .arg(tools.join("wiff_grande.py"))
            .arg(dir.join("tie"))
            .args([
                "--name",
                name,
                "--samples",
                "3",
                "--scans",
                "40",
                "--points",
                "50",
                "--names",
                names,
            ])
            .output()
            .expect("python3 is needed by this test");
        assert!(
            made.status.success(),
            "{}",
            String::from_utf8_lossy(&made.stderr)
        );
        files.push(dir.join(format!("tie/{name}.wiff")).display().to_string());
    }
    let o = run(&["features", &files[0], &files[1], "--sample", "1,2,3"]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert_eq!(
        areas(&header(&o)),
        [
            "datafile:Alpha (x #1)",
            "datafile:Beta",
            "datafile:Gamma (x #3)",
            "datafile:Alpha (y #1)",
            "datafile:Delta",
            "datafile:Gamma (y #3)"
        ]
    );
    assert!(
        o.stderr.contains("(note.features.columnTie)"),
        "{}",
        o.stderr
    );
    // the table is usable: no name is on two columns
    let table = dir.join("t.csv");
    std::fs::write(&table, &o.stdout).unwrap();
    let t = table.to_str().unwrap();
    let c = run(&[
        "table",
        "compare",
        t,
        "--ref",
        "Alpha*",
        "--blank",
        "Delta",
        "--dry-run",
    ]);
    assert_eq!(c.code, 0, "{}", c.stderr);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn without_sample_the_run_says_how_to_choose_and_keeps_its_bytes() {
    let (dir, wiff) = pair("first");
    let o = run(&["features", &wiff]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert_eq!(
        areas(&header(&o)),
        ["datafile:grande"],
        "the first sample keeps the name of the file"
    );
    assert!(
        o.stderr.contains("(note.features.firstSample)")
            && o.stderr.contains("features")
            && o.stderr.contains("--sample LIST")
            && o.stderr.contains("sample list"),
        "{}",
        o.stderr
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_table_with_one_name_on_two_columns_is_exit_4_and_names_the_command_that_fixes_it() {
    let (dir, _) = pair("dup");
    let table = dir.join("dup.csv");
    std::fs::write(
        &table,
        "id,mz,rt,rt_range:min,rt_range:max,charge,datafile:run:area,datafile:run:height,datafile:run:feature_state,datafile:run:area,datafile:run:height,datafile:run:feature_state\n1,229.1,1,0,2,0,100,10,DETECTED,200,20,DETECTED\n",
    )
    .unwrap();
    let t = table.to_str().unwrap();
    for extra in [&["--dry-run"][..], &[]] {
        let mut a = vec!["table", "compare", t, "--ref", "run", "--blank", "zzz"];
        a.extend_from_slice(extra);
        let o = run(&a);
        assert_eq!(o.code, 4, "{}", o.stderr);
        assert!(o.stderr.contains("err.compare.duplicate"), "{}", o.stderr);
        assert!(
            o.stderr.contains("hint:")
                && o.stderr.contains("features FILE --sample LIST")
                && o.stderr.contains("sample list FILE"),
            "{}",
            o.stderr
        );
    }
    let o = run(&[
        "table", "compare", t, "--ref", "run", "--blank", "zzz", "--json",
    ]);
    let text = String::from_utf8_lossy(&o.stdout).into_owned();
    assert!(
        text.contains(r#""error_key":"err.compare.duplicate""#) && text.contains(r#""code":4"#),
        "{text}"
    );
    assert!(
        text.contains("features FILE --sample LIST"),
        "the hint of error/1: {text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn flow_inputs_naming_samples_get_the_same_columns() {
    let (dir, wiff) = pair("flow");
    let inputs = r#"{"name":"grande.wiff@1","type":"raw"},{"name":"grande.wiff@3","type":"raw"},{"name":"samples.json","type":"samples"}"#;
    let plan = format!(
        r#"{{"format":"mzflow/2","inputs":[{inputs}],"steps":[
        {{"id":"idx","step":"index.mzidx@1","in":{{"source":["file:grande.wiff@1","file:grande.wiff@3"]}}}},
        {{"id":"det","step":"features.detect@1","in":{{"index":["idx.index"]}}}},
        {{"id":"ali","step":"features.align@1","in":{{"peaks":["det.peaks"]}}}},
        {{"id":"gap","step":"features.gapfill@1","in":{{"table":"ali.table","index":["idx.index"]}}}},
        {{"id":"csv","step":"export.table_csv@1","in":{{"table":"gap.table","samples":"file:samples.json"}}}}]}}"#
    );
    // the samples file names the two columns by their sample names: it is matched to the table by them
    let rows =
        ["grande_s1.wiff", "grande_s3.wiff"].map(|n| (n.to_string(), String::new(), String::new()));
    let samples = mzlab_core::samples::to_json(&mzlab_core::samples::propose(&rows));
    std::fs::write(dir.join("in/samples.json"), samples).unwrap();
    let w = dir.join("work");
    std::fs::create_dir_all(&w).unwrap();
    let (done, prov) =
        crate::flow::run_plan(plan.as_bytes(), &dir.join("in"), &w, 2, None).unwrap();
    assert!(
        done.skipped.is_empty() && !prov.contains("\"error\""),
        "{prov}"
    );
    let csv = std::fs::read_to_string(w.join("csv.csv")).unwrap();
    let h: Vec<String> = csv
        .lines()
        .next()
        .unwrap()
        .split(',')
        .map(String::from)
        .collect();
    assert_eq!(
        areas(&h),
        ["datafile:grande_s1", "datafile:grande_s3"],
        "{csv}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn quant_targets_says_that_it_integrates_the_first_sample_and_names_the_way_to_the_others() {
    let (dir, wiff) = pair("quant");
    let targets = dir.join("targets.csv");
    std::fs::write(&targets, "id,mz,rt_min,rt_max\nT1,229.1,0.2,1.0\n").unwrap();
    let o = run(&[
        "quant",
        "targets",
        "--targets",
        targets.to_str().unwrap(),
        &wiff,
        "--dry-run",
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert!(
        o.stderr.contains("(note.quant.firstSample)")
            && o.stderr.contains("convert")
            && o.stderr.contains("--sample LIST")
            && o.stderr.contains("sample list"),
        "{}",
        o.stderr
    );
    let _ = std::fs::remove_dir_all(&dir);
}
