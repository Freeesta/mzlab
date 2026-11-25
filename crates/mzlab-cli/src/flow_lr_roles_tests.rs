//! The roles of the files in `tp.dig_lr`: a samples document that lists files the table does not carry (a standard, a dark control) no longer stops the Dig,
//! a table file with no sample row takes the role its name proposes, both are logged, and a wrong samples document gets a hint naming the commands.

use std::path::{Path, PathBuf};

use crate::execute;

/// The five demo files, copied under the names of a series that has a standard and a dark control (`std`, `darkcontrol`: the last one a compound token).
const FILES: [(&str, &str); 5] = [
    ("FullScan_t0.mzML", "Smp_t0.mzML"),
    ("FullScan_t5.mzML", "Smp_t10.mzML"),
    ("FullScan_t10.mzML", "Smp_t30.mzML"),
    ("FullScan_t15.mzML", "Smp_darkcontrol.mzML"),
    ("FullScan_t30.mzML", "Smp_std_5ppm.mzML"),
];
const FORMULA: &str = "C14H13F4N3O2S";

fn fixture(tag: &str, aligned: &[&str], samples_of: &[&str]) -> PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_cli_lrroles_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mzlab/web/esempi");
    for (from, to) in FILES {
        std::fs::copy(src.join(from), d.join(to)).unwrap();
    }
    let rows: Vec<(String, String, String)> = samples_of
        .iter()
        .map(|n| (n.to_string(), String::new(), String::new()))
        .collect();
    let samples = mzlab_core::samples::to_json(&mzlab_core::samples::propose(&rows));
    std::fs::write(d.join("samples.json"), samples).unwrap();
    std::fs::write(
        d.join("parent.json"),
        format!(r#"{{"formula":"{FORMULA}"}}"#),
    )
    .unwrap();
    std::fs::write(d.join("plan.json"), plan(aligned)).unwrap();
    d
}

