use super::*;

#[test]
fn reads_the_instances_in_order() {
    let p = parse(
        br#"{"format":"mzflow/2","program":{"name":"mzLab","version":"1"},
          "inputs":[{"name":"a.raw","sha256":"ab","bytes":3,"type":"raw","facets":{"resolution":["hr"],"ms_levels":[1,2]}}],
          "steps":[{"id":"conv","step":"convert.mzml@1","in":{"raw":"file:a.raw"},"params":{"numpress":true}},
                   {"id":"idx","step":"index.mzidx@2","in":{"source":["conv.mzml"]},"algorithm":"x","preset":"y"}]}"#,
    )
    .unwrap();
    assert_eq!(p.steps.len(), 2);
    assert_eq!(p.steps[0].ins[0].1, vec![Ref::File("a.raw".into())]);
    assert_eq!(p.steps[1].version, 2);
    assert_eq!(p.steps[1].step_ref(), "index.mzidx@2");
    assert_eq!(p.steps[1].sources(), ["conv"]);
    assert_eq!(p.inputs[0].observed.ms_levels, [1, 2]);
    assert_eq!(p.inputs[0].observed.resolution, ["hr"]);
}

#[test]
fn reads_mzflow_1() {
    let p = parse(
        br#"{"format":"mzflow/1","program":{"name":"mzLab","version":"0.9","commit":"abc"},"operations":[{"op":"samples"}]}"#,
    )
    .unwrap();
    assert_eq!(p.format, "mzflow/1");
    assert_eq!(
        (p.program.name.as_str(), p.program.version.as_str()),
        ("mzLab", "0.9")
    );
    assert_eq!(p.program.commit.as_deref(), Some("abc"));
    assert!(p.steps.is_empty());
    assert_eq!(p.operations.unwrap().arr().len(), 1);
}

#[test]
fn refuses_what_is_not_a_workflow() {
    let key = |s: &str| parse(s.as_bytes()).unwrap_err().key;
    assert_eq!(key("{"), "err.flow.json");
    assert_eq!(key("[]"), "err.flow.json");
    assert_eq!(key(r#"{"format":"mzflow/9"}"#), "err.flow.format");
    assert_eq!(
        key(r#"{"steps":[{"id":"a","step":"x@one"}]}"#),
        "err.flow.step"
    );
    assert_eq!(
        key(r#"{"steps":[{"id":"a","step":"x@1","in":{"p":"nothing"}}]}"#),
        "err.flow.ref"
    );
    assert_eq!(key(r#"{"steps":[{"step":"x@1"}]}"#), "err.flow.instance");
}

#[test]
fn step_ids_that_are_not_plain_file_names_are_refused() {
    let plan = |id: &str| {
        let id = id.replace('\\', "\\\\");
        format!(
            r#"{{"format":"mzflow/2","inputs":[],"steps":[{{"id":"{id}","step":"graph.make@1"}}]}}"#
        )
    };
    let long = "a".repeat(129);
    for id in [
        "",
        "/tmp/x",
        "a/b",
        "a\\b",
        "..",
        "../x",
        ".hidden",
        "C:x",
        "a b",
        "é",
        long.as_str(),
    ] {
        let e = parse(plan(id).as_bytes()).unwrap_err();
        assert_eq!(e.key, "err.flow.instance.id", "{id:?}");
    }
    for id in ["graph", "step_1", "a.b-c", "A9", "a".repeat(128).as_str()] {
        assert!(parse(plan(id).as_bytes()).is_ok(), "{id:?}");
    }
}

/// The error key and its `path` for a plan text the reader refuses.
fn refused(text: &str) -> (&'static str, String) {
    let e = parse(text.as_bytes()).unwrap_err();
    let path = e
        .params
        .iter()
        .find(|(k, _)| *k == "path")
        .map(|(_, v)| v.to_string());
    (e.key, path.unwrap_or_default())
}

#[test]
fn a_key_that_is_not_in_the_format_is_named_with_its_path() {
    assert_eq!(
        refused(r#"{"hello":"world"}"#),
        ("err.flow.key.unknown", "hello".into())
    );
    assert_eq!(
        refused(r#"{"schema":"mzflow/2","steps":[]}"#),
        ("err.flow.key.unknown", "schema".into())
    );
    assert_eq!(
        refused(r#"{"steps":[{"id":"a","step":"x@1"},{"id":"b","step":"x@1","param":{}}]}"#),
        ("err.flow.key.unknown", "steps[1].param".into())
    );
    assert_eq!(
        refused(r#"{"steps":[],"inputs":[{"name":"a","path":"b"}]}"#),
        ("err.flow.key.unknown", "inputs[0].path".into())
    );
}

#[test]
fn a_value_of_the_wrong_type_is_named_with_its_path() {
    for (text, path) in [
        (r#"{"steps":{}}"#, "steps"),
        (r#"{"format":"mzflow/2"}"#, "steps"),
        (r#"{"steps":[],"inputs":"a.mzML"}"#, "inputs"),
        (r#"{"steps":["x"]}"#, "steps[0]"),
        (
            r#"{"steps":[{"id":"a","step":"x@1","params":[1]}]}"#,
            "steps[0].params",
        ),
        (
            r#"{"steps":[{"id":"a","step":"x@1","in":[]}]}"#,
            "steps[0].in",
        ),
        (
            r#"{"steps":[{"id":"a","step":"x@1","in":{"p":3}}]}"#,
            "steps[0].in.p",
        ),
        (
            r#"{"steps":[{"id":"a","step":"x@1","in":{"p":["f", 3]}}]}"#,
            "steps[0].in.p[1]",
        ),
        (r#"{"steps":[{"id":7,"step":"x@1"}]}"#, "steps[0].id"),
        (
            r#"{"steps":[{"id":"a","step":"x@1","label":null}]}"#,
            "steps[0].label",
        ),
    ] {
        assert_eq!(refused(text), ("err.flow.shape", path.into()), "{text}");
    }
}

#[test]
fn the_plans_the_page_writes_keep_reading() {
    assert!(parse(br#"{"steps":[]}"#).is_ok());
    assert!(parse(br#"{"format":"mzflow/1","operations":[]}"#).is_ok());
    assert!(parse(br#"{"format":"mzflow/2","program":{"name":"a","version":"1"},"inputs":[],"steps":[],"operations":[{"op":"x"}]}"#).is_ok());
}
