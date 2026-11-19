//! `mzlab-cli sample design`: the bytes are the engine's `design_*` route, a bad key is named, and a design proposed from the names runs a `table.stats@1` plan.

use std::path::{Path, PathBuf};

use mzlab_core::json::parse;
use mzlab_core::samples;

use crate::flow_stats_tests::{table_for, NAMES};

const FILES: [&str; 7] = [
    "blank.mzML",
    "ref_t005min_1.mzML",
    "ref_t005min_2.mzML",
    "ref_t005min_3.mzML",
    "trt_t005min_1.mzML",
    "trt_t005min_2.mzML",
    "trt_t005min_3.mzML",
];

fn run(args: &[&str]) -> crate::Outcome {
    let a: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    crate::execute(&a)
}

fn ok(args: &[&str]) -> String {
    let o = run(args);
    assert_eq!(o.code, 0, "{args:?}: {}", o.stderr);
    String::from_utf8(o.stdout).unwrap()
}

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_cli_design_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn p(d: &Path, name: &str) -> String {
    d.join(name).to_string_lossy().into_owned()
}

/// The `samples/1` of `FILES`, written as `samples.json` by `sample table`.
fn samples_file(d: &Path) -> String {
    std::fs::write(d.join("names.txt"), FILES.join("\n")).unwrap();
    let doc = ok(&["sample", "table", "--names", &p(d, "names.txt")]);
    std::fs::write(d.join("samples.json"), doc).unwrap();
    p(d, "samples.json")
}

fn propose(d: &Path) -> (String, String) {
    let s = samples_file(d);
    let out = ok(&[
        "sample",
        "design",
        "propose",
        "--samples",
        &s,
        "--out",
        &p(d, "design.json"),
    ]);
    (s, out)
}