fn plan(aligned: &[&str]) -> String {
    let raw: Vec<String> = aligned
        .iter()
        .map(|n| format!(r#"{{"name":"{n}","type":"raw"}}"#))
        .collect();
    let src: Vec<String> = aligned.iter().map(|n| format!(r#""file:{n}""#)).collect();
    format!(
        r#"{{"format":"mzflow/2","inputs":[{},{{"name":"samples.json","type":"samples"}},{{"name":"parent.json","type":"structure"}}],"steps":[
        {{"id":"idx","step":"index.mzidx@1","in":{{"source":[{}]}}}},
        {{"id":"det","step":"features.detect@1","in":{{"index":["idx.index"]}}}},
        {{"id":"ali","step":"features.align@1","in":{{"peaks":["det.peaks"]}}}},
        {{"id":"gap","step":"features.gapfill@1","in":{{"table":"ali.table","index":["idx.index"]}}}},
        {{"id":"dig","step":"tp.dig_lr@1","in":{{"table":"gap.table","samples":"file:samples.json","parent":"file:parent.json","index":["idx.index"]}}}}]}}"#,
        raw.join(","),
        src.join(",")
    )
}

fn run(d: &Path, out: &str) -> (u8, String) {
    let o = execute(
        &[
            "flow",
            "run",
            d.join("plan.json").to_str().unwrap(),
            "--in",
            d.to_str().unwrap(),
            "--out",
            d.join(out).to_str().unwrap(),
        ]
        .map(String::from),
    );
    (o.code, o.stderr)
}

fn roles_of(d: &Path, out: &str) -> Vec<(String, String)> {
    let v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(d.join(out).join("dig.json")).unwrap())
            .unwrap();
    v["rows"][0]["points"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            (
                p["file"].as_str().unwrap().to_string(),
                p["role"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

const ALL: [&str; 5] = [
    "Smp_t0.mzML",
    "Smp_t10.mzML",
    "Smp_t30.mzML",
    "Smp_darkcontrol.mzML",
    "Smp_std_5ppm.mzML",
];

/// Problem (b): the samples list the standard and the dark control, the plan aligns the three timed files only. It used to stop on `error.badSamples`.
#[test]
fn samples_the_table_does_not_carry_are_left_out_and_the_dig_runs() {
    let d = fixture("leftout", &ALL[..3], &ALL);
    let (code, err) = run(&d, "a");
    assert_eq!(code, 0, "{err}");
    // the Dig of the same plan with samples that list the three files alone: the same bytes
    std::fs::write(
        d.join("samples.json"),
        mzlab_core::samples::to_json(&mzlab_core::samples::propose(
            &ALL[..3]
                .iter()
                .map(|n| (n.to_string(), String::new(), String::new()))
                .collect::<Vec<_>>(),
        )),
    )
    .unwrap();
    let (code, err) = run(&d, "b");
    assert_eq!(code, 0, "{err}");
    let (a, b) = (
        std::fs::read_to_string(d.join("a/dig.json")).unwrap(),
        std::fs::read_to_string(d.join("b/dig.json")).unwrap(),
    );
    assert!(
        a == b,
        "the dig differs: {} bytes against {}",
        a.len(),
        b.len()
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// Problem (a): the table has the files but the samples lack the dark control and the standard: the roles come from the names, `darkcontrol` is a dark control.
#[test]
fn a_file_with_no_sample_takes_the_role_of_its_name() {
    let d = fixture("proposed", &ALL, &ALL[..3]);
    let (code, err) = run(&d, "a");
    assert_eq!(code, 0, "{err}");
    let roles = roles_of(&d, "a");
    let role = |f: &str| roles.iter().find(|(n, _)| n == f).map(|(_, r)| r.as_str());
    assert_eq!(role("Smp_darkcontrol.mzML"), Some("dark"), "{roles:?}");
    assert_eq!(role("Smp_std_5ppm.mzML"), Some("std"), "{roles:?}");
    assert_eq!(role("Smp_t0.mzML"), Some("t0"), "{roles:?}");
    let _ = std::fs::remove_dir_all(&d);
}

/// The log of each choice: the file, the role and the part of the name that decided it, with the way to change it.
#[test]
fn every_pairing_choice_is_a_line_with_its_evidence() {
    use mzlab_core::samples::order_for_roles;
    let doc = mzlab_core::samples::to_json(&mzlab_core::samples::propose(
        &[("Smp_t0.mzML", "", ""), ("Smp_gone_std.mzML", "", "")]
            .map(|(a, b, c)| (a.to_string(), b.to_string(), c.to_string())),
    ));
    let names: Vec<String> = ["Smp_t0.mzML", "Smp_darkcontrol.mzML", "Smp_x.mzML"]
        .map(String::from)
        .to_vec();
    let paired = order_for_roles(&doc, &names).unwrap();
    let lines = crate::flow_digl::pairnotes::lines(&paired.notes);
    assert_eq!(lines.len(), 3, "{lines:?}");
    assert!(
        lines.iter().any(|l| l.contains("Smp_gone_std.mzML")
            && l.contains("role std")
            && l.contains("left out")),
        "{lines:?}"
    );
    assert!(
        lines.iter().any(|l| l.contains("Smp_darkcontrol.mzML")
            && l.contains("role dark")
            && l.contains("“darkcontrol”")),
        "{lines:?}"
    );
    assert!(
        lines.iter().any(|l| l.contains("Smp_x.mzML")
            && l.contains("no role word")
            && l.contains("sample import")),
        "{lines:?}"
    );
}

/// A `sample-import/1` document as the samples input is not a `samples/1`: the failure names the two commands that write one (AGENT-DISCOVERY).
#[test]
fn a_wrong_samples_document_gets_a_hint_naming_the_commands() {
    let d = fixture("hint", &ALL, &ALL);
    std::fs::write(
        d.join("samples.json"),
        r#"{"schema":"sample-import/1","report":{},"samples":{"schema":"samples/1","samples":[]}}"#,
    )
    .unwrap();
    let (code, err) = run(&d, "a");
    assert_eq!(code, 1, "{err}");
    let hint = err.lines().find(|l| l.starts_with("hint: ")).unwrap_or("");
    assert!(
        hint.contains("mzlab-cli sample table") && hint.contains("mzlab-cli sample import"),
        "{err}"
    );
    let _ = std::fs::remove_dir_all(&d);
}
