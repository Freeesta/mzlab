//! The key of a step that measures what the plan leaves out: what the plan writes is part of it, not only the resolved parameters.

use crate::flow::parse;
use crate::flow::sched::tests::{plan_text, registry};
use crate::flow::sched::{JobId, Sched};
use crate::json::Value;
use crate::step::{Param, Step};

/// A detection that measures what the plan leaves out (`floor`), with a default preset `hr` (`q` = 1).
fn measuring() -> Vec<Step> {
    let mut reg = registry();
    let det = reg.iter_mut().find(|s| s.id == "features.detect").unwrap();
    det.params.push(Param::number("floor", "").estimate());
    det.presets = vec![crate::step::Preset {
        id: "hr",
        title: "",
        when: Default::default(),
        params: vec![("q", Value::Num(1.0))],
        evidence: "",
        algorithm: None,
    }];
    det.default_preset = Some("hr");
    reg
}

#[test]
fn a_preset_named_or_a_value_written_is_another_run_of_a_step_that_measures() {
    // the default preset gives the same `params` whether it is named or not, but the run starts from the values measured on the data and takes only what the plan writes
    let key = |text: String| {
        let s = Sched::new(
            &parse(text.as_bytes()).unwrap(),
            &measuring(),
            "test-1",
            &[],
        );
        s.key(&JobId::new("det", Some("f1.raw"))).unwrap().to_string()
    };
    let bare = plan_text(&["f1.raw"], 1, 1).replace(
        "\"params\":{\"q\":1},\"in\":{\"index\"",
        "\"in\":{\"index\"",
    );
    let named = bare.replace(
        "\"step\":\"features.detect@2\",",
        "\"step\":\"features.detect@2\",\"preset\":\"hr\",",
    );
    let written = plan_text(&["f1.raw"], 1, 1);
    assert_ne!(bare, named);
    assert_ne!(key(bare.clone()), key(named.clone()));
    assert_ne!(key(bare.clone()), key(written.clone()));
    // the preset and the value it writes are the same run
    assert_eq!(key(named), key(written));
    assert_eq!(key(bare.clone()), key(bare));
}
