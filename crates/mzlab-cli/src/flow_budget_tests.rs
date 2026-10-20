//! The memory budget of `flow run` (`--budget-mb`): a small one aligns the series in slabs of m/z and writes the same bytes as the automatic one; a
//! budget that cannot hold the table is an orderly refusal (`error.memoryBudget`), not a crash.

use crate::flow_run_tests::{fixture, flow_run, folder, p};

/// `flow run` of the base chain in `ion` mode with `extra` arguments; the run folder.
fn run_ion(tag: &str, extra: &[&str]) -> Vec<(String, Vec<u8>)> {
    let (dir, _) = fixture(tag);
    let (plan, data, out) = (dir.join("plan.json"), dir.join("data"), dir.join("out"));
    let mut args = vec![
        p(&plan),
        "--in",
        p(&data),
        "--out",
        p(&out),
        "--set",
        "ali.mode=ion",
    ];
    args.extend(extra);
    let o = flow_run(&args);
    assert_eq!(o.code, 0, "{}", o.stderr);
    folder(&out)
}

/// The files of a run folder but the two records, which say which way the alignment took.
fn data(run: &[(String, Vec<u8>)]) -> Vec<&(String, Vec<u8>)> {
    run.iter()
        .filter(|(n, _)| n != "prov.json" && n != "corsa.json")
        .collect()
}

fn text<'a>(run: &'a [(String, Vec<u8>)], name: &str) -> &'a str {
    let (_, b) = run.iter().find(|(n, _)| n == name).unwrap();
    std::str::from_utf8(b).unwrap()
}

#[test]
fn a_small_budget_aligns_in_slabs_and_writes_the_same_bytes() {
    let free = run_ion("budget_free", &[]);
    let small = run_ion("budget_small", &["--budget-mb", "0.001"]);
    assert_eq!(data(&free), data(&small));
    assert!(data(&free).iter().any(|(n, _)| n == "gap.mzfeat"));
    // the two records tell the way and the memory of the step
    assert!(text(&free, "prov.json").contains("\"via\":\"memory\""));
    assert!(text(&small, "prov.json").contains("\"via\":\"slabs\""));
    assert!(text(&small, "corsa.json").contains("\"via\":\"slabs\""));
    assert!(text(&small, "corsa.json").contains("\"mem_mb\":"));
}

#[test]
fn a_budget_that_cannot_hold_the_table_is_refused_in_order() {
    let (dir, _) = fixture("budget_refused");
    let (plan, data, out) = (dir.join("plan.json"), dir.join("data"), dir.join("out"));
    let o = flow_run(&[
        p(&plan),
        "--in",
        p(&data),
        "--out",
        p(&out),
        "--set",
        "ali.mode=family",
        "--budget-mb",
        "0.0001",
    ]);
    assert_eq!(o.code, 1, "{}", o.stderr);
    let prov = std::fs::read_to_string(out.join("prov.json")).unwrap();
    assert!(prov.contains("error.memoryBudget"), "{prov}");
}

#[test]
fn the_budget_must_be_a_positive_number() {
    for bad in ["0", "-3", "many"] {
        let o = flow_run(&["plan.json", "--budget-mb", bad, "--out", "x"]);
        assert_eq!(o.code, 2, "{bad}");
    }
}

#[test]
fn the_family_mode_gives_the_same_bytes_with_and_without_a_budget() {
    let run = |tag: &str, extra: &[&str]| {
        let (dir, _) = fixture(tag);
        let (plan, data, out) = (dir.join("plan.json"), dir.join("data"), dir.join("out"));
        let mut args = vec![p(&plan), "--in", p(&data), "--out", p(&out)];
        args.extend(["--set", "ali.mode=family"]);
        args.extend(extra);
        let o = flow_run(&args);
        assert_eq!(o.code, 0, "{}", o.stderr);
        folder(&out)
    };
    let (free, capped) = (run("fam_free", &[]), run("fam_24", &["--budget-mb", "24"]));
    assert_eq!(data(&free), data(&capped));
}
