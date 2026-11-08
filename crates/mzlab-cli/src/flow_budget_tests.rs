//! The memory budget of `flow run` (`--budget-mb`): a small one aligns the series in slabs of m/z and writes the same bytes as the automatic one; a
//! budget that cannot hold the table is an orderly refusal (`error.memoryBudget`), not a crash. The detection asks for more than these small budgets
//! (`trace_stima`): their runs take it from the cache of a first run at the automatic budget and align again.

use std::path::Path;

use crate::flow_run_tests::{fixture, flow_run, folder, p};

/// The files of a run folder, by name.
type Files = Vec<(String, Vec<u8>)>;

/// `flow run` of the base chain with `extra` arguments in the folder `dir`: its outcome.
fn run_in(dir: &Path, extra: &[&str]) -> crate::Outcome {
    let (plan, data, out) = (dir.join("plan.json"), dir.join("data"), dir.join("out"));
    let mut args = vec![p(&plan), "--in", p(&data), "--out", p(&out)];
    args.extend(extra);
    flow_run(&args)
}

/// The base chain in the mode `mode` at the automatic budget, then again in the same folder with `extra` after removing the table and the gap-filled
/// one, so that the detection comes from the cache and only the alignment runs under `extra`: the two run folders and the outcome of the second run.
fn cached_rerun(tag: &str, mode: &str, extra: &[&str]) -> (Files, Files, crate::Outcome) {
    let (dir, _) = fixture(tag);
    let set = format!("ali.mode={mode}");
    let o = run_in(&dir, &["--set", &set]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let out = dir.join("out");
    let free = folder(&out);
    for name in ["ali.mzfeat", "gap.mzfeat"] {
        std::fs::remove_file(out.join(name)).unwrap();
    }
    let mut args = vec!["--set", &set];
    args.extend(extra);
    let o = run_in(&dir, &args);
    let again = folder(&out);
    // the detection was taken from the cache: the small budget did not run it
    let prov = std::fs::read_to_string(out.join("prov.json")).unwrap_or_default();
    assert!(prov.contains("\"cached\""), "{prov}");
    (free, again, o)
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
    let (free, small, o) = cached_rerun("budget_small", "ion", &["--budget-mb", "0.001"]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert_eq!(data(&free), data(&small));
    assert!(data(&free).iter().any(|(n, _)| n == "gap.mzfeat"));
    // the two records tell the way and the memory of the step
    assert!(text(&free, "prov.json").contains("\"via\":\"memory\""));
    assert!(
        text(&small, "prov.json").contains("\"via\":\"slabs\""),
        "{}",
        text(&small, "prov.json")
    );
    assert!(text(&small, "corsa.json").contains("\"via\":\"slabs\""));
    assert!(text(&small, "corsa.json").contains("\"mem_mb\":"));
}

#[test]
fn a_budget_that_cannot_hold_the_table_is_refused_in_order() {
    let (_, refused, o) = cached_rerun("budget_refused", "family", &["--budget-mb", "0.0001"]);
    assert_eq!(o.code, 1, "{}", o.stderr);
    assert!(
        text(&refused, "prov.json").contains("error.memoryBudget"),
        "{}",
        text(&refused, "prov.json")
    );
    // the refusal is said in words with the next step, under the key and its numbers
    assert!(
        o.stderr
            .contains("give more working memory with --budget-mb N"),
        "{}",
        o.stderr
    );
}

#[test]
fn an_index_refused_for_a_dense_mz_range_names_the_budget_to_set() {
    let (dir, _) = fixture("budget_index");
    let (plan, data, out) = (dir.join("plan.json"), dir.join("data"), dir.join("out"));
    let o = flow_run(&[
        p(&plan),
        "--in",
        p(&data),
        "--out",
        p(&out),
        "--set",
        "idx.budget=1",
    ]);
    assert_eq!(o.code, 1, "{}", o.stderr);
    assert!(
        o.stderr.contains("the densest 1-Da m/z range")
            && o.stderr.contains("try --set idx.budget="),
        "{}",
        o.stderr
    );
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
    let (free, capped, o) = cached_rerun("fam_24", "family", &["--budget-mb", "24"]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert_eq!(data(&free), data(&capped));
    // the family mode works in slabs too when the whole table does not fit
    let (free, small, o) = cached_rerun("fam_small", "family", &["--budget-mb", "0.001"]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert_eq!(data(&free), data(&small));
    assert!(
        text(&small, "prov.json").contains("\"via\":\"slabs\""),
        "{}",
        text(&small, "prov.json")
    );
}

#[test]
fn the_automatic_budget_is_shared_by_the_workers_and_budget_mb_is_kept_as_given() {
    let auto = crate::flow_out::Budget::auto();
    auto.run_with(4);
    let shared = mzlab_core::budget_auto::share_mb(auto.mb, 4);
    assert_eq!(auto.job_mb(), shared);
    auto.run_with(1);
    assert_eq!(auto.job_mb(), auto.mb);
    let given = crate::flow_out::Budget::given(24.0);
    given.run_with(4);
    assert_eq!(given.job_mb(), 24.0);
}

#[path = "../../mzlab-feat-wasm/tests/stima_comune/mod.rs"]
mod comune;

/// `flow run` of the fixture `tests/dati/stima_detect.json` (index, detect) at each of its budgets: a budget below the estimate refuses the detection
/// before it starts, with the phase, the numbers of the page and the next step; one above it detects.
#[test]
fn the_detection_is_refused_with_its_numbers_below_the_estimate_of_the_fixture() {
    let s = comune::spec();
    let mb = s["mb"].as_u64().unwrap();
    for (budget, via) in comune::cases(&s) {
        let dir = std::env::temp_dir().join(format!(
            "mzlab_cli_stima_detect_{budget}_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("data")).unwrap();
        std::fs::write(dir.join("data/stima.mzML"), comune::mzml(&s)).unwrap();
        let plan = r#"{"format":"mzflow/2","inputs":[{"name":"stima.mzML","type":"raw"}],"steps":[
{"id":"idx","step":"index.mzidx@1","in":{"source":["file:stima.mzML"]}},
{"id":"det","step":"features.detect@1","in":{"index":["idx.index"]}}]}"#;
        std::fs::write(dir.join("plan.json"), plan).unwrap();
        let (data, out) = (dir.join("data"), dir.join("out"));
        let mb_arg = budget.to_string();
        let o = flow_run(&[
            p(&dir.join("plan.json")),
            "--in",
            p(&data),
            "--out",
            p(&out),
            "--budget-mb",
            &mb_arg,
        ]);
        let prov = std::fs::read_to_string(out.join("prov.json")).unwrap();
        if via == "memory" {
            assert_eq!(o.code, 0, "{}", o.stderr);
            continue;
        }
        assert_eq!(o.code, 1, "{}", o.stderr);
        for want in [
            "\"error.memoryBudget\"",
            "\"fase\":\"detect\"",
            &format!("\"stima_mb\":\"{mb}\""),
            &format!("\"budget_mb\":\"{budget}\""),
        ] {
            assert!(prov.contains(want), "{want} {prov}");
        }
        assert!(
            o.stderr.contains(&format!(
                "detecting this file needs about {mb} MB and the budget is {budget} MB"
            )),
            "{}",
            o.stderr
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