#[test]
fn propose_gives_the_engine_design_with_each_choice_and_its_evidence() {
    let d = dir("propose");
    let (s, out) = propose(&d);
    assert!(
        out.starts_with("{\"schema\":\"design_proposal/1\",\"design\":{\"schema\":\"design/1\""),
        "{out}"
    );
    assert!(out.ends_with("}\n"));
    // the design is the bytes of the route the site and the app call
    let route =
        samples::route_all("design_propose", &std::fs::read_to_string(&s).unwrap()).unwrap();
    assert!(out.contains(&format!("\"design\":{route},")), "{out}");
    assert_eq!(
        std::fs::read_to_string(d.join("design.json")).unwrap(),
        format!("{route}\n")
    );
    let v = parse(out.trim().as_bytes()).unwrap();
    let choice = |k: &str| {
        v.get("choices")
            .unwrap()
            .arr()
            .iter()
            .find(|c| c.get("choice").and_then(|x| x.str()) == Some(k))
            .unwrap()
    };
    assert_eq!(choice("reference").get("value").unwrap().str(), Some("ref"));
    assert!(choice("reference")
        .get("why")
        .unwrap()
        .str()
        .unwrap()
        .contains("starts with ref"));
    assert_eq!(
        choice("comparisons").get("value").unwrap().str(),
        Some("vs_reference")
    );
    let ev = v.get("evidence").unwrap();
    assert_eq!(ev.get("sample_files").unwrap().f64(), Some(6.0));
    let c = &ev.get("conditions").unwrap().arr()[0];
    assert_eq!(c.get("name").unwrap().str(), Some("ref"));
    assert_eq!(c.get("files").unwrap().arr().len(), 3);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn propose_takes_real_files_hashed_like_sample_table() {
    let d = dir("files");
    for f in FILES {
        std::fs::write(d.join(f), f.as_bytes()).unwrap();
    }
    let paths: Vec<String> = FILES.iter().map(|f| p(&d, f)).collect();
    let mut a = vec!["sample", "design", "propose"];
    a.extend(paths.iter().map(String::as_str));
    let out = ok(&a);
    assert!(out.contains("\"reference\":\"ref\""), "{out}");
    assert!(
        !out.contains("\"sha256\":\"\""),
        "real files carry their SHA-256: {out}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_design_proposed_from_the_names_runs_a_stats_plan_without_an_edit() {
    let d = dir("e2e");
    propose(&d);
    std::fs::write(d.join("table.mzfeat"), table_for(&FILES)).unwrap();
    let plan = r#"{"format":"mzflow/2","inputs":[{"name":"table.mzfeat","type":"table"},{"name":"samples.json","type":"samples"},{"name":"design.json","type":"design"}],"steps":[
    {"id":"s","step":"table.stats@1","in":{"table":"file:table.mzfeat","samples":"file:samples.json","design":"file:design.json"}}]}"#;
    std::fs::write(d.join("plan.json"), plan).unwrap();
    let o = run(&[
        "flow",
        "run",
        &p(&d, "plan.json"),
        "--in",
        &d.to_string_lossy(),
        "--out",
        &p(&d, "run"),
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let stat = std::fs::read_to_string(d.join("run/s.json")).unwrap();
    assert!(stat.starts_with("{\"schema\":\"stat/1\""), "{stat}");
    // the validate of the same pair agrees
    let v = ok(&[
        "sample",
        "design",
        "validate",
        &p(&d, "design.json"),
        "--samples",
        &p(&d, "samples.json"),
    ]);
    assert!(v.contains("\"valid\":true"), "{v}");
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(NAMES.len(), FILES.len());
}

#[test]
fn validate_keeps_the_warnings_of_the_route_byte_for_byte() {
    let d = dir("warn");
    let (s, _) = propose(&d);
    let design = std::fs::read_to_string(d.join("design.json")).unwrap();
    let out = ok(&[
        "sample",
        "design",
        "validate",
        &p(&d, "design.json"),
        "--samples",
        &s,
    ]);
    let route = samples::route_all(
        "design_validate",
        &format!(
            "{{\"design\":{design},\"samples\":{}}}",
            std::fs::read_to_string(&s).unwrap()
        ),
    )
    .unwrap();
    assert!(
        out.ends_with(&format!("\"samples_checked\":true,{}\n", &route[1..])),
        "{out}\n{route}"
    );
    let sha = samples::route_all("design_digest", &design).unwrap();
    assert!(out.contains(&format!("\"sha256\":\"{sha}\"")), "{out}");
    let _ = std::fs::remove_dir_all(&d);
}

fn problems_of(o: &crate::Outcome) -> Vec<(String, String)> {
    let v = parse(&o.stdout).unwrap();
    assert_eq!(v.get("schema").unwrap().str(), Some("error/1"));
    v.get("problems")
        .unwrap()
        .arr()
        .iter()
        .map(|x| {
            (
                x.get("port").unwrap().str().unwrap().to_string(),
                x.get("error_key").unwrap().str().unwrap().to_string(),
            )
        })
        .collect()
}

#[test]
fn an_invalid_design_is_exit_3_and_names_the_bad_keys() {
    let d = dir("bad");
    let bad = r#"{"schema":"design/1","conditions":[{"name":"a","replicates":-1,"levels":{}},{"name":"a","replicates":3,"levels":{"f":1}}],"reference":"zz","comparisons":[{"kind":"pair","a":"a"}],"factors":[],"batches":[]}"#;
    std::fs::write(d.join("bad.json"), bad).unwrap();
    let o = run(&["sample", "design", "validate", &p(&d, "bad.json"), "--json"]);
    assert_eq!(o.code, 3, "{}", o.stderr);
    let got = problems_of(&o);
    for (key, code) in [
        ("conditions[0].replicates", "badNumber"),
        ("conditions[1].levels.f", "wrongType"),
        ("comparisons[0].b", "missing"),
    ] {
        assert!(
            got.contains(&(key.to_string(), format!("err.design.problem.{code}"))),
            "{got:?}"
        );
    }
    assert!(
        o.stderr.contains("conditions[0].replicates"),
        "{}",
        o.stderr
    );
    // a design that reads but names a condition it lacks
    let named = r#"{"schema":"design/1","conditions":[{"name":"a","replicates":3,"levels":{}}],"reference":"zz","comparisons":[{"kind":"pair","a":"a","b":"q"}],"factors":[],"batches":[]}"#;
    std::fs::write(d.join("named.json"), named).unwrap();
    let got = problems_of(&run(&[
        "sample",
        "design",
        "validate",
        &p(&d, "named.json"),
    ]));
    assert_eq!(
        got,
        vec![
            (
                "reference".to_string(),
                "err.design.problem.unknownCondition".to_string()
            ),
            (
                "comparisons[0].b".to_string(),
                "err.design.problem.unknownCondition".to_string()
            )
        ]
    );
    std::fs::write(d.join("junk.json"), "not json").unwrap();
    let o = run(&["sample", "design", "validate", &p(&d, "junk.json")]);
    assert_eq!(o.code, 3);
    assert_eq!(problems_of(&o)[0].0, "$");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_comparison_of_a_condition_without_files_blocks_with_samples() {
    let d = dir("empty");
    let (s, _) = propose(&d);
    let design = std::fs::read_to_string(d.join("design.json")).unwrap();
    let wrong = design.replace(
        "\"conditions\":[",
        "\"conditions\":[{\"levels\":{},\"name\":\"ghost\",\"replicates\":0},",
    );
    let wrong = wrong.replace(
        "\"comparisons\":[{\"kind\":\"vs_reference\"}]",
        "\"comparisons\":[{\"a\":\"ref\",\"b\":\"ghost\",\"kind\":\"pair\"}]",
    );
    std::fs::write(d.join("ghost.json"), wrong).unwrap();
    // without the samples the design is well formed
    assert_eq!(
        run(&["sample", "design", "validate", &p(&d, "ghost.json")]).code,
        0
    );
    let o = run(&[
        "sample",
        "design",
        "validate",
        &p(&d, "ghost.json"),
        "--samples",
        &s,
    ]);
    assert_eq!(o.code, 3, "{}", o.stderr);
    assert_eq!(
        problems_of(&o),
        vec![(
            "comparisons".to_string(),
            "err.design.problem.emptyComparison".to_string()
        )]
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn canonical_sorts_the_keys_and_gives_the_sha256_prov_records() {
    let d = dir("canon");
    let (_, _) = propose(&d);
    let design = std::fs::read_to_string(d.join("design.json")).unwrap();
    // the same design with its members in another order
    let v = parse(design.trim().as_bytes()).unwrap();
    let batches: Vec<String> = v
        .get("batches")
        .unwrap()
        .arr()
        .iter()
        .rev()
        .map(|b| {
            format!(
                "{{\"file\":{:?},\"sha256\":{:?},\"batch\":{:?}}}",
                b.get("file").unwrap().str().unwrap(),
                b.get("sha256").unwrap().str().unwrap(),
                b.get("batch").unwrap().str().unwrap()
            )
        })
        .collect();
    let shuffled = format!(
        "{{\"reference\":\"ref\",\"batches\":[{}],\"factors\":[],\"comparisons\":[{{\"kind\":\"vs_reference\"}}],\"conditions\":[{{\"name\":\"ref\",\"replicates\":3,\"levels\":{{}}}},{{\"name\":\"trt\",\"replicates\":3,\"levels\":{{}}}}],\"schema\":\"design/1\"}}",
        batches.join(",")
    );
    std::fs::write(d.join("shuffled.json"), shuffled).unwrap();
    let out = ok(&[
        "sample",
        "design",
        "canonical",
        &p(&d, "shuffled.json"),
        "--out",
        &p(&d, "canon.json"),
    ]);
    let want = design.trim_end();
    assert_eq!(
        std::fs::read_to_string(d.join("canon.json")).unwrap(),
        format!("{want}\n")
    );
    let sha = samples::route_all("design_digest", want).unwrap();
    assert!(out.starts_with(&format!("{{\"schema\":\"design_canonical/1\",\"sha256\":\"{sha}\",\"bytes\":{},\"design\":{want}}}", want.len())), "{out}");
    assert!(out.ends_with("}\n"));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn usage_and_missing_files_have_their_own_exit_codes() {
    assert_eq!(run(&["sample", "design", "propose"]).code, 2);
    assert_eq!(
        run(&[
            "sample",
            "design",
            "propose",
            "--samples",
            "a.json",
            "b.mzML"
        ])
        .code,
        2
    );
    assert_eq!(run(&["sample", "design", "validate"]).code, 2);
    assert_eq!(run(&["sample", "design", "nope"]).code, 2);
    assert_eq!(
        run(&["sample", "design", "validate", "/no/such/design.json"]).code,
        4
    );
    let d = dir("out");
    let (s, _) = propose(&d);
    let o = run(&[
        "sample",
        "design",
        "propose",
        "--samples",
        &s,
        "--out",
        "/no/such/dir/d.json",
    ]);
    assert_eq!(o.code, 6, "{}", o.stderr);
    let h = ok(&["sample", "design", "--help"]);
    assert!(h.contains("propose") && h.contains("canonical"));
    let _ = std::fs::remove_dir_all(&d);
}
