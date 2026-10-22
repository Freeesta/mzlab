//! The JSON route of the live scheduler: the registry read back from `step/1` JSON drives it like the compiled one, and a short session answers as the Flow page expects.

use super::route;
use crate::flow::sched::tests::{plan_text, registry};
use crate::json;
use crate::step::json::registry_json;
use crate::step::registry_from_value;

#[test]
fn registry_read_back_writes_the_same_json() {
    let text = registry_json(&registry());
    let back = registry_from_value(&json::parse(text.as_bytes()).unwrap()).unwrap();
    assert_eq!(registry_json(&back), text);
}

#[test]
fn a_bad_registry_is_an_error_key() {
    let e =
        registry_from_value(&json::parse(br#"[{"id":"x","in":{"a":{"type":"nope"}}}]"#).unwrap())
            .unwrap_err();
    assert_eq!(e.key, "err.flow.registry");
}

#[test]
fn a_session_runs_through_the_route() {
    let plan = plan_text(&["a.raw"], 1, 1);
    let body = format!(
        "{{\"plan\":{plan},\"registry\":{},\"version\":\"v\"}}",
        registry_json(&registry())
    );
    route("new", &body).unwrap();
    let next = route("next", "{}").unwrap();
    assert!(next.contains("\"instance\":\"conv\""), "{next}");
    let job = "{\"job\":{\"instance\":\"conv\",\"item\":\"a.raw\"}";
    assert_eq!(route("started", &format!("{job}}}")).unwrap(), "true");
    let done = format!("{job},\"result\":{{\"ms\":5}}}}");
    assert_eq!(route("finished", &done).unwrap(), "\"accepted\"");
    assert!(route("jobs", "{}").unwrap().contains("\"state\":\"done\""));
    assert_eq!(
        route("nope", "{}").unwrap_err().key,
        "err.route.unavailable"
    );
    route("drop", "{}").unwrap();
    assert_eq!(route("jobs", "{}").unwrap_err().key, "err.flow.sched");
}
