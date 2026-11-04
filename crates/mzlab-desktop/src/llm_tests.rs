//! Tests of the assistant routes of the app.

use super::*;

#[test]
fn key_file_is_private_and_roundtrips() {
    let d = tempfile::tempdir().unwrap();
    let path = d.path().join(KEYS_FILE);
    let mut k = HashMap::new();
    k.insert("anthropic".to_string(), "secret-test-value".to_string());
    write_keys(&path, &k).unwrap();
    assert_eq!(read_keys(&path), k);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}

#[test]
fn providers_route_never_returns_a_key() {
    let st = State::default();
    let d = tempfile::tempdir().unwrap();
    st.store.lock().unwrap().set_dir(d.path().to_path_buf());
    st.op_llm(
        "llm/key/set",
        &json!({"provider": "anthropic", "key": "secret-test-value"}),
    )
    .unwrap();
    let Answer::Text(t) = st.op_llm("llm/providers", &json!({})).unwrap() else {
        panic!()
    };
    assert!(!t.contains("secret-test-value"));
    assert!(t.contains("\"anthropic\":true"), "{t}");
    st.op_llm("llm/key/forget", &json!({"provider": "anthropic"}))
        .unwrap();
    let Answer::Text(t) = st.op_llm("llm/providers", &json!({})).unwrap() else {
        panic!()
    };
    assert!(t.contains("\"anthropic\":false"), "{t}");
}

#[test]
fn unknown_provider_is_a_conversation_error() {
    let st = State::default();
    let e = st
        .op_llm("llm/key/set", &json!({"provider": "nobody", "key": "x"}))
        .err()
        .unwrap();
    assert_eq!(e.key, "err.llm.conversation");
}

#[test]
fn send_of_a_ready_request_needs_its_provider_and_a_well_formed_request() {
    let st = State::default();
    let d = tempfile::tempdir().unwrap();
    st.store.lock().unwrap().set_dir(d.path().to_path_buf());
    let bad = |p: Value| st.op_llm("llm/send", &p).err().map(|e| e.key.to_string());
    assert_eq!(
        bad(json!({"provider": "nope", "request": "{}"})).as_deref(),
        Some("err.llm.conversation")
    );
    assert!(bad(json!({"provider": "anthropic", "request": "{\"url\":1}"})).is_some());
    assert!(bad(json!({"provider": "anthropic", "request": "not json"})).is_some());
}

fn root(rel: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

/// The body the site sends for a conversation: what `Llm.request` of the `.wasm` does with the catalog file shipped beside the page.
fn site_body(conv: &str, catalog_file: &str) -> String {
    let text = std::fs::read_to_string(root(catalog_file)).unwrap();
    let tools = mzlab_llm::tools::parse_catalog(&text).unwrap();
    let c = Conversation::parse(conv).unwrap();
    provider::request(&find(&c.provider).unwrap(), &c, &tools)
        .unwrap()
        .body
}

#[test]
fn the_demo_body_of_the_app_is_the_body_of_the_site() {
    let conv = std::fs::read_to_string(root("tests/data/llm/demo/conv.json")).unwrap();
    let (_, _, app) = build(&conv).unwrap();
    let site = site_body(&conv, "docs/schemi/strumenti-demo-1.json");
    let sha = mzlab_core::budget::sha256_hex(app.body.as_bytes());
    assert_eq!(sha, mzlab_core::budget::sha256_hex(site.as_bytes()));
    // the same digest the .wasm test of the site checks (crates/mzlab-llm-wasm/src/tests.rs)
    let file = root("tests/data/llm/demo/body.sha256");
    if std::env::var_os("MZLAB_UPDATE_LLM").is_some() {
        std::fs::write(&file, format!("{sha}\n")).unwrap();
    }
    assert_eq!(
        std::fs::read_to_string(&file).unwrap().trim(),
        sha,
        "MZLAB_UPDATE_LLM=1 rewrites it"
    );
    // the demo catalog only: nothing that runs or removes
    assert!(!app.body.contains("\"flow_run\"") && app.body.contains("\"flow_propose_plan\""));
    assert!(app.url.starts_with(demo::PLACEHOLDER) && app.auth.header.is_empty());
}

#[test]
fn the_demo_request_leaves_with_the_same_headers_as_in_the_site() {
    let conv = std::fs::read_to_string(root("tests/data/llm/demo/conv.json")).unwrap();
    let (p, _, app) = build(&conv).unwrap();
    let r = demo::for_send(
        &p,
        app,
        "https://assist.example.org",
        "panel_0123456789",
        "",
    )
    .unwrap();
    assert_eq!(r.url, "https://assist.example.org/v1/messages");
    let names: Vec<&str> = r.headers.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(names, ["content-type", "accept", "x-mzlab-session"]);
}

#[test]
fn the_demo_send_needs_the_proxy_host_and_a_session_but_no_key() {
    let st = State::default();
    let d = tempfile::tempdir().unwrap();
    st.store.lock().unwrap().set_dir(d.path().to_path_buf());
    let conv = std::fs::read_to_string(root("tests/data/llm/demo/conv.json")).unwrap();
    let key = |p: Value| st.op_llm("llm/send", &p).err().map(|e| e.key.to_string());
    // no host yet (proxy-host.js empty): nothing leaves
    assert_eq!(
        key(json!({"conv": conv, "session": "panel_0123456789"})).as_deref(),
        Some("err.llm.network")
    );
    assert_eq!(
        key(json!({"conv": conv, "proxy_host": "https://assist.example.org", "session": "x"}))
            .as_deref(),
        Some("err.llm.conversation")
    );
    let Answer::Text(t) = st.op_llm("llm/providers", &json!({})).unwrap() else {
        panic!()
    };
    assert!(t.contains("\"mzlab-demo\":false"), "{t}");
}
