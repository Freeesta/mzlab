//! `estimate` (AU-6) on the synthetic mzML of the flow tests: each file's numbers are the ones `features.estimate` writes in a plan, the medians are computed from
//! them, the `--set` lines of `--emit-set` are accepted by the checker, and the exit codes and the shape of the contract hold.

use mzlab_core::flow::{check, parse, Platform};
use mzlab_core::json::{self, Value};

use crate::execute;
use crate::flow::steps;
use crate::flow_run::apply_set;
use crate::flow_run_tests::{fixture, p};

fn run(args: &[&str]) -> crate::Outcome {
    let mut a = vec!["estimate".to_string()];
    a.extend(args.iter().map(|x| x.to_string()));
    execute(&a)
}

fn doc(args: &[&str]) -> Value {
    let o = run(args);
    assert_eq!(o.code, 0, "{}", o.stderr);
    json::parse(&o.stdout).expect("JSON")
}

fn paths(d: &std::path::Path) -> Vec<String> {
    ["s_t000", "s_t010", "s_t030"]
        .iter()
        .map(|k| p(&d.join("data").join(format!("{k}.mzML"))).to_string())
        .collect()
}

#[test]
fn the_files_carry_the_numbers_of_the_plan_step_and_the_medians_follow_from_them() {
    let (d, _) = fixture("estimate_cmd");
    let files = paths(&d);
    let mut args: Vec<&str> = files.iter().map(String::as_str).collect();
    args.extend(["--axis", "da", "--json"]);
    let v = doc(&args);
    assert_eq!(v.get("schema").and_then(Value::str), Some("estimate/1"));
    assert!(v.get("citation").is_some());
    let s = v.get("summary").unwrap();
    assert_eq!(s.get("files").and_then(Value::f64), Some(3.0));
    assert_eq!(s.get("estimated").and_then(Value::f64), Some(3.0));
    // the same numbers as the step in a plan
    let plan = std::fs::read_to_string(d.join("plan.json"))
        .unwrap()
        .replace(
            "{\"id\":\"det\"",
            "{\"id\":\"est\",\"step\":\"features.estimate@1\",\"in\":{\"index\":[\"idx.index\"]},\"params\":{\"axis\":\"Da\"}},\n{\"id\":\"det\"",
        );
    let work = d.join("work");
    std::fs::create_dir_all(&work).unwrap();
    crate::flow::run_plan(plan.as_bytes(), &d.join("data"), &work, 1, None).unwrap();
    for (i, k) in ["s_t000", "s_t010", "s_t030"].iter().enumerate() {
        let want = json::parse(
            std::fs::read_to_string(work.join(format!("est-{k}.json")))
                .unwrap()
                .as_bytes(),
        )
        .unwrap();
        let got = &v.get("files").unwrap().arr()[i];
        for key in [
            "n",
            "noise",
            "min_height",
            "ppm",
            "fwhm",
            "rt_tol",
            "dead",
            "da",
            "axis",
            "polarity",
        ] {
            assert_eq!(
                got.get(key).map(mzlab_core::flow::value_text),
                want.get(key).map(mzlab_core::flow::value_text),
                "{k} {key}"
            );
        }
    }
    // each median is the median of its per-file values
    let mut seen = 0;
    for val in v.get("values").unwrap().arr() {
        let mut xs: Vec<f64> = val
            .get("per_file")
            .unwrap()
            .arr()
            .iter()
            .filter_map(|f| f.get("value").and_then(Value::f64))
            .collect();
        xs.sort_by(f64::total_cmp);
        let med = val.get("median").and_then(Value::f64);
        match xs.len() {
            0 => assert!(med.is_none()),
            n => {
                let want = if n % 2 == 1 {
                    xs[n / 2]
                } else {
                    (xs[n / 2 - 1] + xs[n / 2]) / 2.0
                };
                assert_eq!(med, Some(want));
                assert_eq!(val.get("min").and_then(Value::f64), Some(xs[0]));
                assert_eq!(val.get("max").and_then(Value::f64), Some(xs[n - 1]));
                seen += 1;
            }
        }
    }
    assert!(seen >= 2, "only {seen} values were measured");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn emit_set_prints_lines_for_the_instances_of_the_plan_and_the_checker_accepts_them() {
    let (d, _) = fixture("estimate_set");
    let files = paths(&d);
    let plan_path = p(&d.join("plan.json")).to_string();
    let mut args: Vec<&str> = files.iter().map(String::as_str).collect();
    args.extend(["--axis", "da", "--emit-set", &plan_path]);
    let o = run(&args);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let text = String::from_utf8(o.stdout).unwrap();
    assert!(text.contains("--set det.min_height="), "{text}");
    assert!(text.contains("# min_height: median of 3 file(s)"), "{text}");
    let reg = steps();
    let mut plan = parse(&std::fs::read(d.join("plan.json")).unwrap()).unwrap();
    let mut n = 0;
    for line in text.lines() {
        let set = line
            .trim_start_matches("--set ")
            .split("   #")
            .next()
            .unwrap();
        apply_set(&mut plan, &reg, set).unwrap_or_else(|e| panic!("{set}: {e}"));
        n += 1;
    }
    assert!(n >= 2);
    let bad: Vec<&str> = check(&plan, &reg, Platform::Native)
        .iter()
        .map(|p| p.error_key)
        .filter(|k| k.starts_with("err.flow.param"))
        .collect();
    assert!(bad.is_empty(), "{bad:?}");
    // the plan on disk is untouched and a second run gives the same lines
    assert_eq!(run(&args).stdout, text.as_bytes());
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_other_axis_is_skipped_with_its_reason_and_few_files_are_noted_not_refused() {
    let (d, _) = fixture("estimate_axis");
    let files = paths(&d);
    let v = doc(&[&files[0], "--axis", "da", "--json"]);
    let skipped = v.get("skipped").unwrap().arr();
    assert!(skipped
        .iter()
        .any(|s| s.get("param").and_then(Value::str) == Some("ppm")
            && s.get("reason").and_then(Value::str) == Some("axis")));
    let notes = v.get("notes").unwrap().arr();
    assert!(notes
        .iter()
        .any(|n| n.get("key").and_then(Value::str) == Some("note.estimate.fewFiles")));
    // no plan: the instance is the last part of the step id
    assert!(v.get("params").unwrap().arr().iter().any(|p| p
        .get("set")
        .and_then(Value::str)
        .is_some_and(|s| s.starts_with("detect."))));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn every_parameter_a_value_can_set_exists_in_its_step() {
    let reg = steps();
    for (step, param) in mzlab_core::estimates::targets() {
        let s = reg
            .iter()
            .filter(|s| s.id == step)
            .max_by_key(|s| s.version)
            .unwrap_or_else(|| panic!("no step {step}"));
        assert!(s.param(param).is_some(), "{step} has no parameter {param}");
    }
}

#[test]
fn exit_codes_a_missing_file_4_usage_2_a_bad_plan_3_and_a_file_that_cannot_be_read_1() {
    assert_eq!(run(&["no/such/file.mzML"]).code, 4);
    assert_eq!(run(&[]).code, 2);
    let (d, _) = fixture("estimate_exit");
    let files = paths(&d);
    assert_eq!(run(&[&files[0], "--axis", "x"]).code, 2);
    assert_eq!(run(&[&files[0], "--polarity", "2"]).code, 2);
    assert_eq!(run(&[&files[0], "--emit-set", &files[0]]).code, 3);
    assert_eq!(run(&[&files[0], "--emit-set", "no/plan.json"]).code, 4);
    // a file that is not a data file: its entry has the error, the readable one is answered, exit 1
    let junk = d.join("junk.mzML");
    std::fs::write(&junk, b"not xml").unwrap();
    let o = run(&[&files[0], p(&junk), "--json"]);
    assert_eq!(o.code, 1, "{}", o.stderr);
    let v = json::parse(&o.stdout).unwrap();
    assert_eq!(
        v.get("summary")
            .and_then(|s| s.get("no_estimate"))
            .and_then(Value::f64),
        Some(1.0)
    );
    assert!(v.get("files").unwrap().arr()[1].get("error").is_some());
    let _ = std::fs::remove_dir_all(&d);
}
