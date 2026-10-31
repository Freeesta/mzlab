//! The `agent` of `prov/1`: the build keys come after `platform` and only when present; a record without them keeps its old bytes.

use super::*;

fn head(agent: &Agent) -> String {
    let t = prov_json(agent, None, &Run::default());
    t[..t.find(",\"entity\"").unwrap()].to_string()
}

#[test]
fn an_agent_without_build_keys_gives_the_old_bytes() {
    let a = Agent {
        name: "mzLab".into(),
        version: "1.0.0".into(),
        commit: Some("abc1234".into()),
        platform: "native",
        ..Default::default()
    };
    assert_eq!(
        head(&a),
        r#"{"schema":"prov/1","agent":{"name":"mzLab","version":"1.0.0","commit":"abc1234","platform":"native"}}"#
            .replace("}}", "}")
    );
}

#[test]
fn every_build_key_is_written_in_order_after_platform() {
    let a = Agent {
        name: "mzLab".into(),
        version: "1.0.0".into(),
        commit: Some("0123456789012345678901234567890123456789".into()),
        platform: "wasm",
        target: Some("wasm32-unknown-unknown".into()),
        rustc: Some("rustc 1.90.0".into()),
        cargo_lock_sha256: Some("ab".into()),
        modules: vec![
            ("b.wasm".into(), "22".into()),
            ("a.wasm".into(), "11".into()),
        ],
    };
    assert_eq!(
        head(&a),
        concat!(
            r#"{"schema":"prov/1","agent":{"name":"mzLab","version":"1.0.0","commit":"0123456789012345678901234567890123456789","platform":"wasm","#,
            r#""target":"wasm32-unknown-unknown","rustc":"rustc 1.90.0","cargo_lock_sha256":"ab","modules":{"a.wasm":"11","b.wasm":"22"}}"#
        )
    );
}

#[test]
fn build_agent_names_this_build() {
    let a = build_agent("mzLab");
    assert_eq!(a.version, env!("CARGO_PKG_VERSION"));
    assert_eq!(a.platform, Platform::current().name());
    assert!(a.target.is_some_and(|t| !t.is_empty()));
    assert!(a.modules.is_empty());
}
