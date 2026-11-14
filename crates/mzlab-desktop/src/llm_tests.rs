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

#[test]
fn a_ready_request_for_another_origin_never_leaves_with_the_key() {
    let st = State::default();
    let d = tempfile::tempdir().unwrap();
    st.store.lock().unwrap().set_dir(d.path().to_path_buf());
    st.op_llm(
        "llm/key/set",
        &json!({"provider": "anthropic", "key": "sk-test"}),
    )
    .unwrap();
    let send = |url: &str| {
        let r = json!({"url": url, "headers": [], "body": "{}", "auth": {"header": "x-api-key", "prefix": ""}});
        st.op_llm(
            "llm/send",
            &json!({"provider": "anthropic", "request": r.to_string()}),
        )
        .err()
        .map(|e| {
            (
                e.key.to_string(),
                e.params
                    .iter()
                    .find(|(k, _)| *k == "reason")
                    .map(|(_, v)| v.clone()),
            )
        })
    };
    for u in [
        "https://evil.org/v1/messages",
        "https://api.anthropic.com@evil.org/",
        "https://api.anthropic.com.evil.org/",
    ] {
        assert_eq!(
            send(u),
            Some(("err.llm.network".into(), Some("url".into()))),
            "{u}"
        );
    }
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
        st.llm_send(
            &json!({"conv": conv, "session": "x"}),
            "https://assist.example.org"
        )
        .err()
        .map(|e| e.key.to_string())
        .as_deref(),
        Some("err.llm.conversation")
    );
    let Answer::Text(t) = st.op_llm("llm/providers", &json!({})).unwrap() else {
        panic!()
    };
    assert!(t.contains("\"mzlab-demo\":false"), "{t}");
}

#[test]
fn the_proxy_host_is_the_binary_constant_and_the_page_value_is_ignored() {
    let st = State::default();
    let d = tempfile::tempdir().unwrap();
    st.store.lock().unwrap().set_dir(d.path().to_path_buf());
    let r = |url: &str| {
        json!({"url": url, "headers": [], "body": "{}", "auth": {"header": "", "prefix": ""}})
            .to_string()
    };
    let reason = |p: Value, host: &str| {
        st.llm_send(&p, host).err().map(|e| {
            (
                e.key.to_string(),
                e.params
                    .iter()
                    .find(|(k, _)| *k == "reason")
                    .map(|(_, v)| v.clone()),
            )
        })
    };
    let url = Some(("err.llm.network".to_string(), Some("url".to_string())));
    // a page that names its own host (and the url to match) gets nothing, with the real host known or not
    let evil = json!({"provider": "mzlab-demo", "request": r("https://evil.org/v1/messages"), "proxy_host": "https://evil.org", "session": "panel_0123456789"});
    assert_eq!(reason(evil.clone(), "https://assist.example.org"), url);
    assert_eq!(reason(evil, ""), url);
    // the constant of the tree is the one of proxy-host.js (empty until the proxy exists)
    assert!(
        crate::llm::proxy_host().is_empty() || crate::llm::proxy_host().starts_with("https://")
    );
    // `op_llm` never takes the host from the page
    let page = json!({"provider": "mzlab-demo", "request": r("https://evil.org/v1/messages"), "proxy_host": "https://evil.org", "session": "panel_0123456789"});
    assert_eq!(
        st.op_llm("llm/send", &page)
            .err()
            .map(|e| e.key.to_string())
            .as_deref(),
        Some("err.llm.network")
    );
}

#[test]
fn a_redirect_of_the_provider_is_not_followed_with_the_key() {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    let other = TcpListener::bind("127.0.0.1:0").unwrap();
    other.set_nonblocking(true).unwrap();
    let first = TcpListener::bind("127.0.0.1:0").unwrap();
    let (pa, pb) = (
        first.local_addr().unwrap().port(),
        other.local_addr().unwrap().port(),
    );
    let t = std::thread::spawn(move || {
        let (mut c, _) = first.accept().unwrap();
        let mut buf = [0u8; 2048];
        let _ = c.read(&mut buf);
        let _ = write!(c, "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:{pb}/x\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
    });
    let p = find("anthropic").unwrap();
    let req = Request {
        url: format!("http://127.0.0.1:{pa}/v1/messages"),
        headers: vec![],
        body: "{}".into(),
        auth: provider::AuthSlot {
            header: "x-api-key".into(),
            prefix: String::new(),
        },
    };
    let id = with_jobs(|n, m| {
        *n += 1;
        m.insert(*n, Job::default());
        *n
    });
    run(
        id,
        p,
        false,
        req,
        Some("sk-secret".into()),
        Arc::new(AtomicBool::new(false)),
    );
    t.join().unwrap();
    std::thread::sleep(Duration::from_millis(200));
    assert!(other.accept().is_err(), "the redirect target was contacted");
    let ev = with_jobs(|_, m| m.remove(&id).unwrap().events.join("\n"));
    assert!(
        ev.contains("err.llm.network") && ev.contains("redirect"),
        "{ev}"
    );
}
