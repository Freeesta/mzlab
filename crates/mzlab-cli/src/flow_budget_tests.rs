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

#[test]
fn a_small_budget_aligns_in_slabs_and_writes_the_same_bytes() {
    let free = run_ion("budget_free", &[]);
    let small = run_ion("budget_small", &["--budget-mb", "0.001"]);
    assert_eq!(free, small);
    assert!(free.iter().any(|(n, _)| n == "gap.mzfeat"));
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
