//! `flow steps --id STEP --template`: a plan the reader and the checker take as written, for every step of the binary.

use mzlab_core::flow::{check, parse, Platform};

use crate::execute;
use crate::flow::steps;

fn run(args: &[&str]) -> crate::Outcome {
    execute(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

#[test]
fn every_step_has_a_template_that_reads_and_has_no_parameter_or_port_problem() {
    let reg = steps();
    let mut at_most_input_types = 0;
    for s in &reg {
        let o = run(&[
            "flow",
            "steps",
            "--id",
            &format!("{}@{}", s.id, s.version),
            "--template",
        ]);
        assert_eq!(o.code, 0, "{}: {}", s.id, o.stderr);
        let plan = parse(&o.stdout)
            .unwrap_or_else(|e| panic!("{}@{} does not read: {}", s.id, s.version, e.key));
        for p in check(&plan, &reg, Platform::Native) {
            // the placeholder files are named after the port, not given a format: what can differ is the type or the facets of the data
            assert!(
                matches!(
                    p.error_key,
                    "err.flow.type"
                        | "err.flow.facet"
                        | "err.flow.port.format"
                        | "err.flow.lineage"
                        | "err.flow.step.version"
                        | "err.flow.unavailable"
                ),
                "{}@{}: {} {}",
                s.id,
                s.version,
                p.error_key,
                p.port
            );
            at_most_input_types += 1;
        }
    }
    assert!(
        at_most_input_types < reg.len(),
        "most templates are valid as written"
    );
}

#[test]
fn the_template_names_the_instance_and_lists_every_default() {
    let o = run(&["flow", "steps", "--id", "chrom.uv_peaks", "--template"]);
    let text = String::from_utf8(o.stdout).unwrap();
    for want in [
        "\"format\":\"mzflow/2\"",
        "\"id\":\"uv_peaks\"",
        "\"step\":\"chrom.uv_peaks@1\"",
        "\"source\":\"file:source.mzML\"",
        "\"wavelength_nm\":254",
        "\"broad\":false",
    ] {
        assert!(text.contains(want), "{want} in {text}");
    }
    // the same template through the checker of the plan on disk: exit 0
    let dir = std::env::temp_dir().join(format!("mzlab-template-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let plan = dir.join("plan.json");
    std::fs::write(&plan, &text).unwrap();
    let c = run(&["flow", "check", plan.to_str().unwrap()]);
    assert_eq!(c.code, 0, "{}", c.stderr);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn an_unknown_step_is_exit_5_and_a_bad_call_is_exit_2() {
    let o = run(&["flow", "steps", "--id", "nope.step", "--template", "--json"]);
    assert_eq!(o.code, 5);
    assert!(String::from_utf8_lossy(&o.stdout).contains("\"error_key\":\"err.flow.step.unknown\""));
    assert_eq!(run(&["flow", "steps", "--template"]).code, 2);
}

/// A folder with `name` and the plan text `plan` written as `plan.json`.
fn plan_dir(tag: &str, plan: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab-strict-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("plan.json"), plan).unwrap();
    std::fs::write(d.join("index.mzidx"), b"x").unwrap();
    d
}

#[test]
fn a_wrong_plan_is_exit_3_with_its_path_in_check_and_in_dry_run() {
    let good = String::from_utf8(
        run(&["flow", "steps", "--id", "features.detect@2", "--template"]).stdout,
    )
    .unwrap();
    let wrong = [
        ("hello", r#"{"hello":"world"}"#.to_string(), "hello"),
        ("schema", good.replacen("\"format\"", "\"schema\"", 1), "schema"),
        ("param", good.replace("\"params\"", "\"param\""), "steps[0].param"),
        ("steps", r#"{"format":"mzflow/2","steps":{}}"#.to_string(), "steps"),
        ("params", r#"{"format":"mzflow/2","steps":[{"id":"detect","step":"features.detect@2","in":{"index":"file:index.mzidx"},"params":[1]}]}"#.to_string(), "steps[0].params"),
        ("unknown_param", good.replace("\"floor\"", "\"flor\""), "detect.params.flor"),
        ("mistyped", good.replace("\"min_height\":200000", "\"min_height\":\"high\""), "detect.params.min_height"),
        ("empty", r#"{"format":"mzflow/2","steps":[]}"#.to_string(), "steps"),
        ("no_port", good.replace("\"in\":{\"index\":\"file:index.mzidx\"}", "\"in\":{}"), "detect.in.index"),
    ];
    for (tag, plan, path) in wrong {
        let d = plan_dir(tag, &plan);
        let p = d.join("plan.json");
        let p = p.to_str().unwrap();
        for args in [
            vec!["flow", "check", p, "--json"],
            vec![
                "flow",
                "run",
                p,
                "--in",
                d.to_str().unwrap(),
                "--dry-run",
                "--json",
            ],
        ] {
            let o = run(&args);
            let text = format!("{}{}", String::from_utf8_lossy(&o.stdout), o.stderr);
            assert_eq!(o.code, 3, "{tag} {args:?}: {text}");
            assert!(text.contains(path), "{tag} {args:?} names {path}: {text}");
        }
        std::fs::remove_dir_all(&d).ok();
    }
}

#[test]
fn dry_run_of_a_valid_plan_prints_what_would_run_and_writes_nothing() {
    let good = String::from_utf8(
        run(&["flow", "steps", "--id", "features.detect@2", "--template"]).stdout,
    )
    .unwrap();
    let d = plan_dir("dry", &good);
    let o = run(&[
        "flow",
        "run",
        d.join("plan.json").to_str().unwrap(),
        "--in",
        d.to_str().unwrap(),
        "--dry-run",
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let text = String::from_utf8_lossy(&o.stdout).into_owned();
    assert!(
        text.contains("features.detect@2 (detect)") && text.contains("[1/1]"),
        "{text}"
    );
    assert_eq!(
        std::fs::read_dir(&d).unwrap().count(),
        2,
        "nothing is written next to the plan"
    );
    std::fs::remove_dir_all(&d).ok();
}
