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
