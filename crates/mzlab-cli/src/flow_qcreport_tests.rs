//! `qc.run_order@1` and `report.methods@1` in a plan: the files `flow run` writes are the texts of the engines called by hand, the parameters reach them, and every registered step has a
//! sentence template or gets the generic sentence of the methods.

use std::path::{Path, PathBuf};

use mzlab_core::campioni::{self, Campione};
use mzlab_core::json::Value;
use mzlab_feat::methods::{generic_sentence, methods_text, placeholders, template, templated};
use mzlab_feat::runorder::runorder_text;

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_cli_qcreport_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("work")).unwrap();
    d
}

fn run(d: &Path, plan: &str) -> String {
    let (run, prov) = crate::flow::run_plan(plan.as_bytes(), d, &d.join("work"), 1, None).unwrap();
    assert!(
        run.skipped.is_empty() && !prov.contains("\"error\""),
        "{prov}"
    );
    prov
}

/// 12 injections, a QC every 4, no blank, standards at one level.
fn samples() -> String {
    let cs: Vec<Campione> = (0..12)
        .map(|k| {
            let mut c = Campione::new(&format!("f{k:02}.mzML"), "", k + 1);
            c.kind = if k % 4 == 3 {
                "qc"
            } else if k == 0 {
                "std"
            } else {
                "sample"
            }
            .into();
            c.conc = (k == 0).then_some(1.0);
            c
        })
        .collect();
    campioni::to_json(&cs)
}

#[test]
fn run_order_in_a_plan_is_the_text_of_the_engine_and_its_parameters_reach_it() {
    let d = dir("order");
    let text = samples();
    std::fs::write(d.join("samples.json"), &text).unwrap();
    let plan = |params: &str| {
        format!(
            r#"{{"format":"mzflow/2","inputs":[{{"name":"samples.json","type":"samples"}}],"steps":[{{"id":"o","step":"qc.run_order@1","in":{{"samples":"file:samples.json"}},"params":{params}}}]}}"#
        )
    };
    run(&d, &plan("{}"));
    let got = std::fs::read_to_string(d.join("work/o.json")).unwrap();
    assert_eq!(got, runorder_text(&[], text.as_bytes(), None).unwrap());
    assert!(
        got.contains("warn.qc.runorder.no_blank_batch")
            && got.contains("warn.qc.runorder.std_levels"),
        "{got}"
    );
    run(
        &d,
        &plan(r#"{"blank_per_batch":false,"std_levels_min":1,"qc_every_max":3}"#),
    );
    let got = std::fs::read_to_string(d.join("work/o.json")).unwrap();
    assert!(
        !got.contains("warn.qc.runorder.no_blank_batch")
            && !got.contains("warn.qc.runorder.std_levels")
            && got.contains("warn.qc.runorder.qc_gap"),
        "{got}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn methods_in_a_plan_is_the_text_of_the_engine_in_both_styles() {
    let d = dir("methods");
    let corsa = std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/feat/methods-corsa.json"),
    )
    .unwrap();
    std::fs::write(d.join("run.json"), &corsa).unwrap();
    for style in ["paragraph", "list"] {
        let plan = format!(
            r#"{{"format":"mzflow/2","inputs":[{{"name":"run.json","type":"report"}}],"steps":[{{"id":"m","step":"report.methods@1","in":{{"run":"file:run.json"}},"params":{{"style":"{style}"}}}}]}}"#
        );
        run(&d, &plan);
        let got = std::fs::read_to_string(d.join("work/m.md")).unwrap();
        let want = vec![("style".to_string(), Value::Str(style.to_string()))];
        assert_eq!(got, methods_text(&want, &corsa).unwrap(), "{style}");
    }
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn every_registered_step_has_a_template_or_the_generic_sentence() {
    let reg = crate::flow::steps();
    let ids: Vec<&str> = reg.iter().map(|s| s.id).collect();
    for (id, alg) in templated() {
        assert!(
            ids.contains(&id),
            "the template of {id} names no registered step"
        );
        let step = reg.iter().find(|s| s.id == id).unwrap();
        let known: Vec<&str> = step
            .params
            .iter()
            .map(|p| p.name)
            .chain(
                step.algorithms
                    .iter()
                    .flat_map(|a| a.params.iter().map(|p| p.name)),
            )
            .chain(["algorithm", "preset"])
            .collect();
        for p in placeholders(template(id, alg).unwrap()) {
            assert!(
                known.contains(&p),
                "{id}: {{{p}}} is no parameter of the step"
            );
        }
    }
    let bare: Vec<&str> = templated().iter().map(|t| t.0).collect();
    for s in &reg {
        let sentence = if bare.contains(&s.id) {
            template(s.id, "")
                .or(template(s.id, "x"))
                .map(str::to_string)
        } else {
            None
        };
        let text = sentence.unwrap_or_else(|| generic_sentence(s.id, &[("a", "1")]));
        assert!(text.ends_with('.') && text.len() > 20, "{}: {text}", s.id);
    }
}
