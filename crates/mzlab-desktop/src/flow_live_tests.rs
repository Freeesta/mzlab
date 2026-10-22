//! `flow/live/<op>` answers as `api/flow/sched/<op>` of the site does: the same calls on the scheduler of the site (`sched_route`) and on the run of the
//! app (no worker: nothing runs, the scheduler only answers) give the same texts, but for the keys, which hold the hash of the inputs.

use serde_json::{json, Value};

use crate::engine::State;

const PLAN: &str = r#"{"format":"mzflow/2","inputs":[{"name":"s_t000.mzML","type":"raw"},{"name":"s_t010.mzML","type":"raw"}],"steps":[
{"id":"idx","step":"index.mzidx@1","in":{"source":["file:s_t000.mzML","file:s_t010.mzML"]}},
{"id":"det","step":"features.detect@1","in":{"index":["idx.index"]}},
{"id":"ali","step":"features.align@1","in":{"peaks":["det.peaks"]}}]}"#;

fn no_keys(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(k) = rest.find("\"key\":\"") {
        let k = k + 7;
        out += &rest[..k];
        out += "_";
        rest = &rest[k + rest[k..].find('"').unwrap_or(0)..];
    }
    out + rest
}

fn answer(r: crate::rpc::Answer) -> String {
    match r {
        crate::rpc::Answer::Text(t) => t,
        _ => panic!("a text"),
    }
}

#[test]
fn the_routes_answer_as_the_scheduler_of_the_site() {
    let changed = PLAN.replace(
        r#""id":"det","step":"features.detect@1","#,
        r#""id":"det","step":"features.detect@1","params":{"min_height":50000},"#,
    );
    let files = json!({ "s_t000.mzML": "a", "s_t010.mzML": "b" });
    let st = State::new();
    st.live_start(&json!({ "plan": PLAN, "files": files }), 0)
        .unwrap();

    let steps = mzlab_core::step::json::registry_json(&mzlab_cli::flow::steps());
    let new = format!(
        r#"{{"plan":{},"registry":{steps},"version":{:?}}}"#,
        serde_json::to_string(PLAN).unwrap(),
        env!("CARGO_PKG_VERSION")
    );
    mzlab_core::flow::sched_route("new", &new).unwrap();

    let job = json!({ "instance": "det", "item": "s_t010.mzML" });
    let calls: Vec<(&str, Value)> = vec![
        ("jobs", json!({})),
        ("nodes", json!({})),
        ("events", json!({ "since": 0 })),
        ("pause", json!({})),
        ("jobs", json!({})),
        ("resume", json!({})),
        ("update", json!({ "plan": changed })),
        ("events", json!({ "since": 12 })),
        ("nodes", json!({})),
        ("retry", json!({ "job": job })),
        ("unblock", json!({ "job": job })),
        ("rev", json!({})),
        ("jobs", json!({})),
    ];
    for (op, body) in calls {
        let site = mzlab_core::flow::sched_route(op, &body.to_string()).unwrap();
        let app = answer(st.flow_live(op, &body).unwrap());
        assert_eq!(no_keys(&app), no_keys(&site), "{op}");
    }
    assert!(st.flow_live("nope", &json!({})).is_err());
    st.flow_live("drop", &json!({})).unwrap();
    assert!(st.flow_live("jobs", &json!({})).is_err());
}
